mod audio;
mod capture;
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
use tauri::{Emitter, Manager};
use transcription::Transcriber;

struct DbState(Mutex<rusqlite::Connection>);
struct RecordingState(Mutex<Option<ActiveRecording>>);

struct ActiveRecording {
    handle: capture::session::RecordingHandle,
    processing_thread: std::thread::JoinHandle<()>,
    interview_id: i64,
}

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
fn list_input_devices() -> Result<Vec<String>, AppError> {
    capture::device::list_input_devices()
}

#[tauri::command]
async fn start_recording(
    app: tauri::AppHandle,
    title: String,
    device_name: Option<String>,
    expected_speaker_count: Option<usize>,
) -> Result<db::models::Interview, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        start_recording_local(&app, title, device_name, expected_speaker_count)
    })
    .await
    .map_err(|err| AppError::Audio(format!("demarrage interrompu: {err}")))?
}

fn start_recording_local(
    app: &tauri::AppHandle,
    title: String,
    device_name: Option<String>,
    expected_speaker_count: Option<usize>,
) -> Result<db::models::Interview, AppError> {
    let recording = app.state::<RecordingState>();
    {
        let guard = recording
            .0
            .lock()
            .map_err(|_| AppError::Audio("etat d'enregistrement indisponible".into()))?;
        if guard.is_some() {
            return Err(AppError::Audio(
                "un enregistrement est deja en cours".into(),
            ));
        }
    }

    // Fail fast on missing/corrupt models before ever opening the microphone.
    let transcription::model::ModelStatus::Ready {
        path: model_path, ..
    } = transcription::model::ensure_model(app)?;
    let transcription::model::ModelStatus::Ready {
        path: diarization_model_path,
        ..
    } = transcription::model::ensure_manifest(app, transcription::model::diarization_manifest()?)?;

    let db = app.state::<DbState>();
    let audio_dir = app_data_subdir(app, "audio")?;
    let interview = {
        let conn = lock_db(&db)?;
        let interview = db::interviews::create_realtime(&conn, &title, "")?;
        let wav_path = audio_dir.join(format!("{}.wav", interview.id));
        db::interviews::set_audio_path(&conn, interview.id, &wav_path.to_string_lossy())?;
        db::interviews::get(&conn, interview.id)?
    };

    let wav_path = Path::new(&interview.audio_path).to_path_buf();
    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let handle = capture::session::start(device_name, wav_path, event_tx).inspect_err(|err| {
        if let Ok(conn) = lock_db(&db) {
            let _ =
                db::interviews::update_status(&conn, interview.id, "error", Some(&err.to_string()));
        }
    })?;

    let processing_app = app.clone();
    let interview_id = interview.id;
    let processing_thread = std::thread::spawn(move || {
        run_recording_processing(
            processing_app,
            interview_id,
            model_path,
            diarization_model_path,
            expected_speaker_count,
            event_rx,
        );
    });

    let mut guard = recording
        .0
        .lock()
        .map_err(|_| AppError::Audio("etat d'enregistrement indisponible".into()))?;
    *guard = Some(ActiveRecording {
        handle,
        processing_thread,
        interview_id: interview.id,
    });

    Ok(interview)
}

#[tauri::command]
fn pause_recording(recording: tauri::State<RecordingState>) -> Result<(), AppError> {
    let guard = recording
        .0
        .lock()
        .map_err(|_| AppError::Audio("etat d'enregistrement indisponible".into()))?;
    match guard.as_ref() {
        Some(active) => {
            active.handle.pause();
            Ok(())
        }
        None => Err(AppError::Audio("aucun enregistrement en cours".into())),
    }
}

#[tauri::command]
fn resume_recording(
    recording: tauri::State<RecordingState>,
    device_name: Option<String>,
) -> Result<(), AppError> {
    let guard = recording
        .0
        .lock()
        .map_err(|_| AppError::Audio("etat d'enregistrement indisponible".into()))?;
    match guard.as_ref() {
        Some(active) => {
            active.handle.resume(device_name);
            Ok(())
        }
        None => Err(AppError::Audio("aucun enregistrement en cours".into())),
    }
}

#[tauri::command]
async fn stop_recording(app: tauri::AppHandle) -> Result<db::models::InterviewDetail, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let recording = app.state::<RecordingState>();
        let active = {
            let mut guard = recording
                .0
                .lock()
                .map_err(|_| AppError::Audio("etat d'enregistrement indisponible".into()))?;
            guard
                .take()
                .ok_or_else(|| AppError::Audio("aucun enregistrement en cours".into()))?
        };
        active.handle.stop();
        let _ = active.processing_thread.join();
        let db = app.state::<DbState>();
        let conn = lock_db(&db)?;
        db::interviews::update_status(&conn, active.interview_id, "transcribed", None)?;
        db::get_detail(&conn, active.interview_id)
    })
    .await
    .map_err(|err| AppError::Audio(format!("arret interrompu: {err}")))?
}

/// Consumes capture events for one recording session until the capture
/// thread closes the channel (on `stop`): transcribes and diarizes each
/// finished chunk with a single long-lived `Clusterer` so speaker identity
/// stays stable across the whole session, exactly like `transcribe_local`
/// does for a whole file at once - the only difference is doing it one
/// chunk at a time, offsetting timestamps by however much came before.
fn run_recording_processing(
    app: tauri::AppHandle,
    interview_id: i64,
    model_path: String,
    diarization_model_path: String,
    expected_speaker_count: Option<usize>,
    events: std::sync::mpsc::Receiver<capture::session::SessionEvent>,
) {
    let transcriber =
        match transcription::whisper_cpp::WhisperCppTranscriber::load(Path::new(&model_path)) {
            Ok(transcriber) => transcriber,
            Err(err) => return report_recording_error(&app, interview_id, err),
        };
    let mut extractor =
        match diarization::EmbeddingExtractor::load(Path::new(&diarization_model_path)) {
            Ok(extractor) => extractor,
            Err(err) => return report_recording_error(&app, interview_id, err),
        };
    let mut clusterer = diarization::Clusterer::new(expected_speaker_count);
    let mut speakers: Vec<db::models::Speaker> = Vec::new();
    let mut elapsed_ms: i64 = 0;

    for event in events {
        match event {
            capture::session::SessionEvent::LevelUpdate { rms } => {
                let _ = app.emit("recording-level", rms);
            }
            capture::session::SessionEvent::Error(message) => {
                let _ = app.emit("recording-error", message);
            }
            capture::session::SessionEvent::ChunkReady { pcm } => {
                if pcm.is_empty() {
                    continue;
                }
                let chunk_duration_ms =
                    (pcm.len() as i64 * 1000) / audio::decode::WHISPER_SAMPLE_RATE as i64;
                let result = process_recording_chunk(
                    &app,
                    interview_id,
                    &transcriber,
                    &mut extractor,
                    &mut clusterer,
                    &mut speakers,
                    elapsed_ms,
                    &pcm,
                );
                match result {
                    Ok(()) => {
                        let _ = app.emit("segments-updated", interview_id);
                    }
                    Err(err) => {
                        let _ = app.emit("recording-error", err.to_string());
                    }
                }
                elapsed_ms += chunk_duration_ms;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_recording_chunk(
    app: &tauri::AppHandle,
    interview_id: i64,
    transcriber: &transcription::whisper_cpp::WhisperCppTranscriber,
    extractor: &mut diarization::EmbeddingExtractor,
    clusterer: &mut diarization::Clusterer,
    speakers: &mut Vec<db::models::Speaker>,
    time_offset_ms: i64,
    pcm: &[f32],
) -> Result<(), AppError> {
    let raw_segments = transcriber.transcribe(pcm, None)?;
    if raw_segments.is_empty() {
        return Ok(());
    }

    let assignments = raw_segments
        .iter()
        .map(|segment| {
            let slice = diarization::slice_pcm_ms(pcm, segment.start_ms, segment.end_ms);
            let embedding = extractor.extract(slice)?;
            Ok(clusterer.assign(&embedding))
        })
        .collect::<Result<Vec<diarization::Assignment>, AppError>>()?;

    let db = app.state::<DbState>();
    let conn = lock_db(&db)?;

    while speakers.len() < clusterer.speaker_count() {
        let index = speakers.len() + 1;
        speakers.push(db::speakers::create_numbered(&conn, interview_id, index)?);
    }

    let speaker_ids: Vec<i64> = assignments
        .iter()
        .map(|assignment| speakers[assignment.speaker_index].id)
        .collect();
    let uncertain_flags: Vec<bool> = assignments.iter().map(|a| a.uncertain).collect();

    let offset_segments: Vec<transcription::RawSegment> = raw_segments
        .into_iter()
        .map(|segment| transcription::RawSegment {
            start_ms: segment.start_ms + time_offset_ms,
            end_ms: segment.end_ms + time_offset_ms,
            text: segment.text,
            confidence: segment.confidence,
        })
        .collect();

    let new_segments = transcription::to_new_segments(offset_segments, &speaker_ids);
    let inserted_ids = db::segments::insert_batch(&conn, interview_id, &new_segments)?;
    for (segment_id, uncertain) in inserted_ids.iter().zip(&uncertain_flags) {
        if *uncertain {
            db::segments::mark_uncertain(&conn, *segment_id)?;
        }
    }
    Ok(())
}

fn report_recording_error(app: &tauri::AppHandle, interview_id: i64, err: AppError) {
    let db = app.state::<DbState>();
    if let Ok(conn) = db.0.lock() {
        let _ = db::interviews::update_status(&conn, interview_id, "error", Some(&err.to_string()));
    }
    let _ = app.emit("recording-error", err.to_string());
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
            app.manage(RecordingState(Mutex::new(None)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            application_status,
            import_interview,
            list_interviews,
            get_interview,
            ensure_whisper_model,
            transcribe_interview,
            list_input_devices,
            start_recording,
            pause_recording,
            resume_recording,
            stop_recording,
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
