mod audio;
mod cleanup;
mod db;
mod diarization;
mod error;
mod export;
mod transcription;

use std::path::Path;
use std::sync::Mutex;

use error::AppError;
use serde::Serialize;
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
    expected_speaker_count: Option<usize>,
) -> Result<db::models::InterviewDetail, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<DbState>();
        transcribe_local(&app, &db, interview_id, expected_speaker_count)
    })
    .await
    .map_err(|err| AppError::Transcription(format!("traitement interrompu: {err}")))?
}

fn transcribe_local(
    app: &tauri::AppHandle,
    db: &DbState,
    interview_id: i64,
    expected_speaker_count: Option<usize>,
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

    let transcription::model::ModelStatus::Ready {
        path: diarization_model_path,
        ..
    } = transcription::model::ensure_manifest(app, transcription::model::diarization_manifest()?)
        .map_err(mark_error)?;
    let mut extractor = diarization::EmbeddingExtractor::load(Path::new(&diarization_model_path))
        .map_err(mark_error)?;
    let mut clusterer = diarization::Clusterer::new(expected_speaker_count);
    let assignments = raw_segments
        .iter()
        .map(|segment| {
            let slice = diarization::slice_pcm_ms(&pcm, segment.start_ms, segment.end_ms);
            let embedding = extractor.extract(slice)?;
            Ok(clusterer.assign(&embedding))
        })
        .collect::<Result<Vec<diarization::Assignment>, AppError>>()
        .map_err(mark_error)?;

    let conn = lock_db(db)?;
    let speakers: Vec<db::models::Speaker> = (1..=clusterer.speaker_count())
        .map(|index| db::speakers::create_numbered(&conn, interview_id, index))
        .collect::<Result<_, _>>()?;
    let speaker_ids: Vec<i64> = assignments
        .iter()
        .map(|assignment| speakers[assignment.speaker_index].id)
        .collect();
    let uncertain_flags: Vec<bool> = assignments.iter().map(|a| a.uncertain).collect();

    let new_segments = transcription::to_new_segments(raw_segments, &speaker_ids);
    let inserted_ids = db::segments::insert_batch(&conn, interview_id, &new_segments)?;
    // insert_batch always writes 'raw'; segments the clusterer flagged as an
    // uncertain speaker match get promoted to 'uncertain' rather than forcing
    // silent confidence the diarization step doesn't actually have.
    for (segment_id, uncertain) in inserted_ids.iter().zip(&uncertain_flags) {
        if *uncertain {
            db::segments::mark_uncertain(&conn, *segment_id)?;
        }
    }
    db::interviews::update_status(&conn, interview_id, "transcribed", None)?;
    db::get_detail(&conn, interview_id)
}

#[tauri::command]
fn rename_speaker(
    db: tauri::State<DbState>,
    speaker_id: i64,
    display_name: String,
) -> Result<db::models::Speaker, AppError> {
    let conn = lock_db(&db)?;
    db::speakers::rename(&conn, speaker_id, &display_name)
}

#[tauri::command]
fn merge_speakers(
    db: tauri::State<DbState>,
    interview_id: i64,
    keep_id: i64,
    remove_id: i64,
) -> Result<Vec<db::models::Speaker>, AppError> {
    let conn = lock_db(&db)?;
    db::speakers::merge(&conn, interview_id, keep_id, remove_id)
}

#[tauri::command]
fn create_speaker(
    db: tauri::State<DbState>,
    interview_id: i64,
    label: String,
) -> Result<db::models::Speaker, AppError> {
    let conn = lock_db(&db)?;
    db::speakers::create_speaker(&conn, interview_id, &label)
}

#[tauri::command]
fn reassign_segment_speaker(
    db: tauri::State<DbState>,
    segment_id: i64,
    speaker_id: Option<i64>,
) -> Result<db::models::Segment, AppError> {
    let conn = lock_db(&db)?;
    db::segments::reassign_speaker(&conn, segment_id, speaker_id)
}

#[derive(Serialize)]
struct CleanupApplied {
    segment: db::models::Segment,
    outcome: cleanup::CleanupOutcome,
}

#[tauri::command]
fn apply_segment_cleanup(
    db: tauri::State<DbState>,
    segment_id: i64,
) -> Result<CleanupApplied, AppError> {
    let conn = lock_db(&db)?;
    let segment = db::segments::get(&conn, segment_id)?;
    let current = db::edits::current_text(&conn, segment_id, &segment.raw_text)?;
    let outcome = cleanup::analyze(&current);
    if outcome.cleaned_text == current {
        return Ok(CleanupApplied { segment, outcome });
    }
    db::edits::record(
        &conn,
        segment_id,
        "cleanup",
        &current,
        &outcome.cleaned_text,
    )?;
    let segment = db::segments::get(&conn, segment_id)?;
    Ok(CleanupApplied { segment, outcome })
}

#[tauri::command]
fn save_segment_edit(
    db: tauri::State<DbState>,
    segment_id: i64,
    text: String,
) -> Result<db::models::Segment, AppError> {
    if text.trim().is_empty() {
        return Err(AppError::Edit("le texte ne peut pas etre vide".into()));
    }
    let conn = lock_db(&db)?;
    let segment = db::segments::get(&conn, segment_id)?;
    let current = db::edits::current_text(&conn, segment_id, &segment.raw_text)?;
    if text != current {
        db::edits::record(&conn, segment_id, "manual", &current, &text)?;
    }
    db::segments::get(&conn, segment_id)
}

#[tauri::command]
fn undo_segment_edit(
    db: tauri::State<DbState>,
    segment_id: i64,
) -> Result<db::models::Segment, AppError> {
    let conn = lock_db(&db)?;
    let reverted = db::edits::revert_latest(&conn, segment_id)?;
    if reverted.is_none() {
        return Err(AppError::NotFound(
            "aucune modification a annuler pour ce segment".into(),
        ));
    }
    db::segments::get(&conn, segment_id)
}

#[tauri::command]
fn list_segment_edits(
    db: tauri::State<DbState>,
    segment_id: i64,
) -> Result<Vec<db::models::Edit>, AppError> {
    let conn = lock_db(&db)?;
    db::edits::list_for_segment(&conn, segment_id)
}

#[tauri::command]
fn export_interview(
    db: tauri::State<DbState>,
    interview_id: i64,
    format: String,
    show_timestamps: bool,
    use_cleaned_text: bool,
    destination_path: String,
) -> Result<(), AppError> {
    let detail = {
        let conn = lock_db(&db)?;
        db::get_detail(&conn, interview_id)?
    };
    let options = export::ExportOptions {
        show_timestamps,
        use_cleaned_text,
    };
    let rendered = export::render(&format, &detail, &options)?;
    std::fs::write(destination_path, rendered)?;
    Ok(())
}

#[tauri::command]
fn check_doc_export_available() -> bool {
    export::doc::is_available()
}

#[tauri::command]
async fn export_interview_doc(
    app: tauri::AppHandle,
    interview_id: i64,
    show_timestamps: bool,
    use_cleaned_text: bool,
    destination_path: String,
) -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<DbState>();
        let detail = {
            let conn = lock_db(&db)?;
            db::get_detail(&conn, interview_id)?
        };
        let options = export::ExportOptions {
            show_timestamps,
            use_cleaned_text,
        };
        let work_dir = app_data_subdir(&app, "export-tmp")?;
        let rendered = export::doc::render(&detail, &options, &work_dir)?;
        std::fs::write(destination_path, rendered)?;
        Ok(())
    })
    .await
    .map_err(|err| AppError::Export(format!("export DOC interrompu: {err}")))?
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
            rename_speaker,
            merge_speakers,
            create_speaker,
            reassign_segment_speaker,
            apply_segment_cleanup,
            save_segment_edit,
            undo_segment_edit,
            list_segment_edits,
            export_interview,
            export_interview_doc,
            check_doc_export_available
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
