use super::{format_timestamp, segment_text, speaker_label, ExportOptions};
use crate::db::models::InterviewDetail;

pub fn render(detail: &InterviewDetail, options: &ExportOptions) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", detail.interview.title));
    for segment in &detail.segments {
        let label = speaker_label(detail, segment.speaker_id);
        let text = segment_text(segment, options);
        if options.show_timestamps {
            out.push_str(&format!(
                "**{}** _{}_\n\n{}\n\n",
                label,
                format_timestamp(segment.start_ms),
                text
            ));
        } else {
            out.push_str(&format!("**{label}**\n\n{text}\n\n"));
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
                language: None,
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
                start_ms: 5_000,
                end_ms: 6_000,
                raw_text: "Bonjour".into(),
                current_text: "Bonjour".into(),
                confidence: None,
                status: "raw".into(),
            }],
        }
    }

    #[test]
    fn renders_title_as_heading() {
        let out = render(
            &sample_detail(),
            &ExportOptions {
                show_timestamps: true,
                use_cleaned_text: false,
            },
        );
        assert!(out.starts_with("# Entretien test\n"));
        assert!(out.contains("00:05"));
    }
}
