use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Copies a source audio file into the app's private storage, named after the
/// interview it belongs to. Never touches the original file.
pub fn copy_into_storage(
    source: &Path,
    dest_dir: &Path,
    interview_id: i64,
) -> Result<PathBuf, AppError> {
    std::fs::create_dir_all(dest_dir)?;
    let ext = source
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("audio");
    let dest = dest_dir.join(format!("{interview_id}.{ext}"));
    std::fs::copy(source, &dest)?;
    Ok(dest)
}

/// Android equivalent: the system file picker (SAF) hands back a `content://`
/// URI, not a real filesystem path - `std::fs::copy` cannot read that at all.
/// Goes through `tauri_plugin_fs`, which already knows how to resolve one
/// (same approach as `transcription::model::install_bundled` uses for the
/// bundled Whisper model, generalized there to any `content://`/`asset://`
/// source, not just APK assets).
#[cfg(target_os = "android")]
pub fn copy_content_uri_into_storage(
    app: &tauri::AppHandle,
    source_uri: &str,
    dest_dir: &Path,
    interview_id: i64,
) -> Result<PathBuf, AppError> {
    use std::str::FromStr;

    use tauri_plugin_fs::{FilePath, FsExt};

    std::fs::create_dir_all(dest_dir)?;
    let ext = Path::new(source_uri)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("audio");
    let dest = dest_dir.join(format!("{interview_id}.{ext}"));

    let file_path = FilePath::from_str(source_uri).expect("FilePath::from_str is infallible");
    copy_from_reader(&dest, || {
        app.fs()
            .open(file_path, tauri_plugin_fs::OpenOptions::default())
    })?;
    Ok(dest)
}

// Compiled on desktop in tests to exercise the Android copy algorithm without
// a real device - same pattern as `transcription::model::install_bundled`.
#[cfg(any(target_os = "android", test))]
fn copy_from_reader(
    dest: &Path,
    open_source: impl FnOnce() -> std::io::Result<std::fs::File>,
) -> Result<(), AppError> {
    use std::io::Write;

    let mut source = open_source()?;
    let mut file = std::fs::File::create(dest)?;
    std::io::copy(&mut source, &mut file)?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_file_preserving_extension() {
        let tmp = std::env::temp_dir().join(format!("interviewscribe-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let source = tmp.join("source.wav");
        std::fs::write(&source, b"not really audio").unwrap();

        let dest_dir = tmp.join("storage");
        let dest = copy_into_storage(&source, &dest_dir, 42).unwrap();

        assert_eq!(dest, dest_dir.join("42.wav"));
        assert_eq!(std::fs::read(&dest).unwrap(), b"not really audio");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn missing_source_is_an_error_not_a_panic() {
        let tmp = std::env::temp_dir().join(format!(
            "interviewscribe-test-missing-{}",
            std::process::id()
        ));
        let result = copy_into_storage(Path::new("/no/such/file.wav"), &tmp, 1);
        assert!(result.is_err());
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "interviewscribe-content-uri-test-{}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn copy_from_reader_writes_the_full_source_content() {
        let source_path = temp_path("source.wav");
        std::fs::write(&source_path, b"content uri bytes").unwrap();
        let dest = temp_path("dest.wav");

        copy_from_reader(&dest, || std::fs::File::open(&source_path)).unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), b"content uri bytes");

        std::fs::remove_file(&source_path).ok();
        std::fs::remove_file(&dest).ok();
    }

    #[test]
    fn copy_from_reader_propagates_a_failure_to_open_the_source() {
        let dest = temp_path("never-written.wav");
        let result = copy_from_reader(&dest, || std::fs::File::open(temp_path("does-not-exist")));
        assert!(result.is_err());
        assert!(!dest.exists());
    }
}
