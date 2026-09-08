use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::audio::decode::{resample, WHISPER_SAMPLE_RATE};
use crate::capture::chunker::{ChunkEvent, Chunker};
use crate::capture::vad::{rms, Vad, VadConfig};
use crate::error::AppError;

const MAX_CHUNK_DURATION_MS: u32 = 30_000;
const SILENCE_TO_CLOSE_MS: u32 = 800;
const LEVEL_UPDATE_INTERVAL_MS: u32 = 100;

/// Events pushed out of the capture thread. The caller (`lib.rs`) owns
/// transcription/diarization/persistence - this module only knows about
/// capturing, leveling and chunking audio, never about Whisper or the
/// database.
pub enum SessionEvent {
    LevelUpdate {
        rms: f32,
    },
    /// Always 16kHz mono f32, ready for `Transcriber::transcribe` directly.
    ChunkReady {
        pcm: Vec<f32>,
    },
    Error(String),
}

enum ControlMsg {
    Pause,
    Resume(Option<String>),
    Stop,
}

/// `cpal::Stream` is not `Send`, so it can only ever live on the dedicated
/// thread this handle controls - never move it out. Every method here just
/// sends a message to that thread.
pub struct RecordingHandle {
    control: mpsc::Sender<ControlMsg>,
}

impl RecordingHandle {
    pub fn pause(&self) {
        let _ = self.control.send(ControlMsg::Pause);
    }

    pub fn resume(&self, device_name: Option<String>) {
        let _ = self.control.send(ControlMsg::Resume(device_name));
    }

    pub fn stop(self) {
        let _ = self.control.send(ControlMsg::Stop);
    }
}

/// Starts capturing from `device_name` (or the host default if `None`),
/// writing incrementally to `wav_path` (crash-recoverable: read back through
/// the existing a-posteriori pipeline if the app never gets to call `stop`)
/// and pushing `SessionEvent`s to `events` as they happen. Blocks briefly
/// until the capture thread confirms the device actually opened, so a bad
/// device name surfaces as a normal `Err` here rather than only later as an
/// event.
pub fn start(
    device_name: Option<String>,
    wav_path: PathBuf,
    events: mpsc::Sender<SessionEvent>,
) -> Result<RecordingHandle, AppError> {
    let (control_tx, control_rx) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::channel();

    std::thread::spawn(move || {
        run_capture_thread(device_name, wav_path, events, control_rx, ready_tx)
    });

    match ready_rx.recv() {
        Ok(Ok(())) => Ok(RecordingHandle {
            control: control_tx,
        }),
        Ok(Err(message)) => Err(AppError::Audio(message)),
        Err(_) => Err(AppError::Audio(
            "le thread de capture s'est arrete avant d'etre pret".into(),
        )),
    }
}

fn wav_spec(sample_rate: u32) -> hound::WavSpec {
    hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    }
}

fn resolve_device(host: &cpal::Host, name: Option<&str>) -> Result<cpal::Device, String> {
    match name {
        Some(name) => host
            .input_devices()
            .map_err(|err| err.to_string())?
            .find(|device| {
                device
                    .description()
                    .map(|description| description.name() == name)
                    .unwrap_or(false)
            })
            .ok_or_else(|| format!("peripherique introuvable: {name}")),
        None => host
            .default_input_device()
            .ok_or_else(|| "aucun peripherique d'entree audio disponible".to_string()),
    }
}

type SharedWriter = Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>;
type SharedFrameState = Arc<Mutex<FrameState>>;

struct FrameState {
    vad: Vad,
    chunker: Chunker,
    last_level_emit: std::time::Instant,
}

impl FrameState {
    fn new() -> Self {
        Self {
            vad: Vad::new(VadConfig::default()),
            chunker: Chunker::new(MAX_CHUNK_DURATION_MS, SILENCE_TO_CLOSE_MS),
            last_level_emit: std::time::Instant::now(),
        }
    }
}

/// Flushes whatever partial chunk is still buffered (e.g. on stop, so the
/// last utterance before the user hits "stop" is never silently dropped -
/// see docs/PRODUCT.md "sauvegarde incrementale pendant l'enregistrement").
fn flush_pending_chunk(
    state: &SharedFrameState,
    sample_rate: u32,
    events: &mpsc::Sender<SessionEvent>,
) {
    if let Ok(mut guard) = state.lock() {
        if let ChunkEvent::Ready(chunk) = guard.chunker.flush() {
            emit_ready_chunk(chunk, sample_rate, events);
        }
    }
}

fn build_stream(
    device: &cpal::Device,
    writer: SharedWriter,
    events: mpsc::Sender<SessionEvent>,
) -> Result<(cpal::Stream, SharedFrameState, u32), String> {
    let config = device
        .default_input_config()
        .map_err(|err| err.to_string())?;
    let sample_rate = config.sample_rate();
    let channels = config.channels() as usize;
    let stream_config = config.config();

    let state: SharedFrameState = Arc::new(Mutex::new(FrameState::new()));

    let error_events = events.clone();
    let err_fn = move |err: cpal::Error| {
        let _ = error_events.send(SessionEvent::Error(format!("erreur audio: {err}")));
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let writer = writer.clone();
            let events = events.clone();
            let state = state.clone();
            device
                .build_input_stream(
                    stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        process_input(data, channels, sample_rate, &state, &writer, &events);
                    },
                    err_fn,
                    None,
                )
                .map_err(|err| err.to_string())?
        }
        cpal::SampleFormat::I16 => {
            let writer = writer.clone();
            let events = events.clone();
            let state = state.clone();
            device
                .build_input_stream(
                    stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let converted: Vec<f32> =
                            data.iter().map(|s| *s as f32 / i16::MAX as f32).collect();
                        process_input(&converted, channels, sample_rate, &state, &writer, &events);
                    },
                    err_fn,
                    None,
                )
                .map_err(|err| err.to_string())?
        }
        other => return Err(format!("format audio non supporte: {other:?}")),
    };

    Ok((stream, state, sample_rate))
}

fn process_input(
    data: &[f32],
    channels: usize,
    sample_rate: u32,
    state: &SharedFrameState,
    writer: &SharedWriter,
    events: &mpsc::Sender<SessionEvent>,
) {
    let mono: Vec<f32> = if channels <= 1 {
        data.to_vec()
    } else {
        data.chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    };

    if let Ok(mut guard) = writer.lock() {
        if let Some(writer) = guard.as_mut() {
            for sample in &mono {
                let _ = writer.write_sample(*sample);
            }
        }
    }

    let energy = rms(&mono);
    let frame_ms = (((mono.len() as f64 / sample_rate as f64) * 1000.0).round() as u32).max(1);

    let Ok(mut guard) = state.lock() else {
        return;
    };
    let speech_active = guard.vad.process_frame(energy, frame_ms);

    if guard.last_level_emit.elapsed().as_millis() as u32 >= LEVEL_UPDATE_INTERVAL_MS {
        let _ = events.send(SessionEvent::LevelUpdate { rms: energy });
        guard.last_level_emit = std::time::Instant::now();
    }

    if let ChunkEvent::Ready(chunk) = guard.chunker.push_frame(&mono, speech_active, frame_ms) {
        drop(guard);
        emit_ready_chunk(chunk, sample_rate, events);
    }
}

fn emit_ready_chunk(chunk: Vec<f32>, sample_rate: u32, events: &mpsc::Sender<SessionEvent>) {
    let resampled = if sample_rate == WHISPER_SAMPLE_RATE {
        Ok(chunk)
    } else {
        resample(&chunk, sample_rate, WHISPER_SAMPLE_RATE)
    };
    match resampled {
        Ok(pcm) => {
            let _ = events.send(SessionEvent::ChunkReady { pcm });
        }
        Err(err) => {
            let _ = events.send(SessionEvent::Error(err.to_string()));
        }
    }
}

fn run_capture_thread(
    device_name: Option<String>,
    wav_path: PathBuf,
    events: mpsc::Sender<SessionEvent>,
    control_rx: mpsc::Receiver<ControlMsg>,
    ready_tx: mpsc::Sender<Result<(), String>>,
) {
    let host = cpal::default_host();
    crate::diagnostics::log(
        "INFO",
        &format!(
            "resolution du peripherique: {}",
            device_name.as_deref().unwrap_or("defaut")
        ),
    );
    let device = match resolve_device(&host, device_name.as_deref()) {
        Ok(device) => device,
        Err(message) => {
            crate::diagnostics::log("ERROR", &format!("peripherique introuvable: {message}"));
            let _ = ready_tx.send(Err(message));
            return;
        }
    };

    let sample_rate = match device.default_input_config() {
        Ok(config) => config.sample_rate(),
        Err(err) => {
            crate::diagnostics::log(
                "ERROR",
                &format!("configuration d'entree audio indisponible: {err}"),
            );
            let _ = ready_tx.send(Err(err.to_string()));
            return;
        }
    };

    let file_writer = match hound::WavWriter::create(&wav_path, wav_spec(sample_rate)) {
        Ok(writer) => writer,
        Err(err) => {
            crate::diagnostics::log(
                "ERROR",
                &format!("impossible de creer le fichier audio: {err}"),
            );
            let _ = ready_tx.send(Err(format!("impossible de creer le fichier audio: {err}")));
            return;
        }
    };
    let writer: SharedWriter = Arc::new(Mutex::new(Some(file_writer)));

    let (mut stream, mut frame_state, mut stream_rate) =
        match build_stream(&device, writer.clone(), events.clone()) {
            Ok(built) => built,
            Err(message) => {
                crate::diagnostics::log(
                    "ERROR",
                    &format!("construction du flux audio echouee: {message}"),
                );
                let _ = ready_tx.send(Err(message));
                return;
            }
        };
    if let Err(err) = stream.play() {
        crate::diagnostics::log(
            "ERROR",
            &format!("impossible de demarrer la capture: {err}"),
        );
        let _ = ready_tx.send(Err(format!("impossible de demarrer la capture: {err}")));
        return;
    }
    crate::diagnostics::log("INFO", "flux audio en lecture");
    let _ = ready_tx.send(Ok(()));

    let mut current_device_name = device_name;
    for msg in control_rx {
        match msg {
            ControlMsg::Pause => {
                crate::diagnostics::log("INFO", "capture: pause");
                let _ = stream.pause();
            }
            ControlMsg::Resume(requested_device) => {
                crate::diagnostics::log("INFO", "capture: reprise");
                let changing_device =
                    requested_device.is_some() && requested_device != current_device_name;
                if changing_device {
                    match resolve_device(&host, requested_device.as_deref())
                        .and_then(|device| build_stream(&device, writer.clone(), events.clone()))
                    {
                        Ok((new_stream, new_state, new_rate)) => {
                            // The previous device's still-accumulating chunk
                            // is flushed rather than silently discarded.
                            flush_pending_chunk(&frame_state, stream_rate, &events);
                            stream = new_stream;
                            frame_state = new_state;
                            stream_rate = new_rate;
                            current_device_name = requested_device;
                            if let Err(err) = stream.play() {
                                let _ = events.send(SessionEvent::Error(format!(
                                    "impossible de reprendre sur ce peripherique: {err}"
                                )));
                            }
                        }
                        Err(message) => {
                            let _ = events.send(SessionEvent::Error(message));
                        }
                    }
                } else if let Err(err) = stream.play() {
                    let _ = events.send(SessionEvent::Error(format!(
                        "impossible de reprendre la capture: {err}"
                    )));
                }
            }
            ControlMsg::Stop => {
                crate::diagnostics::log("INFO", "capture: arret");
                break;
            }
        }
    }

    flush_pending_chunk(&frame_state, stream_rate, &events);
    drop(stream);
    if let Ok(mut guard) = writer.lock() {
        if let Some(writer) = guard.take() {
            let _ = writer.finalize();
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercises the real default input device: opens it, captures for a few
    /// seconds, pauses, resumes, then stops - verifying at least one level
    /// update arrives and the resulting WAV is valid and non-trivial. This is
    /// the one thing the pure `vad`/`chunker` unit tests structurally cannot
    /// cover (real cpal/ALSA device access) - see docs/ARCHITECTURE.md
    /// "Pipeline temps reel". Ignored by default: requires real hardware and
    /// must never run unattended in CI, same policy as `whisper_smoke`.
    #[test]
    #[ignore]
    fn real_microphone_capture_produces_a_valid_recoverable_wav() {
        let wav_path = std::env::temp_dir().join(format!(
            "interviewscribe-capture-smoke-{}.wav",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&wav_path);

        let (events_tx, events_rx) = mpsc::channel();
        let handle = start(None, wav_path.clone(), events_tx)
            .expect("no default input device available on this machine");

        std::thread::sleep(std::time::Duration::from_secs(2));
        handle.pause();
        std::thread::sleep(std::time::Duration::from_millis(200));
        handle.resume(None);
        std::thread::sleep(std::time::Duration::from_secs(2));
        handle.stop();

        let mut saw_level_update = false;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            match events_rx.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(SessionEvent::LevelUpdate { .. }) => saw_level_update = true,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(
            saw_level_update,
            "expected at least one LevelUpdate from the real capture callback"
        );

        let pcm = crate::audio::decode::decode_to_mono_pcm16k(&wav_path)
            .expect("the incrementally-written WAV must be a valid, readable file");
        assert!(
            !pcm.is_empty(),
            "expected the ~4s recording to contain samples"
        );

        std::fs::remove_file(&wav_path).ok();
    }
}
