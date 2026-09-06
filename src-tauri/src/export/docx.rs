use docx_rs::{Docx, Paragraph, Run};

use super::{format_timestamp, segment_text, speaker_label, ExportOptions};
use crate::db::models::InterviewDetail;
use crate::error::AppError;

pub fn render(detail: &InterviewDetail, options: &ExportOptions) -> Result<Vec<u8>, AppError> {
    let mut docx = Docx::new().add_paragraph(
        Paragraph::new().add_run(Run::new().add_text(&detail.interview.title).bold().size(32)),
    );

    for segment in &detail.segments {
        let label = speaker_label(detail, segment.speaker_id);
        let text = segment_text(segment, options);
        let heading = if options.show_timestamps {
            format!("{label} [{}]", format_timestamp(segment.start_ms))
        } else {
            label
        };
        docx = docx
            .add_paragraph(Paragraph::new().add_run(Run::new().add_text(heading).bold()))
            .add_paragraph(Paragraph::new().add_run(Run::new().add_text(text)));
    }

    let mut cursor = std::io::Cursor::new(Vec::new());
    docx.build()
        .pack(&mut cursor)
        .map_err(|err| AppError::Export(format!("erreur DOCX: {err:?}")))?;
    Ok(cursor.into_inner())
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
                raw_text: "Une hésitation « guillemets »".into(),
                current_text: "Une hésitation « guillemets »".into(),
                confidence: None,
                status: "raw".into(),
            }],
        }
    }

    #[test]
    fn produces_a_valid_docx_containing_the_segment_text() {
        let bytes = render(
            &sample_detail(),
            &ExportOptions {
                show_timestamps: true,
                use_cleaned_text: false,
            },
        )
        .unwrap();

        assert!(bytes.starts_with(b"PK"));

        let json = docx_rs::read_docx(&bytes).unwrap().json();
        assert!(json.contains("Entretien café"));
        assert!(json.contains("Une hésitation « guillemets »"));
        assert!(json.contains("00:05"));
    }
}
