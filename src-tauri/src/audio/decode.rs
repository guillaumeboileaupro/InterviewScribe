use std::fs::File;
use std::path::Path;

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler, WindowFunction};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::error::AppError;

/// The sample rate whisper.cpp expects.
pub const WHISPER_SAMPLE_RATE: u32 = 16_000;

/// Decodes any symphonia-supported audio (or the audio track of a video) file
/// to mono f32 PCM at 16kHz, the format whisper.cpp expects.
pub fn decode_to_mono_pcm16k(path: &Path) -> Result<Vec<f32>, AppError> {
    let (mono_samples, source_rate) = decode_to_mono(path)?;
    if source_rate == WHISPER_SAMPLE_RATE {
        return Ok(mono_samples);
    }
    resample(&mono_samples, source_rate, WHISPER_SAMPLE_RATE)
}

fn decode_to_mono(path: &Path) -> Result<(Vec<f32>, u32), AppError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        hint.with_extension(ext);
    }

    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|err| AppError::Audio(format!("format non reconnu: {err}")))?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| AppError::Audio("aucune piste audio trouvee".into()))?;
    let track_id = track.id;

    let audio_params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .ok_or_else(|| AppError::Audio("codec audio non reconnu".into()))?;
    let source_rate = audio_params
        .sample_rate
        .ok_or_else(|| AppError::Audio("frequence d'echantillonnage inconnue".into()))?;

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(audio_params, &AudioDecoderOptions::default())
        .map_err(|err| AppError::Audio(format!("codec non supporte: {err}")))?;

    let mut mono_samples = Vec::new();
    let mut interleaved = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(SymphoniaError::ResetRequired) => break,
            Err(err) => return Err(AppError::Audio(format!("erreur de lecture: {err}"))),
        };
        if packet.track_id != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                let channels = audio_buf.spec().channels().count().max(1);
                interleaved.resize(audio_buf.samples_interleaved(), 0f32);
                audio_buf.copy_to_slice_interleaved(&mut interleaved);
                for frame in interleaved.chunks(channels) {
                    let sum: f32 = frame.iter().sum();
                    mono_samples.push(sum / channels as f32);
                }
            }
            // A single bad packet must not lose the whole recording.
            Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::DecodeError(_)) => continue,
            Err(err) => return Err(AppError::Audio(format!("erreur de decodage: {err}"))),
        }
    }

    if mono_samples.is_empty() {
        return Err(AppError::Audio("aucun echantillon audio decode".into()));
    }

    Ok((mono_samples, source_rate))
}

/// Shared with `capture::session`, which resamples a finished mic chunk to
/// 16kHz the same way a decoded file is.
pub(crate) fn resample(samples: &[f32], rate_in: u32, rate_out: u32) -> Result<Vec<f32>, AppError> {
    let input = InterleavedSlice::new(samples, 1, samples.len())
        .map_err(|err| AppError::Audio(format!("tampon audio invalide: {err}")))?;
    let mut resampler = Fft::<f32>::new_custom(
        rate_in as usize,
        rate_out as usize,
        1024,
        1,
        1,
        WindowFunction::BlackmanHarris2,
        FixedSync::Both,
    )
    .map_err(|err| AppError::Audio(format!("initialisation du resampler: {err}")))?;
    let output = resampler
        .process_all(&input, samples.len(), None)
        .map_err(|err| AppError::Audio(format!("erreur de resampling: {err}")))?;
    Ok(output.take_data())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_test_wav(path: &Path, sample_rate: u32, channels: u16, samples: &[i16]) {
        let mut file = std::fs::File::create(path).unwrap();
        let data_len = (samples.len() * 2) as u32;
        let byte_rate = sample_rate * channels as u32 * 2;
        let block_align = channels * 2;

        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
        file.write_all(b"WAVE").unwrap();
        file.write_all(b"fmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
        file.write_all(&channels.to_le_bytes()).unwrap();
        file.write_all(&sample_rate.to_le_bytes()).unwrap();
        file.write_all(&byte_rate.to_le_bytes()).unwrap();
        file.write_all(&block_align.to_le_bytes()).unwrap();
        file.write_all(&16u16.to_le_bytes()).unwrap(); // bits per sample
        file.write_all(b"data").unwrap();
        file.write_all(&data_len.to_le_bytes()).unwrap();
        for sample in samples {
            file.write_all(&sample.to_le_bytes()).unwrap();
        }
    }

    fn sine_wave(len: usize, freq_hz: f32, sample_rate: u32) -> Vec<i16> {
        (0..len)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                ((t * freq_hz * std::f32::consts::TAU).sin() * i16::MAX as f32 * 0.5) as i16
            })
            .collect()
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "interviewscribe-decode-test-{}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn decodes_mono_wav_already_at_target_rate() {
        let path = temp_path("mono16k.wav");
        let samples = sine_wave(WHISPER_SAMPLE_RATE as usize, 440.0, WHISPER_SAMPLE_RATE);
        write_test_wav(&path, WHISPER_SAMPLE_RATE, 1, &samples);

        let pcm = decode_to_mono_pcm16k(&path).unwrap();
        assert_eq!(pcm.len(), samples.len());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn downmixes_stereo_and_resamples() {
        let path = temp_path("stereo44100.wav");
        let source_rate = 44_100;
        let mono = sine_wave(source_rate as usize, 440.0, source_rate);
        let mut stereo = Vec::with_capacity(mono.len() * 2);
        for sample in &mono {
            stereo.push(*sample);
            stereo.push(*sample);
        }
        write_test_wav(&path, source_rate, 2, &stereo);

        let pcm = decode_to_mono_pcm16k(&path).unwrap();
        let expected_len =
            (mono.len() as f64 * WHISPER_SAMPLE_RATE as f64 / source_rate as f64).round() as usize;
        assert!(
            pcm.len().abs_diff(expected_len) < WHISPER_SAMPLE_RATE as usize / 10,
            "resampled length {} too far from expected {}",
            pcm.len(),
            expected_len
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn corrupt_file_is_an_error_not_a_panic() {
        let path = temp_path("corrupt.wav");
        std::fs::write(&path, b"not a real wav file at all").unwrap();

        let result = decode_to_mono_pcm16k(&path);
        assert!(result.is_err());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn decodes_every_supported_fixture_format_to_mono_16khz() {
        let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/audio");
        for extension in ["wav", "mp3", "m4a", "flac", "ogg", "aac"] {
            let path = fixture_dir.join(format!("tone.{extension}"));
            let pcm = decode_to_mono_pcm16k(&path)
                .unwrap_or_else(|error| panic!("failed to decode {}: {error}", path.display()));
            assert!(
                (3_500..=5_500).contains(&pcm.len()),
                "unexpected decoded length for {extension}: {}",
                pcm.len()
            );
            let peak = pcm.iter().copied().map(f32::abs).fold(0.0, f32::max);
            assert!(peak > 0.01, "decoded {extension} fixture is silent");
        }
    }

    #[test]
    fn normalizes_sample_rate_channel_count_and_duration_extremes() {
        for (name, source_rate, channels, duration_ms) in [
            ("short-mono-8k.wav", 8_000, 1, 10),
            ("mono-16k.wav", 16_000, 1, 250),
            ("stereo-44100.wav", 44_100, 2, 250),
            ("long-stereo-48k.wav", 48_000, 2, 60_000),
        ] {
            let path = temp_path(name);
            let frame_count = source_rate as usize * duration_ms as usize / 1_000;
            let mono = sine_wave(frame_count, 440.0, source_rate);
            let samples = if channels == 1 {
                mono
            } else {
                mono.into_iter()
                    .flat_map(|sample| [sample, sample])
                    .collect()
            };
            write_test_wav(&path, source_rate, channels, &samples);

            let pcm = decode_to_mono_pcm16k(&path).unwrap_or_else(|error| {
                panic!("failed matrix case {name}: {error}");
            });
            let expected = WHISPER_SAMPLE_RATE as usize * duration_ms as usize / 1_000;
            let tolerance = (WHISPER_SAMPLE_RATE as usize / 100).max(1);
            assert!(
                pcm.len().abs_diff(expected) <= tolerance,
                "{name}: decoded {} samples, expected about {expected}",
                pcm.len()
            );
            std::fs::remove_file(&path).ok();
        }
    }

    #[test]
    fn empty_file_is_reported_as_an_audio_error() {
        let path = temp_path("empty.wav");
        std::fs::write(&path, []).unwrap();
        let result = decode_to_mono_pcm16k(&path);
        std::fs::remove_file(&path).ok();
        assert!(result.is_err());
    }

    #[test]
    fn truncated_wav_header_is_reported_as_an_audio_error() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/audio/tone.wav");
        let bytes = std::fs::read(fixture).unwrap();
        let path = temp_path("truncated.wav");
        std::fs::write(&path, &bytes[..40]).unwrap();
        let result = decode_to_mono_pcm16k(&path);
        std::fs::remove_file(&path).ok();
        assert!(result.is_err());
    }

    #[test]
    fn content_probe_decodes_a_wav_with_a_misleading_mp3_extension() {
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/audio/tone.wav");
        let path = temp_path("actually-wav.mp3");
        std::fs::copy(fixture, &path).unwrap();
        let pcm = decode_to_mono_pcm16k(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert!(!pcm.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn permission_denied_is_propagated_without_a_panic() {
        use std::os::unix::fs::PermissionsExt;

        let path = temp_path("permission-denied.wav");
        std::fs::write(&path, b"audio remains unread").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        let result = decode_to_mono_pcm16k(&path);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::remove_file(&path).ok();
        assert!(result.is_err());
    }
}
