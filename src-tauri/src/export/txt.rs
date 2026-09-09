use super::{format_timestamp, segment_text, speaker_label, ExportOptions};
use crate::db::models::InterviewDetail;

pub fn render(detail: &InterviewDetail, options: &ExportOptions) -> String {
    let mut out = String::new();
    out.push_str(&detail.interview.title);
    out.push_str("\n\n");
    for segment in &detail.segments {
        let label = speaker_label(detail, segment.speaker_id);
        let text = segment_text(segment, options);
        if options.show_timestamps {
            out.push_str(&format!(
                "[{}] {}: {}\n",
                format_timestamp(segment.start_ms),
                label,
                text
            ));
        } else {
            out.push_str(&format!("{label}: {text}\n"));
        }
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
                language: Some("fr".into()),
                mode: "posteriori".into(),
                audio_path: "/audio/1.wav".into(),
                status: "transcribed".into(),
                error_message: None,
                notes: None,
                created_at: "0".into(),
                updated_at: "0".into(),
            },
            speakers: vec![],
            segments: vec![Segment {
                id: 1,
                interview_id: 1,
                speaker_id: None,
                start_ms: 65_000,
                end_ms: 66_000,
                raw_text: "Bonjour euh".into(),
                current_text: "Bonjour".into(),
                confidence: Some(0.9),
                status: "raw".into(),
            }],
        }
    }

    #[test]
    fn includes_timestamp_when_enabled() {
        let out = render(
            &sample_detail(),
            &ExportOptions {
                show_timestamps: true,
                use_cleaned_text: false,
            },
        );
        assert!(out.contains("[01:05]"));
        assert!(out.contains("Bonjour euh"));
    }

    #[test]
    fn hides_timestamp_when_disabled() {
        let out = render(
            &sample_detail(),
            &ExportOptions {
                show_timestamps: false,
                use_cleaned_text: false,
            },
        );
        assert!(!out.contains("01:05"));
        assert!(out.contains("Bonjour euh"));
    }

    #[test]
    fn uses_cleaned_text_when_requested() {
        let out = render(
            &sample_detail(),
            &ExportOptions {
                show_timestamps: false,
                use_cleaned_text: true,
            },
        );
        assert!(out.contains("Bonjour"));
        assert!(!out.contains("Bonjour euh"));
    }
}
