use super::{format_full_timestamp, segment_text, speaker_label, ExportOptions};
use crate::db::models::InterviewDetail;

/// WebVTT always includes timestamps, for the same reason SRT and JSON do.
pub fn render(detail: &InterviewDetail, options: &ExportOptions) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for segment in &detail.segments {
        let label = speaker_label(detail, segment.speaker_id);
        let text = segment_text(segment, options);
        out.push_str(&format!(
            "{} --> {}\n{}: {}\n\n",
            format_full_timestamp(segment.start_ms, '.'),
            format_full_timestamp(segment.end_ms, '.'),
            label,
            text
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{Interview, Segment};

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
            segments: vec![Segment {
                id: 1,
                interview_id: 1,
                speaker_id: None,
                start_ms: 3_661_500,
                end_ms: 3_662_750,
                raw_text: "Bonjour".into(),
                current_text: "Bonjour".into(),
                confidence: None,
                status: "raw".into(),
            }],
        }
    }

    #[test]
    fn starts_with_the_webvtt_header() {
        let out = render(
            &sample_detail(),
            &ExportOptions {
                show_timestamps: false,
                use_cleaned_text: false,
            },
        );
        assert!(out.starts_with("WEBVTT\n\n"));
        assert!(out.contains("01:01:01.500 --> 01:01:02.750"));
    }
}
