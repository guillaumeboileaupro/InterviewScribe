use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;

use crate::error::AppError;

const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const LOG_FILE_NAME: &str = "interviewscribe.log";
const BACKUP_FILE_NAME: &str = "interviewscribe.log.1";

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Resolves the local, on-disk directory diagnostics are written to and
/// installs a panic hook that logs panics instead of letting them vanish
/// silently. This matters most for the capture/processing background
/// threads (`capture::session::run_capture_thread`,
/// `run_recording_processing`), which run outside the `Result`/`AppError`
/// path entirely - a panic there would otherwise just end the thread with
/// no trace at all. See docs/ARCHITECTURE.md "Diagnostics locaux": this
/// file only ever contains technical events and error messages, never
/// audio, transcript text or speaker names.
pub fn init(app: &tauri::AppHandle) -> Result<(), AppError> {
    let dir = crate::app_data_subdir(app, "logs")?;
    let _ = LOG_DIR.set(dir);

    std::panic::set_hook(Box::new(|info| {
        log("PANIC", &info.to_string());
    }));

    Ok(())
}

/// Appends one line to the local diagnostics log. A no-op if `init` was
/// never called (unit tests) or if the write itself fails - a broken log
/// must never break the feature it is trying to diagnose.
pub fn log(level: &str, message: &str) {
    if let Some(dir) = LOG_DIR.get() {
        write_line(dir, level, message, SystemTime::now());
    }
}

/// Reads back the last `max_lines` lines (oldest rotated file first, so
/// output stays chronological) for display in the Reglages "Diagnostics"
/// panel. Never fails outright - an unreadable or missing log just yields
/// less history, not an error the user has to deal with.
pub fn read_tail(max_lines: usize) -> String {
    match LOG_DIR.get() {
        Some(dir) => read_tail_from(dir, max_lines),
        None => String::new(),
    }
}

fn write_line(dir: &Path, level: &str, message: &str, now: SystemTime) {
    let path = dir.join(LOG_FILE_NAME);
    rotate_if_needed(&path, dir);

    let line = format!("[{}] [{level}] {message}\n", format_timestamp(now));
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
}

fn rotate_if_needed(path: &Path, dir: &Path) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    if metadata.len() >= MAX_LOG_BYTES {
        let _ = std::fs::rename(path, dir.join(BACKUP_FILE_NAME));
    }
}

fn read_tail_from(dir: &Path, max_lines: usize) -> String {
    let mut combined = std::fs::read_to_string(dir.join(BACKUP_FILE_NAME)).unwrap_or_default();
    combined.push_str(&std::fs::read_to_string(dir.join(LOG_FILE_NAME)).unwrap_or_default());

    let lines: Vec<&str> = combined.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

fn format_timestamp(now: SystemTime) -> String {
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (secs / 86_400) as i64;
    let time_of_day = secs % 86_400;
    let (hour, minute, second) = (
        time_of_day / 3600,
        (time_of_day % 3600) / 60,
        time_of_day % 60,
    );
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Days-since-1970-01-01 to (year, month, day), UTC. Standard public-domain
/// algorithm (Howard Hinnant, "chrono-Compatible Low-Level Date
/// Algorithms") - not worth pulling a whole date/time crate just to
/// timestamp diagnostic log lines.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_line_then_read_tail_roundtrips_the_message() {
        let dir = std::env::temp_dir().join(format!("interviewscribe-diag-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        write_line(&dir, "INFO", "demarrage test", SystemTime::now());
        write_line(&dir, "ERROR", "quelque chose a echoue", SystemTime::now());

        let tail = read_tail_from(&dir, 10);
        assert!(tail.contains("[INFO] demarrage test"));
        assert!(tail.contains("[ERROR] quelque chose a echoue"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_tail_on_missing_directory_returns_empty_string_not_a_panic() {
        let dir = std::env::temp_dir().join("interviewscribe-diag-does-not-exist");
        assert_eq!(read_tail_from(&dir, 10), "");
    }

    #[test]
    fn read_tail_limits_to_the_requested_number_of_lines() {
        let dir =
            std::env::temp_dir().join(format!("interviewscribe-diag-limit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        for i in 0..5 {
            write_line(&dir, "INFO", &format!("ligne {i}"), SystemTime::now());
        }

        let tail = read_tail_from(&dir, 2);
        assert_eq!(tail.lines().count(), 2);
        assert!(tail.contains("ligne 3"));
        assert!(tail.contains("ligne 4"));
        assert!(!tail.contains("ligne 2"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rotation_moves_the_oversized_file_to_a_backup_before_the_next_write() {
        let dir = std::env::temp_dir().join(format!(
            "interviewscribe-diag-rotate-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(LOG_FILE_NAME);
        std::fs::write(&path, vec![b'x'; MAX_LOG_BYTES as usize]).unwrap();

        write_line(&dir, "INFO", "apres rotation", SystemTime::now());

        assert!(dir.join(BACKUP_FILE_NAME).exists());
        let current = std::fs::read_to_string(&path).unwrap();
        assert!(current.contains("apres rotation"));
        assert!(current.len() < MAX_LOG_BYTES as usize);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn civil_from_days_matches_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_947), (2024, 8, 12));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
        assert_eq!(civil_from_days(10_957), (2000, 1, 1));
    }
}
