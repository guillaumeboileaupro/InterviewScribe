use serde::Serialize;

use super::speaker_label;
use crate::db::models::InterviewDetail;
use crate::error::AppError;

#[derive(Serialize)]
struct JsonSegment<'a> {
    start_ms: i64,
    end_ms: i64,
    speaker: String,
    text: &'a str,
    confidence: Option<f64>,
}

#[derive(Serialize)]
struct JsonExport<'a> {
    title: &'a str,
    language: &'a Option<String>,
    segments: Vec<JsonSegment<'a>>,
}

/// Always includes timestamps: JSON's purpose is preserving the complete
/// structure (docs/PRODUCT.md), unlike TXT/Markdown where the toggle is a
/// display concern.
pub fn render(detail: &InterviewDetail) -> Result<String, AppError> {
    let segments = detail
        .segments
        .iter()
        .map(|segment| JsonSegment {
            start_ms: segment.start_ms,
            end_ms: segment.end_ms,
            speaker: speaker_label(detail, segment.speaker_id),
            text: &segment.raw_text,
            confidence: segment.confidence,
        })
        .collect();

    let payload = JsonExport {
        title: &detail.interview.title,
        language: &detail.interview.language,
        segments,
    };
    serde_json::to_string_pretty(&payload)
        .map_err(|err| AppError::Export(format!("erreur JSON: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{Interview, Segment};

    #[test]
    fn always_includes_timestamps() {
        let detail = InterviewDetail {
            interview: Interview {
                id: 1,
                title: "Entretien test".into(),
                language: Some("fr".into()),
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
                start_ms: 1_000,
                end_ms: 2_000,
                raw_text: "Bonjour".into(),
                confidence: Some(0.8),
                status: "raw".into(),
            }],
        };
        let json = render(&detail).unwrap();
        assert!(json.contains("\"start_ms\": 1000"));
        assert!(json.contains("\"end_ms\": 2000"));
        assert!(json.contains("Bonjour"));
    }
}
