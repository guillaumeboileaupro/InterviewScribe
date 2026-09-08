use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("audio error: {0}")]
    Audio(String),
    #[error("transcription error: {0}")]
    Transcription(String),
    #[error("model error: {0}")]
    Model(String),
    #[error("export error: {0}")]
    Export(String),
    #[error("edition invalide: {0}")]
    Edit(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not found: {0}")]
    NotFound(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Single choke point every command's error passes through on its way
        // to the frontend - logging here covers all of them for free. Safe:
        // every `AppError` message is a technical library/validation string
        // (see docs/ARCHITECTURE.md "Diagnostics locaux"), never audio,
        // transcript text or a speaker name.
        crate::diagnostics::log("ERROR", &self.to_string());
        serializer.serialize_str(&self.to_string())
    }
}
