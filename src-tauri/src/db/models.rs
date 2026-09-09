use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Interview {
    pub id: i64,
    pub title: String,
    pub notes: Option<String>,
    pub language: Option<String>,
    pub mode: String,
    pub audio_path: String,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Speaker {
    pub id: i64,
    pub interview_id: i64,
    pub label: String,
    pub color: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Segment {
    pub id: i64,
    pub interview_id: i64,
    pub speaker_id: Option<i64>,
    pub start_ms: i64,
    pub end_ms: i64,
    pub raw_text: String,
    /// Derived, never persisted on the row: the latest non-reverted edit's
    /// text, or `raw_text` if the segment has never been edited.
    pub current_text: String,
    pub confidence: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Edit {
    pub id: i64,
    pub segment_id: i64,
    pub operation: String,
    pub before_text: String,
    pub after_text: String,
    pub created_at: String,
    pub reverted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InterviewDetail {
    pub interview: Interview,
    pub speakers: Vec<Speaker>,
    pub segments: Vec<Segment>,
}
