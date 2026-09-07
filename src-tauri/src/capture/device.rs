use cpal::traits::{DeviceTrait, HostTrait};

use crate::error::AppError;

/// Names of the available microphone input devices. Not unit-tested (depends
/// on real hardware/host state) - verified manually against the actual
/// machine, same as any other hardware-dependent path in this project.
pub fn list_input_devices() -> Result<Vec<String>, AppError> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .map_err(|err| AppError::Audio(format!("peripheriques audio indisponibles: {err}")))?;
    let mut names = Vec::new();
    for device in devices {
        if let Ok(description) = device.description() {
            names.push(description.name().to_string());
        }
    }
    Ok(names)
}
