mod audio;
mod db;
mod error;
mod export;
mod transcription;

use std::path::Path;
use std::sync::Mutex;

use error::AppError;
use tauri::Manager;
use transcription::Transcriber;

struct DbState(Mutex<rusqlite::Connection>);

fn app_data_subdir(app: &tauri::AppHandle, name: &str) -> Result<std::path::PathBuf, AppError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|err| AppError::Io(std::io::Error::other(err.to_string())))?;
    let dir = base.join(name);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn lock_db(db: &DbState) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, AppError> {
    db.0.lock()
        .map_err(|_| AppError::Db(rusqlite::Error::InvalidQuery))
}

#[tauri::command]
fn application_status() -> &'static str {
    "foundation"
}

#[tauri::command]
fn import_interview(
    app: tauri::AppHandle,
    db: tauri::State<DbState>,
    title: String,
    source_path: String,
    language: Option<String>,
) -> Result<db::models::Interview, AppError> {
    let conn = lock_db(&db)?;
    let interview = db::interviews::create(&conn, &title, language.as_deref(), &source_path)?;
    let audio_dir = app_data_subdir(&app, "audio")?;
    let dest = audio::import::copy_into_storage(Path::new(&source_path), &audio_dir, interview.id)?;
    db::interviews::set_audio_path(&conn, interview.id, &dest.to_string_lossy())?;
    db::interviews::get(&conn, interview.id)
}

#[tauri::command]
fn list_interviews(db: tauri::State<DbState>) -> Result<Vec<db::models::Interview>, AppError> {
    let conn = lock_db(&db)?;
    db::interviews::list(&conn)
}

#[tauri::command]
fn get_interview(
    db: tauri::State<DbState>,
    interview_id: i64,
) -> Result<db::models::InterviewDetail, AppError> {
    let conn = lock_db(&db)?;
    db::get_detail(&conn, interview_id)
}

#[tauri::command]
async fn ensure_whisper_model(
    app: tauri::AppHandle,
) -> Result<transcription::model::ModelStatus, AppError> {
    tauri::async_runtime::spawn_blocking(move || transcription::model::ensure_model(&app))
        .await
        .map_err(|err| AppError::Model(format!("verification du modele interrompue: {err}")))?
}

#[tauri::command]
async fn transcribe_interview(
    app: tauri::AppHandle,
    interview_id: i64,
) -> Result<db::models::InterviewDetail, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<DbState>();
        transcribe_local(&app, &db, interview_id)
    })
    .await
    .map_err(|err| AppError::Transcription(format!("traitement interrompu: {err}")))?
}

fn transcribe_local(
    app: &tauri::AppHandle,
    db: &DbState,
    interview_id: i64,
) -> Result<db::models::InterviewDetail, AppError> {
    let mark_error = |err: AppError| -> AppError {
        if let Ok(conn) = db.0.lock() {
            let _ =
                db::interviews::update_status(&conn, interview_id, "error", Some(&err.to_string()));
        }
        err
    };

    let transcription::model::ModelStatus::Ready {
        path: model_path, ..
    } = transcription::model::ensure_model(app)?;

    let interview = {
        let conn = lock_db(db)?;
        let interview = db::interviews::get(&conn, interview_id)?;
        if interview.status == "transcribing" {
            return Err(AppError::Transcription(
                "cet entretien est deja en cours de traitement".into(),
            ));
        }
        if interview.status == "transcribed"
            || !db::segments::list_for_interview(&conn, interview_id)?.is_empty()
        {
            return db::get_detail(&conn, interview_id);
        }
        db::interviews::update_status(&conn, interview_id, "transcribing", None)?;
        interview
    };

    let pcm = audio::decode::decode_to_mono_pcm16k(Path::new(&interview.audio_path))
        .map_err(mark_error)?;

    let transcriber =
        transcription::whisper_cpp::WhisperCppTranscriber::load(Path::new(&model_path))
            .map_err(mark_error)?;
    let raw_segments = transcriber
        .transcribe(&pcm, interview.language.as_deref())
        .map_err(mark_error)?;

    let conn = lock_db(db)?;
    let speaker = db::speakers::create_default(&conn, interview_id)?;
    let new_segments = transcription::to_new_segments(raw_segments, speaker.id);
    db::segments::insert_batch(&conn, interview_id, &new_segments)?;
    db::interviews::update_status(&conn, interview_id, "transcribed", None)?;
    db::get_detail(&conn, interview_id)
}

#[tauri::command]
fn export_interview(
    db: tauri::State<DbState>,
    interview_id: i64,
    format: String,
    show_timestamps: bool,
    destination_path: String,
) -> Result<(), AppError> {
    let detail = {
        let conn = lock_db(&db)?;
        db::get_detail(&conn, interview_id)?
    };
    let options = export::ExportOptions { show_timestamps };
    let rendered = export::render(&format, &detail, &options)?;
    std::fs::write(destination_path, rendered)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "android")]
    let builder = builder.plugin(tauri_plugin_fs::init());
    builder
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let base = app.path().app_data_dir()?;
            std::fs::create_dir_all(&base)?;
            let conn = db::open(&base.join("interviewscribe.sqlite3"))?;
            app.manage(DbState(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            application_status,
            import_interview,
            list_interviews,
            get_interview,
            ensure_whisper_model,
            transcribe_interview,
            export_interview
        ])
        .run(tauri::generate_context!())
        .expect("failed to run InterviewScribe");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_status_reports_foundation() {
        assert_eq!(application_status(), "foundation");
    }
}
