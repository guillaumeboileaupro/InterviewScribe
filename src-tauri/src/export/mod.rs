pub mod json;
pub mod markdown;
pub mod txt;

use crate::db::models::InterviewDetail;
use crate::error::AppError;

pub struct ExportOptions {
    pub show_timestamps: bool,
}

/// Renders an interview in the given format. `format` is one of "txt",
/// "markdown" or "json". JSON always retains timestamps regardless of
/// `options.show_timestamps`, since it exists to preserve the complete
/// structure (see docs/PRODUCT.md's Export section).
pub fn render(
    format: &str,
    detail: &InterviewDetail,
    options: &ExportOptions,
) -> Result<String, AppError> {
    match format {
        "txt" => Ok(txt::render(detail, options)),
        "markdown" | "md" => Ok(markdown::render(detail, options)),
        "json" => json::render(detail),
        other => Err(AppError::Export(format!(
            "format d'export inconnu: {other}"
        ))),
    }
}

pub(crate) fn speaker_label(detail: &InterviewDetail, speaker_id: Option<i64>) -> String {
    speaker_id
        .and_then(|id| detail.speakers.iter().find(|speaker| speaker.id == id))
        .map(|speaker| {
            speaker
                .display_name
                .clone()
                .unwrap_or_else(|| speaker.label.clone())
        })
        .unwrap_or_else(|| "Intervenant".to_string())
}

pub(crate) fn format_timestamp(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_format_is_an_error() {
        let detail = InterviewDetail {
            interview: crate::db::models::Interview {
                id: 1,
                title: "Test".into(),
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
        };
        let result = render(
            "pdf",
            &detail,
            &ExportOptions {
                show_timestamps: true,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn format_timestamp_pads_minutes_and_seconds() {
        assert_eq!(format_timestamp(0), "00:00");
        assert_eq!(format_timestamp(65_000), "01:05");
    }
}
