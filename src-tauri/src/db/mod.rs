pub mod edits;
pub mod interviews;
pub mod models;
pub mod schema;
pub mod segments;
pub mod speakers;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::error::AppError;
use models::InterviewDetail;

pub fn open(path: &Path) -> Result<Connection, AppError> {
    let conn = Connection::open(path)?;
    schema::init(&conn)?;
    Ok(conn)
}

pub fn get_detail(conn: &Connection, interview_id: i64) -> Result<InterviewDetail, AppError> {
    let interview = interviews::get(conn, interview_id)?;
    let speakers = speakers::list_for_interview(conn, interview_id)?;
    let segments = segments::list_for_interview(conn, interview_id)?;
    Ok(InterviewDetail {
        interview,
        speakers,
        segments,
    })
}

pub(crate) fn now_epoch_secs() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
