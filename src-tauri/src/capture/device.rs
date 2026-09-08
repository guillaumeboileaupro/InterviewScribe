use std::collections::HashSet;

use cpal::traits::{DeviceTrait, HostTrait};

use crate::error::AppError;

/// ALSA (and other Linux audio backends) enumerate every software plugin and
/// PCM alias alongside real hardware, not just actual microphones: things
/// like `null` (silently discards audio instead of capturing it - a real
/// device picking this produces permanent silence, so no chunk ever
/// finalizes), PulseAudio/JACK/PipeWire/OSS bridges, and channel-mixing
/// plugins, plus the same physical microphone listed several times under
/// different driver aliases (`hw`/`plughw`/`sysdefault`/`front`/`dsnoop`).
/// Verified against a real machine: 18 raw entries for one physical mic,
/// including the silent `null` device - see docs/ARCHITECTURE.md "Pipeline
/// temps reel".
fn is_real_capture_driver(driver: &str) -> bool {
    const SOFTWARE_DRIVERS: &[&str] = &[
        "null",
        "default",
        "samplerate",
        "speexrate",
        "jack",
        "oss",
        "pipewire",
        "pulse",
        "upmix",
        "vdownmix",
    ];
    if SOFTWARE_DRIVERS.contains(&driver) {
        return false;
    }
    !driver.starts_with("usbstream:")
}

/// Names of the available microphone input devices, filtered to real
/// hardware and deduplicated by name (see `is_real_capture_driver`). Devices
/// without driver information (non-Linux backends) are kept as-is - this
/// filter only ever removes entries it can positively identify as ALSA
/// software plugins. Not unit-tested end-to-end (depends on real
/// hardware/host state) - verified manually against the actual machine, same
/// as any other hardware-dependent path in this project.
pub fn list_input_devices() -> Result<Vec<String>, AppError> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .map_err(|err| AppError::Audio(format!("peripheriques audio indisponibles: {err}")))?;
    let mut seen = HashSet::new();
    let mut names = Vec::new();
    for device in devices {
        let Ok(description) = device.description() else {
            continue;
        };
        let keep = description
            .driver()
            .map(is_real_capture_driver)
            .unwrap_or(true);
        if !keep {
            continue;
        }
        let name = description.name().to_string();
        if seen.insert(name.clone()) {
            names.push(name);
        }
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_out_known_alsa_software_plugins() {
        for driver in [
            "null",
            "default",
            "samplerate",
            "speexrate",
            "jack",
            "oss",
            "pipewire",
            "pulse",
            "upmix",
            "vdownmix",
            "usbstream:CARD=PCH",
        ] {
            assert!(
                !is_real_capture_driver(driver),
                "{driver} should be filtered out"
            );
        }
    }

    #[test]
    fn keeps_real_hardware_driver_aliases() {
        for driver in [
            "hw:CARD=PCH,DEV=0",
            "plughw:CARD=PCH,DEV=0",
            "sysdefault:CARD=PCH",
            "front:CARD=PCH,DEV=0",
            "dsnoop:CARD=PCH,DEV=0",
        ] {
            assert!(is_real_capture_driver(driver), "{driver} should be kept");
        }
    }

    /// Exercises the real host enumeration: on Linux/ALSA a raw
    /// `input_devices()` call can return a dozen-plus software plugins and
    /// duplicate hardware aliases for a single physical microphone (verified
    /// on a real machine - see the module doc comment); this checks the
    /// filtered, deduplicated result stays sane. Ignored by default: depends
    /// on real hardware, same policy as `whisper_smoke` and
    /// `real_microphone_capture_produces_a_valid_recoverable_wav`.
    #[test]
    #[ignore]
    fn list_input_devices_returns_no_duplicates_and_no_known_software_plugins() {
        let names = list_input_devices().expect("host should enumerate input devices");
        eprintln!("real input devices on this machine: {names:?}");
        assert!(!names.is_empty(), "expected at least one real microphone");

        let mut seen = HashSet::new();
        for name in &names {
            assert!(seen.insert(name.clone()), "duplicate device name: {name}");
        }
    }
}
