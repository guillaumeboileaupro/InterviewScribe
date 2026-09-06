use genpdf::{elements, fonts, Document};

use super::{format_timestamp, segment_text, speaker_label, ExportOptions};
use crate::db::models::InterviewDetail;
use crate::error::AppError;

// Embedded directly (unlike the Whisper model): small, permissively licensed
// (DejaVu/Bitstream Vera, see resources/fonts/LICENSE), no Tauri resource
// resolution or Android extraction needed since the bytes are compiled in.
const REGULAR: &[u8] = include_bytes!("../../resources/fonts/DejaVuSans.ttf");
const BOLD: &[u8] = include_bytes!("../../resources/fonts/DejaVuSans-Bold.ttf");

fn font_family() -> Result<fonts::FontFamily<fonts::FontData>, AppError> {
    let load = |bytes: &[u8]| -> Result<fonts::FontData, AppError> {
        fonts::FontData::new(bytes.to_vec(), None)
            .map_err(|err| AppError::Export(format!("erreur de police PDF: {err}")))
    };
    Ok(fonts::FontFamily {
        regular: load(REGULAR)?,
        bold: load(BOLD)?,
        italic: load(REGULAR)?,
        bold_italic: load(BOLD)?,
    })
}

pub fn render(detail: &InterviewDetail, options: &ExportOptions) -> Result<Vec<u8>, AppError> {
    let mut doc = Document::new(font_family()?);
    // Deliberately not calling `doc.set_title()`: printpdf 0.3.4's PDF info
    // dictionary mis-encodes non-ASCII title metadata (verified: "café"
    // becomes "cafÃ©" in `pdfinfo`), even though the embedded-font body text
    // below renders and extracts correctly. Wrong metadata is worse than no
    // metadata, and the title is already the document's first line anyway.
    doc.push(elements::Paragraph::new(&detail.interview.title));
    doc.push(elements::Break::new(1));

    for segment in &detail.segments {
        let label = speaker_label(detail, segment.speaker_id);
        let text = segment_text(segment, options);
        let heading = if options.show_timestamps {
            format!("{label} [{}]", format_timestamp(segment.start_ms))
        } else {
            label
        };
        doc.push(elements::Paragraph::new(heading));
        doc.push(elements::Paragraph::new(text));
        doc.push(elements::Break::new(1));
    }

    let mut buffer = Vec::new();
    doc.render(&mut buffer)
        .map_err(|err| AppError::Export(format!("erreur PDF: {err}")))?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{Interview, Segment};

    fn sample_detail() -> InterviewDetail {
        InterviewDetail {
            interview: Interview {
                id: 1,
                title: "Entretien café".into(),
                language: None,
                mode: "posteriori".into(),
                audio_path: "/audio/1.wav".into(),
                status: "transcribed".into(),
                error_message: None,
                created_at: "0".into(),
                updated_at: "0".into(),
            },
            speakers: vec![],
            segments: vec![Segment {
                id: 1,
                interview_id: 1,
                speaker_id: None,
                start_ms: 5_000,
                end_ms: 6_000,
                raw_text: "Une hésitation".into(),
                current_text: "Une hésitation".into(),
                confidence: None,
                status: "raw".into(),
            }],
        }
    }

    #[test]
    fn produces_a_valid_pdf() {
        let bytes = render(
            &sample_detail(),
            &ExportOptions {
                show_timestamps: true,
                use_cleaned_text: false,
            },
        )
        .unwrap();
        assert!(bytes.starts_with(b"%PDF"));
        assert!(bytes.len() > 1000);
    }

    /// Manual QA: writes a real multi-page PDF with accented French text to
    /// disk for visual/`pdftotext`/`pdfinfo` inspection. Never runs in CI.
    /// Run with:
    ///   cargo test --manifest-path src-tauri/Cargo.toml export::pdf -- --ignored --nocapture
    #[test]
    #[ignore]
    fn write_manual_qa_pdf() {
        let mut detail = sample_detail();
        detail.segments = (0..80)
            .map(|i| Segment {
                id: i,
                interview_id: 1,
                speaker_id: None,
                start_ms: i * 5_000,
                end_ms: i * 5_000 + 4_000,
                raw_text: format!(
                    "Segment {i}: café, œuvre, hésitation, « guillemets », un tiret – et voilà."
                ),
                current_text: format!(
                    "Segment {i}: café, œuvre, hésitation, « guillemets », un tiret – et voilà."
                ),
                confidence: None,
                status: "raw".into(),
            })
            .collect();

        let bytes = render(
            &detail,
            &ExportOptions {
                show_timestamps: true,
                use_cleaned_text: false,
            },
        )
        .unwrap();

        let path = std::env::temp_dir().join("interviewscribe-qa.pdf");
        std::fs::write(&path, &bytes).unwrap();
        println!("wrote {}", path.display());
    }
}
