pub mod doc;
pub mod docx;
pub mod json;
pub mod markdown;
pub mod pdf;
pub mod srt;
pub mod txt;
pub mod vtt;

use crate::db::models::{InterviewDetail, Segment};
use crate::error::AppError;

pub struct ExportOptions {
    pub show_timestamps: bool,
    /// When true, text-bearing formats (all but JSON, which always includes
    /// both) render `segment.current_text` instead of `segment.raw_text`.
    pub use_cleaned_text: bool,
}

/// Renders an interview in the given format. `format` is one of "txt",
/// "markdown" or "json". JSON always retains timestamps regardless of
/// `options.show_timestamps`, since it exists to preserve the complete
/// structure (see docs/PRODUCT.md's Export section).
pub fn render(
    format: &str,
    detail: &InterviewDetail,
    options: &ExportOptions,
) -> Result<Vec<u8>, AppError> {
    match format {
        "txt" => Ok(txt::render(detail, options).into_bytes()),
        "markdown" | "md" => Ok(markdown::render(detail, options).into_bytes()),
        "json" => json::render(detail).map(String::into_bytes),
        "srt" => Ok(srt::render(detail, options).into_bytes()),
        "vtt" => Ok(vtt::render(detail, options).into_bytes()),
        "docx" => docx::render(detail, options),
        "pdf" => pdf::render(detail, options),
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

/// The text to render for one segment, according to `options.use_cleaned_text`.
pub(crate) fn segment_text<'a>(segment: &'a Segment, options: &ExportOptions) -> &'a str {
    if options.use_cleaned_text {
        &segment.current_text
    } else {
        &segment.raw_text
    }
}

pub(crate) fn format_timestamp(ms: i64) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}

/// `HH:MM:SS<sep>mmm`, used by both SRT (comma) and WebVTT (period).
pub(crate) fn format_full_timestamp(ms: i64, decimal_sep: char) -> String {
    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1000;
    let millis = ms % 1000;
    format!("{hours:02}:{minutes:02}:{seconds:02}{decimal_sep}{millis:03}")
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
            "rtf",
            &detail,
            &ExportOptions {
                show_timestamps: true,
                use_cleaned_text: false,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn format_timestamp_pads_minutes_and_seconds() {
        assert_eq!(format_timestamp(0), "00:00");
        assert_eq!(format_timestamp(65_000), "01:05");
    }

    #[test]
    fn format_full_timestamp_handles_hour_rollover_and_separator() {
        assert_eq!(format_full_timestamp(0, ','), "00:00:00,000");
        assert_eq!(format_full_timestamp(3_661_500, ','), "01:01:01,500");
        assert_eq!(format_full_timestamp(3_661_500, '.'), "01:01:01.500");
    }
}
