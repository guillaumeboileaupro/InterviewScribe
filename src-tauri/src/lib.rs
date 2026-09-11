mod audio;
mod capture;
mod cleanup;
mod db;
mod diagnostics;
mod diarization;
mod error;
pub mod evaluation;
mod export;
pub mod performance;
mod recovery;
mod transcription;

use std::path::Path;
use std::sync::Mutex;

use error::AppError;
use serde::Serialize;
use tauri::{Emitter, Manager};
use transcription::Transcriber;

struct DbState(Mutex<rusqlite::Connection>);
struct RecordingState(Mutex<Option<ActiveRecording>>);
struct TranscriptionState(Mutex<Option<ActiveTranscription>>);

struct ActiveRecording {
    handle: capture::session::RecordingHandle,
    processing_thread: std::thread::JoinHandle<()>,
    telemetry_thread: std::thread::JoinHandle<()>,
    interview_id: i64,
}

/// A posteriori transcription currently running in `transcribe_local`, so
/// `stop_transcription` has something to signal. Mirrors `ActiveRecording`'s
/// shape, but the "handle" here is just a flag `transcribe_local`'s chunk
/// loop polls between chunks - there is no live stream to pause/resume, only
/// a clean point to stop at.
struct ActiveTranscription {
    interview_id: i64,
    stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

struct TranscriptionSlotGuard {
    app: tauri::AppHandle,
}

impl Drop for TranscriptionSlotGuard {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.app.state::<TranscriptionState>().0.lock() {
            guard.take();
        }
    }
}

/// Emitted as `"chunk-progress"` while `transcribe_local` works through the
/// chunks a posteriori import got split into (see `transcription::chunk_pcm`),
/// giving real, observable progress instead of whisper.cpp's own single-file
/// heuristic percentage - the same split that makes `stop_transcription`
/// possible between chunks.
#[derive(Serialize, Clone)]
struct ChunkProgress {
    interview_id: i64,
    chunk_index: usize,
    chunk_count: usize,
    chunk_start_ms: i64,
    chunk_end_ms: i64,
    audio_duration_ms: i64,
    estimated_remaining_ms: i64,
}

#[derive(Serialize, Clone)]
struct TaskProgress {
    interview_id: i64,
    task: &'static str,
    stage: &'static str,
    stage_label: &'static str,
    percent: i32,
}

fn emit_task_progress(
    app: &tauri::AppHandle,
    interview_id: i64,
    task: &'static str,
    stage: &'static str,
    stage_label: &'static str,
    percent: i32,
) {
    let _ = app.emit(
        "task-progress",
        TaskProgress {
            interview_id,
            task,
            stage,
            stage_label,
            percent: percent.clamp(0, 100),
        },
    );
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
    #[cfg(not(target_os = "android"))]
    let dest = audio::import::copy_into_storage(Path::new(&source_path), &audio_dir, interview.id)?;
    #[cfg(target_os = "android")]
    let dest =
        audio::import::copy_content_uri_into_storage(&app, &source_path, &audio_dir, interview.id)?;
    db::interviews::set_audio_path(&conn, interview.id, &dest.to_string_lossy())?;
    db::interviews::get(&conn, interview.id)
}

#[tauri::command]
fn list_interviews(db: tauri::State<DbState>) -> Result<Vec<db::models::Interview>, AppError> {
    let conn = lock_db(&db)?;
    db::interviews::list(&conn)
}

#[tauri::command]
fn list_recent_segments(
    db: tauri::State<DbState>,
    interview_id: i64,
    limit: usize,
) -> Result<Vec<db::models::Segment>, AppError> {
    let conn = lock_db(&db)?;
    db::segments::list_recent_for_interview(&conn, interview_id, limit)
}

#[tauri::command]
fn list_recovery_candidates(
    db: tauri::State<DbState>,
    recording: tauri::State<RecordingState>,
) -> Result<Vec<db::models::Interview>, AppError> {
    let active_interview_id = recording
        .0
        .lock()
        .map_err(|_| AppError::Audio("etat d'enregistrement indisponible".into()))?
        .as_ref()
        .map(|active| active.interview_id);
    let conn = lock_db(&db)?;
    db::interviews::list_recovery_candidates(&conn, active_interview_id)
}

#[tauri::command]
fn inspect_recovery_candidate(
    app: tauri::AppHandle,
    db: tauri::State<DbState>,
    recording: tauri::State<RecordingState>,
    interview_id: i64,
) -> Result<recovery::RecoveryInspection, AppError> {
    let active = recording
        .0
        .lock()
        .map_err(|_| AppError::Audio("etat d'enregistrement indisponible".into()))?;
    if active
        .as_ref()
        .is_some_and(|recording| recording.interview_id == interview_id)
    {
        return Err(AppError::Audio(
            "recuperation refusee: l'enregistrement est encore actif".into(),
        ));
    }
    let audio_dir = app_data_subdir(&app, "audio")?;
    let conn = lock_db(&db)?;
    recovery::inspect(&conn, interview_id, &audio_dir)
}

#[tauri::command]
fn keep_interrupted_as_is(
    db: tauri::State<DbState>,
    recording: tauri::State<RecordingState>,
    interview_id: i64,
) -> Result<db::models::InterviewDetail, AppError> {
    if recording
        .0
        .lock()
        .map_err(|_| AppError::Audio("etat d'enregistrement indisponible".into()))?
        .as_ref()
        .is_some_and(|active| active.interview_id == interview_id)
    {
        return Err(AppError::Audio(
            "conservation refusee: l'enregistrement est encore actif".into(),
        ));
    }
    let conn = lock_db(&db)?;
    let interview = db::interviews::get(&conn, interview_id)?;
    if interview.mode != "realtime" || interview.status != "transcribing" {
        return Err(AppError::Audio(
            "cette session n'est pas un enregistrement interrompu".into(),
        ));
    }
    db::interviews::update_status(&conn, interview_id, "transcribed", None)?;
    db::get_detail(&conn, interview_id)
}

/// Posteriori counterpart to `keep_interrupted_as_is`: kept separate rather
/// than widening that function's mode check, since its "still active" guard
/// reads `RecordingState` (never populated by a posteriori job) - a shared
/// function would need both states threaded through it for one extra mode.
#[tauri::command]
fn keep_posteriori_as_is(
    db: tauri::State<DbState>,
    transcription: tauri::State<TranscriptionState>,
    interview_id: i64,
) -> Result<db::models::InterviewDetail, AppError> {
    if transcription
        .0
        .lock()
        .map_err(|_| AppError::Transcription("etat de transcription indisponible".into()))?
        .as_ref()
        .is_some_and(|active| active.interview_id == interview_id)
    {
        return Err(AppError::Transcription(
            "conservation refusee: la transcription est encore active".into(),
        ));
    }
    let conn = lock_db(&db)?;
    let interview = db::interviews::get(&conn, interview_id)?;
    if interview.mode != "posteriori"
        || !matches!(interview.status.as_str(), "transcribing" | "error")
    {
        return Err(AppError::Transcription(
            "cet entretien n'est pas une transcription arretee".into(),
        ));
    }
    db::interviews::update_status(&conn, interview_id, "transcribed", None)?;
    db::get_detail(&conn, interview_id)
}

#[tauri::command]
async fn recover_interview(
    app: tauri::AppHandle,
    interview_id: i64,
    model_id: Option<String>,
) -> Result<db::models::InterviewDetail, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<DbState>();
        let recording = app.state::<RecordingState>();
        if recording
            .0
            .lock()
            .map_err(|_| AppError::Audio("etat d'enregistrement indisponible".into()))?
            .as_ref()
            .is_some_and(|active| active.interview_id == interview_id)
        {
            return Err(AppError::Audio(
                "recuperation refusee: l'enregistrement est encore actif".into(),
            ));
        }
        recover_interview_local(&app, &db, interview_id, model_id)
    })
    .await
    .map_err(|err| AppError::Transcription(format!("recuperation interrompue: {err}")))?
}

fn recover_interview_local(
    app: &tauri::AppHandle,
    db: &DbState,
    interview_id: i64,
    model_id: Option<String>,
) -> Result<db::models::InterviewDetail, AppError> {
    emit_task_progress(
        app,
        interview_id,
        "recovery",
        "model",
        "Vérification de l’enregistrement",
        5,
    );
    let audio_dir = app_data_subdir(app, "audio")?;
    let conn = lock_db(db)?;
    let inspection = recovery::inspect(&conn, interview_id, &audio_dir)?;
    let interview = db::interviews::get(&conn, interview_id)?;
    let existing = db::segments::list_for_interview(&conn, interview_id)?;
    drop(conn);

    let pcm = audio::decode::decode_to_mono_pcm16k(Path::new(&interview.audio_path))?;
    emit_task_progress(
        app,
        interview_id,
        "recovery",
        "decode",
        "Préparation de l’audio restant",
        18,
    );
    let transcription::model::ModelStatus::Ready { path, .. } =
        transcription::model::ensure_manifest(
            app,
            transcription::model::selected_whisper_manifest(model_id.as_deref())?,
        )?;
    let transcriber = transcription::whisper_cpp::WhisperCppTranscriber::load(Path::new(&path))?;
    emit_task_progress(
        app,
        interview_id,
        "recovery",
        "whisper",
        "Récupération de la transcription",
        24,
    );
    let recovered = recovery::transcribe_remainder(
        &transcriber,
        &pcm,
        interview.language.as_deref(),
        inspection.last_stable_end_ms,
        &existing,
    )?;
    emit_task_progress(
        app,
        interview_id,
        "recovery",
        "save",
        "Sauvegarde des segments récupérés",
        96,
    );

    let conn = lock_db(db)?;
    recovery::insert_as_uncertain(&conn, interview_id, recovered)?;
    db::interviews::update_status(&conn, interview_id, "transcribed", None)?;
    emit_task_progress(
        app,
        interview_id,
        "recovery",
        "done",
        "Récupération terminée",
        100,
    );
    db::get_detail(&conn, interview_id)
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
fn update_interview_notes(
    db: tauri::State<DbState>,
    interview_id: i64,
    notes: Option<String>,
) -> Result<db::models::Interview, AppError> {
    let conn = lock_db(&db)?;
    db::interviews::update_notes(&conn, interview_id, notes.as_deref())?;
    db::interviews::get(&conn, interview_id)
}

#[tauri::command]
fn delete_interview(
    app: tauri::AppHandle,
    db: tauri::State<DbState>,
    recording: tauri::State<RecordingState>,
    interview_id: i64,
) -> Result<(), AppError> {
    let active = recording
        .0
        .lock()
        .map_err(|_| AppError::Audio("etat d'enregistrement indisponible".into()))?;
    if active
        .as_ref()
        .is_some_and(|recording| recording.interview_id == interview_id)
    {
        return Err(AppError::Audio(
            "impossible de supprimer un enregistrement en cours".into(),
        ));
    }
    let audio_dir = app_data_subdir(&app, "audio")?;
    let mut conn = lock_db(&db)?;
    db::interviews::delete_with_managed_audio(&mut conn, interview_id, &audio_dir)
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
fn list_available_models() -> Result<Vec<transcription::model::ModelOption>, AppError> {
    transcription::model::list_whisper_models()
}

#[tauri::command]
async fn transcribe_interview(
    app: tauri::AppHandle,
    interview_id: i64,
    expected_speaker_count: Option<usize>,
    model_id: Option<String>,
) -> Result<db::models::InterviewDetail, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<DbState>();
        transcribe_local(&app, &db, interview_id, expected_speaker_count, model_id)
    })
    .await
    .map_err(|err| AppError::Transcription(format!("traitement interrompu: {err}")))?
}

const POSTERIORI_CHUNK_MAX_MS: u32 = 30_000;
const POSTERIORI_CHUNK_SILENCE_MS: u32 = 800;

/// A posteriori import is decoded once, then split into transcription-ready
/// chunks (`transcription::chunk_pcm`) instead of one uninterruptible
/// whisper.cpp pass over the whole file: each chunk is transcribed,
/// diarized and persisted immediately, `"chunk-progress"` reports real
/// progress after each one, and `stop_transcription` can request a clean
/// stop between chunks without losing anything already inserted. Calling
/// this again on a partially-transcribed interview resumes from whatever
/// was last persisted (`boundary_ms` below) instead of restarting - this
/// one function is the entry point for both a first pass and a resume.
fn transcribe_local(
    app: &tauri::AppHandle,
    db: &DbState,
    interview_id: i64,
    expected_speaker_count: Option<usize>,
    model_id: Option<String>,
) -> Result<db::models::InterviewDetail, AppError> {
    let stop_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let state = app.state::<TranscriptionState>();
        let mut active = state
            .0
            .lock()
            .map_err(|_| AppError::Transcription("etat de transcription indisponible".into()))?;
        if active.is_some() {
            return Err(AppError::Transcription(
                "une transcription est deja en cours".into(),
            ));
        }
        *active = Some(ActiveTranscription {
            interview_id,
            stop_flag: stop_flag.clone(),
        });
    }
    let _transcription_slot = TranscriptionSlotGuard { app: app.clone() };

    // Persist the active state before model loading or audio decoding. If the
    // process closes during either long preparation step, the library can
    // still discover and resume this interview on the next launch.
    let interview = {
        let conn = lock_db(db)?;
        let interview = db::interviews::get(&conn, interview_id)?;
        if interview.status == "transcribed" {
            return db::get_detail(&conn, interview_id);
        }
        // Checked before flipping the status: a resume on a moved/deleted
        // source file should fail immediately with a clear message, not
        // flip to "transcribing" only to bounce straight to "error" once
        // decoding fails below - and not fail confusingly deep inside a
        // generic IO error either.
        if !Path::new(&interview.audio_path).exists() {
            return Err(AppError::Audio(
                "le fichier audio de cet entretien est introuvable - il a peut-etre ete deplace ou supprime".into(),
            ));
        }
        if interview.status != "transcribing" {
            db::interviews::update_status(&conn, interview_id, "transcribing", None)?;
        }
        interview
    };

    diagnostics::log("INFO", "transcription: verification du modele");
    emit_task_progress(
        app,
        interview_id,
        "transcription",
        "model",
        "Vérification du modèle local",
        5,
    );
    let mark_error = |err: AppError| -> AppError {
        if let Ok(conn) = db.0.lock() {
            let _ =
                db::interviews::update_status(&conn, interview_id, "error", Some(&err.to_string()));
        }
        err
    };

    let transcription::model::ModelStatus::Ready {
        path: model_path, ..
    } = transcription::model::ensure_manifest(
        app,
        transcription::model::selected_whisper_manifest(model_id.as_deref())?,
    )?;
    let transcription::model::ModelStatus::Ready {
        path: diarization_model_path,
        ..
    } = transcription::model::ensure_manifest(app, transcription::model::diarization_manifest()?)?;

    if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
        diagnostics::log("INFO", "transcription: arretee pendant la preparation");
        let conn = lock_db(db)?;
        return db::get_detail(&conn, interview_id);
    }

    let pcm = audio::decode::decode_to_mono_pcm16k(Path::new(&interview.audio_path))
        .map_err(mark_error)?;
    diagnostics::log("INFO", "transcription: audio decode");
    emit_task_progress(
        app,
        interview_id,
        "transcription",
        "decode",
        "Décodage et normalisation audio",
        18,
    );
    if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
        diagnostics::log("INFO", "transcription: arretee apres le decodage");
        let conn = lock_db(db)?;
        return db::get_detail(&conn, interview_id);
    }

    // Resume point: pick up after whatever's already durably persisted for
    // this interview (0 on a first pass, or after a previous stop) - the
    // same one-line derivation `recovery.rs` uses for interrupted realtime
    // sessions, reimplemented locally here so this stays independent of
    // that module.
    let boundary_ms = {
        let conn = lock_db(db)?;
        let durable_cursor = db::interviews::transcription_cursor_ms(&conn, interview_id)?;
        if durable_cursor > 0 {
            durable_cursor
        } else {
            // Compatibility with partial transcriptions created before the
            // durable cursor migration. New chunks always use the exact end
            // of the committed audio block instead of this approximation.
            db::segments::list_for_interview(&conn, interview_id)?
                .iter()
                .map(|segment| segment.end_ms)
                .max()
                .unwrap_or(0)
        }
    };
    let boundary_sample =
        ((boundary_ms.max(0) as u64) * audio::decode::WHISPER_SAMPLE_RATE as u64 / 1_000) as usize;
    let remaining_pcm = &pcm[boundary_sample.min(pcm.len())..];

    let existing_speaker_count = {
        let conn = lock_db(db)?;
        db::speakers::list_for_interview(&conn, interview_id)?.len()
    };
    let transcriber =
        transcription::whisper_cpp::WhisperCppTranscriber::load(Path::new(&model_path))
            .map_err(mark_error)?;
    // A fresh assigner every call, including a resume: any `Clusterer`
    // state from before a stop lived only in memory and is gone now. Never
    // pretend continuity that isn't there - new segments get their own,
    // independently-numbered speakers, the same honesty principle
    // `recovery::insert_as_uncertain` already applies to interrupted
    // realtime sessions.
    let mut assigner = SpeakerAssigner::new(
        &diarization_model_path,
        expected_speaker_count,
        existing_speaker_count,
        boundary_ms > 0,
    )
    .map_err(mark_error)?;

    let chunks = transcription::chunk_pcm_ranges(
        remaining_pcm,
        POSTERIORI_CHUNK_MAX_MS,
        POSTERIORI_CHUNK_SILENCE_MS,
    );
    let chunk_count = chunks.len();
    if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
        diagnostics::log("INFO", "transcription: arretee apres le decoupage");
        let conn = lock_db(db)?;
        return db::get_detail(&conn, interview_id);
    }

    let mut offset_ms = boundary_ms;
    let mut stopped_early = false;
    let transcription_started = std::time::Instant::now();
    let audio_duration_ms = (pcm.len() as i64 * 1_000) / audio::decode::WHISPER_SAMPLE_RATE as i64;
    for (index, chunk_range) in chunks.into_iter().enumerate() {
        if stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            stopped_early = true;
            break;
        }
        let chunk = &remaining_pcm[chunk_range];
        let chunk_duration_ms =
            (chunk.len() as i64 * 1_000) / audio::decode::WHISPER_SAMPLE_RATE as i64;
        let raw_segments = transcriber
            .transcribe(chunk, interview.language.as_deref())
            .map_err(mark_error)?;
        let (new_segments, uncertain_flags) = if !raw_segments.is_empty() {
            // Diarization slices the current chunk, so it must receive the
            // chunk-local Whisper timestamps. Absolute interview timestamps
            // are applied only afterwards for persistence and display.
            let conn = lock_db(db)?;
            let (speaker_ids, uncertain_flags) = assigner
                .assign_chunk(&conn, interview_id, &raw_segments, chunk)
                .map_err(mark_error)?;
            let offset_segments: Vec<transcription::RawSegment> = raw_segments
                .into_iter()
                .map(|segment| transcription::RawSegment {
                    start_ms: segment.start_ms + offset_ms,
                    end_ms: segment.end_ms + offset_ms,
                    text: segment.text,
                    confidence: segment.confidence,
                })
                .collect();
            (
                transcription::to_new_segments(offset_segments, &speaker_ids),
                uncertain_flags,
            )
        } else {
            (Vec::new(), Vec::new())
        };
        let chunk_start_ms = offset_ms;
        offset_ms += chunk_duration_ms;
        {
            let conn = lock_db(db)?;
            db::segments::insert_transcription_chunk(
                &conn,
                interview_id,
                &new_segments,
                &uncertain_flags,
                offset_ms,
            )?;
        }
        let _ = app.emit("segments-updated", interview_id);
        let _ = app.emit(
            "chunk-progress",
            ChunkProgress {
                interview_id,
                chunk_index: index + 1,
                chunk_count,
                chunk_start_ms,
                chunk_end_ms: offset_ms,
                audio_duration_ms,
                estimated_remaining_ms: {
                    let processed_ms = (offset_ms - boundary_ms).max(1);
                    let remaining_ms = (audio_duration_ms - offset_ms).max(0);
                    let elapsed_ms = transcription_started.elapsed().as_millis() as i64;
                    elapsed_ms.saturating_mul(remaining_ms) / processed_ms
                },
            },
        );
        emit_task_progress(
            app,
            interview_id,
            "transcription",
            "whisper",
            "Transcription Whisper",
            18 + (((index + 1) * 62) / chunk_count.max(1)) as i32,
        );
    }
    if stopped_early {
        diagnostics::log("INFO", "transcription: arretee par l'utilisateur");
        let conn = lock_db(db)?;
        return db::get_detail(&conn, interview_id);
    }

    diagnostics::log("INFO", "transcription: inference Whisper terminee");
    emit_task_progress(
        app,
        interview_id,
        "transcription",
        "speakers",
        "Analyse des intervenants",
        82,
    );
    emit_task_progress(
        app,
        interview_id,
        "transcription",
        "save",
        "Enregistrement des segments",
        96,
    );
    let conn = lock_db(db)?;
    db::interviews::update_status(&conn, interview_id, "transcribed", None)?;
    diagnostics::log("INFO", "transcription: traitement termine");
    emit_task_progress(
        app,
        interview_id,
        "transcription",
        "done",
        "Transcription terminée",
        100,
    );
    db::get_detail(&conn, interview_id)
}

#[tauri::command]
fn stop_transcription(
    transcription: tauri::State<TranscriptionState>,
    interview_id: i64,
) -> Result<(), AppError> {
    diagnostics::log("INFO", "stop_transcription");
    let guard = transcription
        .0
        .lock()
        .map_err(|_| AppError::Transcription("etat de transcription indisponible".into()))?;
    match guard.as_ref() {
        Some(active) if active.interview_id == interview_id => {
            active
                .stop_flag
                .store(true, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }
        _ => Err(AppError::Transcription(
            "aucune transcription en cours pour cet entretien".into(),
        )),
    }
}

#[tauri::command]
fn list_resumable_posteriori(
    db: tauri::State<DbState>,
    transcription: tauri::State<TranscriptionState>,
) -> Result<Vec<db::models::Interview>, AppError> {
    let active_interview_id = transcription
        .0
        .lock()
        .map_err(|_| AppError::Transcription("etat de transcription indisponible".into()))?
        .as_ref()
        .map(|active| active.interview_id);
    let conn = lock_db(&db)?;
    db::interviews::list_resumable_posteriori(&conn, active_interview_id)
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
    model_id: Option<String>,
) -> Result<db::models::Interview, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        start_recording_local(&app, title, device_name, expected_speaker_count, model_id)
    })
    .await
    .map_err(|err| AppError::Audio(format!("demarrage interrompu: {err}")))?
}

fn start_recording_local(
    app: &tauri::AppHandle,
    title: String,
    device_name: Option<String>,
    expected_speaker_count: Option<usize>,
    model_id: Option<String>,
) -> Result<db::models::Interview, AppError> {
    diagnostics::log(
        "INFO",
        &format!(
            "start_recording device={} expected_speaker_count={:?}",
            device_name.as_deref().unwrap_or("defaut"),
            expected_speaker_count
        ),
    );
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
    diagnostics::log(
        "INFO",
        "verification des modeles avant ouverture du microphone",
    );
    let transcription::model::ModelStatus::Ready {
        path: model_path, ..
    } = transcription::model::ensure_manifest(
        app,
        transcription::model::selected_whisper_manifest(model_id.as_deref())?,
    )?;
    let transcription::model::ModelStatus::Ready {
        path: diarization_model_path,
        ..
    } = transcription::model::ensure_manifest(app, transcription::model::diarization_manifest()?)?;
    diagnostics::log("INFO", "modeles ok");

    let db = app.state::<DbState>();
    let audio_dir = app_data_subdir(app, "audio")?;
    let interview = {
        let conn = lock_db(&db)?;
        let interview = db::interviews::create_realtime(&conn, &title, "")?;
        let wav_path = audio_dir.join(format!("{}.wav", interview.id));
        db::interviews::set_audio_path(&conn, interview.id, &wav_path.to_string_lossy())?;
        db::interviews::get(&conn, interview.id)?
    };

    diagnostics::log("INFO", "ouverture du peripherique audio");
    let wav_path = Path::new(&interview.audio_path).to_path_buf();
    // Eight bounded 15-second chunks cap the live-transcription backlog at
    // roughly two minutes. The WAV keeps being written even if inference
    // falls behind, so multi-hour recordings cannot consume unbounded RAM.
    let (event_tx, event_rx) = std::sync::mpsc::sync_channel(32);
    let (chunk_tx, chunk_rx) = std::sync::mpsc::sync_channel(8);
    let handle =
        capture::session::start(device_name, wav_path, event_tx, chunk_tx).inspect_err(|err| {
            diagnostics::log("ERROR", &format!("echec d'ouverture du microphone: {err}"));
            if let Ok(conn) = lock_db(&db) {
                let _ = db::interviews::update_status(
                    &conn,
                    interview.id,
                    "error",
                    Some(&err.to_string()),
                );
            }
        })?;
    diagnostics::log("INFO", "microphone ouvert, capture demarree");

    let processing_app = app.clone();
    let interview_id = interview.id;
    let processing_thread = std::thread::spawn(move || {
        run_recording_processing(
            processing_app,
            interview_id,
            model_path,
            diarization_model_path,
            expected_speaker_count,
            chunk_rx,
        );
    });
    let telemetry_app = app.clone();
    let telemetry_thread = std::thread::spawn(move || {
        for event in event_rx {
            match event {
                capture::session::SessionEvent::LevelUpdate { rms } => {
                    let _ = telemetry_app.emit("recording-level", rms);
                }
                capture::session::SessionEvent::Error(message) => {
                    let _ = telemetry_app.emit("recording-error", message);
                }
            }
        }
    });

    let mut guard = recording
        .0
        .lock()
        .map_err(|_| AppError::Audio("etat d'enregistrement indisponible".into()))?;
    *guard = Some(ActiveRecording {
        handle,
        processing_thread,
        telemetry_thread,
        interview_id: interview.id,
    });

    Ok(interview)
}

#[tauri::command]
fn pause_recording(recording: tauri::State<RecordingState>) -> Result<(), AppError> {
    diagnostics::log("INFO", "pause_recording");
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
    diagnostics::log(
        "INFO",
        &format!(
            "resume_recording device={}",
            device_name.as_deref().unwrap_or("inchange")
        ),
    );
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
    diagnostics::log("INFO", "stop_recording");
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
        let lag_flag = active.handle.lag_flag();
        active.handle.stop();
        let _ = active.processing_thread.join();
        let _ = active.telemetry_thread.join();
        let db = app.state::<DbState>();
        let conn = lock_db(&db)?;
        if lag_flag.load(std::sync::atomic::Ordering::Relaxed) {
            db::interviews::update_status(
                &conn,
                active.interview_id,
                "transcribing",
                Some(
                    "La transcription en direct n'a pas suivi la capture. L'audio complet est conserve; utilisez Recuperer pour terminer.",
                ),
            )?;
        } else {
            db::interviews::update_status(&conn, active.interview_id, "transcribed", None)?;
        }
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
fn consume_recording_chunks<I, ProcessChunk, OnUpdated, OnError>(
    chunks: I,
    mut process_chunk: ProcessChunk,
    mut on_updated: OnUpdated,
    mut on_error: OnError,
) where
    I: IntoIterator<Item = Vec<f32>>,
    ProcessChunk: FnMut(i64, &[f32]) -> Result<(), AppError>,
    OnUpdated: FnMut(),
    OnError: FnMut(String),
{
    let mut elapsed_ms = 0i64;
    for pcm in chunks {
        if pcm.is_empty() {
            continue;
        }
        let chunk_duration_ms =
            (pcm.len() as i64 * 1000) / audio::decode::WHISPER_SAMPLE_RATE as i64;
        match process_chunk(elapsed_ms, &pcm) {
            Ok(()) => on_updated(),
            Err(error) => on_error(error.to_string()),
        }
        // An erroneous chunk still occupied time in the saved audio.
        // Advancing preserves timestamps for every later successful
        // chunk instead of collapsing them onto the failed interval.
        elapsed_ms += chunk_duration_ms;
    }
}

fn run_recording_processing(
    app: tauri::AppHandle,
    interview_id: i64,
    model_path: String,
    diarization_model_path: String,
    expected_speaker_count: Option<usize>,
    chunks: std::sync::mpsc::Receiver<Vec<f32>>,
) {
    let transcriber =
        match transcription::whisper_cpp::WhisperCppTranscriber::load(Path::new(&model_path)) {
            Ok(transcriber) => transcriber,
            Err(err) => return report_recording_error(&app, interview_id, err),
        };
    let mut assigner =
        match SpeakerAssigner::new(&diarization_model_path, expected_speaker_count, 0, false) {
            Ok(assigner) => assigner,
            Err(err) => return report_recording_error(&app, interview_id, err),
        };
    consume_recording_chunks(
        chunks,
        |elapsed_ms, pcm| {
            process_recording_chunk(
                &app,
                interview_id,
                &transcriber,
                &mut assigner,
                elapsed_ms,
                pcm,
            )
        },
        || {
            let _ = app.emit("segments-updated", interview_id);
        },
        |message| {
            let _ = app.emit("recording-error", message);
        },
    );
}

fn process_recording_chunk(
    app: &tauri::AppHandle,
    interview_id: i64,
    transcriber: &transcription::whisper_cpp::WhisperCppTranscriber,
    assigner: &mut SpeakerAssigner,
    time_offset_ms: i64,
    pcm: &[f32],
) -> Result<(), AppError> {
    diagnostics::log("INFO", "enregistrement: transcription d'un bloc audio");
    let raw_segments = transcriber.transcribe(pcm, None)?;
    if raw_segments.is_empty() {
        return Ok(());
    }

    let db = app.state::<DbState>();
    let conn = lock_db(&db)?;
    let (speaker_ids, uncertain_flags) =
        assigner.assign_chunk(&conn, interview_id, &raw_segments, pcm)?;

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
    diagnostics::log("INFO", "enregistrement: bloc audio traite");
    Ok(())
}

/// Owns whatever state speaker assignment needs across an entire recording
/// session. On desktop, one persistent diarization `Clusterer` so speaker
/// identity stays stable chunk to chunk; on Android, just the single speaker
/// every segment is attributed to, since diarization isn't available there
/// (see docs/ARCHITECTURE.md "Diarisation") - same fallback `assign_speakers`
/// uses for the a-posteriori pipeline, kept alive across chunks instead of
/// recreated once per file.
enum SpeakerAssigner {
    #[cfg(not(target_os = "android"))]
    Diarizing {
        extractor: diarization::EmbeddingExtractor,
        clusterer: diarization::Clusterer,
        speakers: Vec<db::models::Speaker>,
        first_speaker_index: usize,
        force_uncertain: bool,
    },
    #[cfg(target_os = "android")]
    SingleSpeaker {
        speaker: Option<db::models::Speaker>,
        speaker_index: usize,
        force_uncertain: bool,
    },
}

impl SpeakerAssigner {
    #[cfg(not(target_os = "android"))]
    fn new(
        diarization_model_path: &str,
        expected_speaker_count: Option<usize>,
        existing_speaker_count: usize,
        force_uncertain: bool,
    ) -> Result<Self, AppError> {
        let extractor = diarization::EmbeddingExtractor::load(Path::new(diarization_model_path))?;
        Ok(Self::Diarizing {
            extractor,
            clusterer: diarization::Clusterer::new(expected_speaker_count),
            speakers: Vec::new(),
            first_speaker_index: existing_speaker_count + 1,
            force_uncertain,
        })
    }

    #[cfg(target_os = "android")]
    fn new(
        _diarization_model_path: &str,
        _expected_speaker_count: Option<usize>,
        existing_speaker_count: usize,
        force_uncertain: bool,
    ) -> Result<Self, AppError> {
        Ok(Self::SingleSpeaker {
            speaker: None,
            speaker_index: existing_speaker_count + 1,
            force_uncertain,
        })
    }

    fn assign_chunk(
        &mut self,
        conn: &rusqlite::Connection,
        interview_id: i64,
        raw_segments: &[transcription::RawSegment],
        pcm: &[f32],
    ) -> Result<(Vec<i64>, Vec<bool>), AppError> {
        match self {
            #[cfg(not(target_os = "android"))]
            Self::Diarizing {
                extractor,
                clusterer,
                speakers,
                first_speaker_index,
                force_uncertain,
            } => {
                let assignments = raw_segments
                    .iter()
                    .map(|segment| {
                        let slice =
                            diarization::slice_pcm_ms(pcm, segment.start_ms, segment.end_ms);
                        let embedding = extractor.extract(slice)?;
                        Ok(clusterer.assign(&embedding))
                    })
                    .collect::<Result<Vec<diarization::Assignment>, AppError>>()?;
                while speakers.len() < clusterer.speaker_count() {
                    let index = *first_speaker_index + speakers.len();
                    speakers.push(db::speakers::create_numbered(conn, interview_id, index)?);
                }
                let speaker_ids = assignments
                    .iter()
                    .map(|assignment| speakers[assignment.speaker_index].id)
                    .collect();
                let uncertain_flags = assignments
                    .iter()
                    .map(|assignment| *force_uncertain || assignment.uncertain)
                    .collect();
                Ok((speaker_ids, uncertain_flags))
            }
            #[cfg(target_os = "android")]
            Self::SingleSpeaker {
                speaker,
                speaker_index,
                force_uncertain,
            } => {
                if speaker.is_none() {
                    *speaker = Some(db::speakers::create_numbered(
                        conn,
                        interview_id,
                        *speaker_index,
                    )?);
                }
                let id = speaker.as_ref().expect("just set above").id;
                let speaker_ids = vec![id; raw_segments.len()];
                let uncertain_flags = vec![*force_uncertain; raw_segments.len()];
                Ok((speaker_ids, uncertain_flags))
            }
        }
    }
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

/// Returns the tail of the local diagnostics log for display in Reglages -
/// see docs/ARCHITECTURE.md "Diagnostics locaux". Never audio/transcript
/// content, only technical events and error messages.
#[tauri::command]
fn read_recent_logs() -> String {
    diagnostics::read_tail(500)
}

/// Lets the frontend append to the same diagnostics log, so a JS-side
/// failure (including one that happens before any backend command is even
/// reached) still leaves a trace the user can hand over.
#[tauri::command]
fn client_log(level: String, message: String) {
    diagnostics::log(&level, &format!("[frontend] {message}"));
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
    // E2E test builds only (`--features wdio-e2e`) - never present in a real
    // build, see the dependency comment in Cargo.toml.
    #[cfg(feature = "wdio-e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio_webdriver::init())
        .plugin(tauri_plugin_wdio::init());
    builder
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            diagnostics::init(app.handle())?;
            let base = app.path().app_data_dir()?;
            std::fs::create_dir_all(&base)?;
            let conn = db::open(&base.join("interviewscribe.sqlite3"))?;
            app.manage(DbState(Mutex::new(conn)));
            app.manage(RecordingState(Mutex::new(None)));
            app.manage(TranscriptionState(Mutex::new(None)));
            // Lets the frontend play back an interview's own audio via
            // convertFileSrc() (re-listen feature) without widening the
            // asset protocol to the whole filesystem - only this app's
            // private audio directory is ever reachable through it.
            let audio_dir = app_data_subdir(app.handle(), "audio")?;
            app.asset_protocol_scope()
                .allow_directory(&audio_dir, false)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            application_status,
            import_interview,
            list_interviews,
            list_recent_segments,
            list_recovery_candidates,
            inspect_recovery_candidate,
            keep_interrupted_as_is,
            recover_interview,
            get_interview,
            update_interview_notes,
            delete_interview,
            ensure_whisper_model,
            list_available_models,
            transcribe_interview,
            stop_transcription,
            list_resumable_posteriori,
            keep_posteriori_as_is,
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
            check_doc_export_available,
            read_recent_logs,
            client_log
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

    #[test]
    fn failed_chunk_keeps_prior_audio_and_processing_continues_at_the_right_offset() {
        let audio_path = std::env::temp_dir().join(format!(
            "interviewscribe-chunk-failure-{}.wav",
            std::process::id()
        ));
        let audio_evidence = b"saved audio evidence";
        std::fs::write(&audio_path, audio_evidence).unwrap();
        let chunks = [1.0f32, 2.0, 3.0]
            .map(|marker| vec![marker; audio::decode::WHISPER_SAMPLE_RATE as usize]);
        let mut persisted_offsets = Vec::new();
        let mut errors = Vec::new();
        let mut updates = 0;

        consume_recording_chunks(
            chunks,
            |offset, pcm| {
                if pcm[0] == 2.0 {
                    Err(AppError::Transcription("segment test invalide".into()))
                } else {
                    persisted_offsets.push(offset);
                    Ok(())
                }
            },
            || updates += 1,
            |message| errors.push(message),
        );

        assert_eq!(persisted_offsets, vec![0, 2_000]);
        assert_eq!(updates, 2);
        assert_eq!(errors.len(), 1);
        assert_eq!(std::fs::read(&audio_path).unwrap(), audio_evidence);
        std::fs::remove_file(audio_path).ok();
    }
}
