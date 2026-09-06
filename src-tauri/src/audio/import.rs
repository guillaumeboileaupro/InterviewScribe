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
}
