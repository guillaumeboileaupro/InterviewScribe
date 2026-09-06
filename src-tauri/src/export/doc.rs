use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use super::{docx, ExportOptions};
use crate::db::models::InterviewDetail;
use crate::error::AppError;

static EXPORT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Whether a local LibreOffice install is available to convert DOCX to the
/// legacy binary DOC format. No Rust crate writes real .doc files directly,
/// so this is the only viable local (offline) path. The frontend is expected
/// to call this proactively so it can hide/disable the DOC export option
/// instead of only discovering unavailability after a failed attempt.
pub fn is_available() -> bool {
    which::which("soffice").is_ok()
}

/// Renders the interview as DOCX, then shells out to a local LibreOffice to
/// convert it to legacy DOC. Never touches any other export format's
/// availability: if `soffice` is missing, only this one call fails.
pub fn render(
    detail: &InterviewDetail,
    options: &ExportOptions,
    work_dir: &Path,
) -> Result<Vec<u8>, AppError> {
    if !is_available() {
        return Err(AppError::Export(
            "LibreOffice (soffice) est introuvable. Installez LibreOffice pour activer l'export DOC ; les autres formats restent disponibles.".into(),
        ));
    }

    std::fs::create_dir_all(work_dir)?;
    // A DOC conversion may run concurrently with another export. A
    // process-local sequence prevents both soffice invocations from sharing
    // (and deleting) the same temporary files.
    let sequence = EXPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let stem = format!("interviewscribe-{}-{sequence}", std::process::id());
    let docx_path = work_dir.join(format!("{stem}.docx"));
    let doc_path = work_dir.join(format!("{stem}.doc"));

    let docx_bytes = docx::render(detail, options)?;
    std::fs::write(&docx_path, &docx_bytes)?;

    let result = (|| -> Result<Vec<u8>, AppError> {
        let status = std::process::Command::new("soffice")
            .args(["--headless", "--convert-to", "doc", "--outdir"])
            .arg(work_dir)
            .arg(&docx_path)
            .status()?;
        if !status.success() {
            return Err(AppError::Export(
                "echec de la conversion DOC via LibreOffice".into(),
            ));
        }
        Ok(std::fs::read(&doc_path)?)
    })();

    // Best-effort cleanup: a failure here must never mask the real result.
    let _ = std::fs::remove_file(&docx_path);
    let _ = std::fs::remove_file(&doc_path);

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Interview;

    fn sample_detail() -> InterviewDetail {
        InterviewDetail {
            interview: Interview {
                id: 1,
                title: "Entretien test".into(),
                language: None,
                mode: "posteriori".into(),
                audio_path: "/audio/1.wav".into(),
                status: "transcribed".into(),
                error_message: None,
                created_at: "0".into(),
                updated_at: "0".into(),
            },
            speakers: vec![],
            segments: vec![],
        }
    }

    #[test]
    fn is_available_reflects_whether_soffice_is_on_path() {
        // Just exercises the real detection logic; the result depends on the
        // machine running the test, so no fixed assertion either way.
        let _ = is_available();
    }

    /// Manual QA against a real local LibreOffice install. Never runs in CI
    /// (the machine may not have `soffice`, and per AGENTS.md we never
    /// silently skip — this is explicitly opt-in via #[ignore]).
    /// Run with:
    ///   cargo test --manifest-path src-tauri/Cargo.toml export::doc -- --ignored --nocapture
    #[test]
    #[ignore]
    fn converts_to_a_real_doc_file() {
        assert!(
            is_available(),
            "soffice must be installed to run this QA test"
        );
        let work_dir = std::env::temp_dir().join("interviewscribe-doc-qa");
        let bytes = render(
            &sample_detail(),
            &ExportOptions {
                show_timestamps: true,
                use_cleaned_text: false,
            },
            &work_dir,
        )
        .unwrap();
        assert!(!bytes.is_empty());
        // A real MS Word 97 .doc file is an OLE Compound File; its magic
        // bytes are D0 CF 11 E0 A1 B1 1A E1.
        assert_eq!(
            &bytes[0..8],
            &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]
        );
    }
}
