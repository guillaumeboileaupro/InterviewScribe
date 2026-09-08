use std::io::Read;
#[cfg(any(target_os = "android", test))]
use std::io::Write;
use std::path::Path;
#[cfg(any(target_os = "android", test))]
use std::path::PathBuf;
#[cfg(any(target_os = "android", test))]
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Manager;

use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct ModelManifest {
    pub name: String,
    pub file: String,
    pub size_bytes: u64,
    pub sha256: String,
}

pub fn manifest() -> Result<ModelManifest, AppError> {
    serde_json::from_str(include_str!("../../resources/models/manifest.json"))
        .map_err(|err| AppError::Model(format!("description du modele invalide: {err}")))
}

fn small_manifest() -> Result<ModelManifest, AppError> {
    serde_json::from_str(include_str!(
        "../../resources/models/whisper-small-manifest.json"
    ))
    .map_err(|err| AppError::Model(format!("description du modele invalide: {err}")))
}

fn base_manifest() -> Result<ModelManifest, AppError> {
    serde_json::from_str(include_str!(
        "../../resources/models/whisper-base-manifest.json"
    ))
    .map_err(|err| AppError::Model(format!("description du modele invalide: {err}")))
}

/// One entry per Whisper transcription model the user can pick between (see
/// docs/PRODUCT.md): id is stable across releases and is what the frontend
/// sends back to select a model - never the display name, which can change.
const WHISPER_MODEL_IDS: [&str; 3] = ["large-v3-turbo", "small", "base"];

fn whisper_manifest_by_id(id: &str) -> Result<ModelManifest, AppError> {
    match id {
        "large-v3-turbo" => manifest(),
        "small" => small_manifest(),
        "base" => base_manifest(),
        other => Err(AppError::Model(format!("modele inconnu: {other}"))),
    }
}

/// Resolves the model to use for a transcription: the user's explicit choice
/// if any and known, otherwise the default (`large-v3-turbo`, unchanged
/// behavior for anyone who never sees or uses the picker).
pub fn selected_whisper_manifest(model_id: Option<&str>) -> Result<ModelManifest, AppError> {
    match model_id {
        Some(id) => whisper_manifest_by_id(id),
        None => manifest(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelOption {
    pub id: String,
    pub name: String,
    pub size_mb: u64,
}

/// All Whisper models bundled with this build, for the model picker in
/// Reglages/Preparation. Bundled at build time like the single model always
/// was (see docs/README.md "Telechargement") - never downloaded by the
/// installed app.
pub fn list_whisper_models() -> Result<Vec<ModelOption>, AppError> {
    WHISPER_MODEL_IDS
        .iter()
        .map(|&id| {
            let manifest = whisper_manifest_by_id(id)?;
            Ok(ModelOption {
                id: id.to_string(),
                name: manifest.name,
                size_mb: manifest.size_bytes.div_ceil(1_000_000),
            })
        })
        .collect()
}

/// The speaker-embedding model used by `diarization` (see docs/ARCHITECTURE.md
/// "Diarisation"). Same bundling contract as the Whisper manifest above: built
/// into the binary, verified by checksum, never fetched at runtime.
pub fn diarization_manifest() -> Result<ModelManifest, AppError> {
    serde_json::from_str(include_str!(
        "../../resources/models/diarization-manifest.json"
    ))
    .map_err(|err| AppError::Model(format!("description du modele invalide: {err}")))
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state")]
pub enum ModelStatus {
    Ready {
        path: String,
        name: String,
        size_mb: u64,
    },
}

/// Uses only resources shipped with the application. Never accesses the network.
/// Android assets are streamed into private storage because Whisper needs a disk path.
pub fn ensure_model(app: &tauri::AppHandle) -> Result<ModelStatus, AppError> {
    ensure_manifest(app, manifest()?)
}

/// Same guarantee as `ensure_model`, generalized to any bundled model
/// manifest so `diarization` can reuse the exact verify/install logic
/// instead of duplicating the Android asset-streaming path.
pub fn ensure_manifest(
    app: &tauri::AppHandle,
    model: ModelManifest,
) -> Result<ModelStatus, AppError> {
    let resource = app
        .path()
        .resolve(
            format!("models/{}", model.file),
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|err| AppError::Model(format!("ressource du modele introuvable: {err}")))?;

    #[cfg(not(target_os = "android"))]
    let path = {
        // Cargo tests/dev runs may not have copied Tauri resources yet.
        #[cfg(debug_assertions)]
        let resource = if resource.is_file() {
            resource
        } else {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("resources/models")
                .join(&model.file)
        };
        verify_model(&resource, &model).map_err(|err| AppError::Model(format!(
            "Le modele integre {} est absent ou invalide. Reinstallez le paquet complet. En developpement, executez pnpm models:prepare. {err}", model.name
        )))?;
        resource
    };

    #[cfg(target_os = "android")]
    let path = {
        use tauri_plugin_fs::FsExt;
        let models_dir = app
            .path()
            .app_data_dir()
            .map_err(|err| AppError::Model(err.to_string()))?
            .join("models");
        install_bundled(&models_dir, &model, || {
            app.fs()
                .open(resource.clone(), tauri_plugin_fs::OpenOptions::default())
        })?
    };

    Ok(ModelStatus::Ready {
        path: path.to_string_lossy().into_owned(),
        name: model.name,
        size_mb: model.size_bytes.div_ceil(1_000_000),
    })
}

// Compiled on desktop in tests to exercise the Android extraction algorithm.
#[cfg(any(target_os = "android", test))]
fn install_bundled(
    models_dir: &Path,
    model: &ModelManifest,
    open_resource: impl FnOnce() -> std::io::Result<std::fs::File>,
) -> Result<PathBuf, AppError> {
    static INSTALL_LOCK: Mutex<()> = Mutex::new(());
    let _guard = INSTALL_LOCK
        .lock()
        .map_err(|_| AppError::Model("installation du modele interrompue".into()))?;
    std::fs::create_dir_all(models_dir)?;
    let destination = models_dir.join(&model.file);
    if verify_model(&destination, model).is_ok() {
        return Ok(destination);
    }
    let temporary = models_dir.join(format!("{}.part", model.file));
    let result = (|| {
        let mut source = open_resource()?;
        let mut file = std::fs::File::create(&temporary)?;
        std::io::copy(&mut source, &mut file)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);
        verify_model(&temporary, model)?;
        // Android uses POSIX rename: atomically replaces an invalid old cache.
        std::fs::rename(&temporary, &destination)?;
        Ok(destination)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn verify_model(path: &Path, model: &ModelManifest) -> Result<(), AppError> {
    if std::fs::metadata(path)?.len() != model.size_bytes {
        return Err(AppError::Model(
            "taille du modele integre incorrecte".into(),
        ));
    }
    verify_checksum(path, &model.sha256)
}

pub fn verify_checksum(path: &Path, expected_hex: &str) -> Result<(), AppError> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let actual: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if actual.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(AppError::Model(
            "somme de controle du modele invalide".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(label: &str) -> (PathBuf, ModelManifest, PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "interviewscribe-bundle-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let source = directory.join("source");
        std::fs::write(&source, b"hello world").unwrap();
        let model = ModelManifest {
            name: "Test".into(),
            file: "test.bin".into(),
            size_bytes: 11,
            sha256: "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".into(),
        };
        (directory, model, source)
    }

    #[test]
    fn default_is_large_v3_turbo() {
        let model = manifest().unwrap();
        assert_eq!(model.file, "ggml-large-v3-turbo-q5_0.bin");
        assert_eq!(model.sha256.len(), 64);
    }

    #[test]
    fn lists_all_three_bundled_whisper_models_with_valid_manifests() {
        let models = list_whisper_models().unwrap();
        let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["large-v3-turbo", "small", "base"]);
        for model in &models {
            assert!(model.size_mb > 0);
            assert!(!model.name.is_empty());
        }
    }

    #[test]
    fn selected_whisper_manifest_falls_back_to_default_when_unset() {
        let default = selected_whisper_manifest(None).unwrap();
        assert_eq!(default.file, "ggml-large-v3-turbo-q5_0.bin");
    }

    #[test]
    fn selected_whisper_manifest_resolves_a_smaller_model_by_id() {
        let small = selected_whisper_manifest(Some("small")).unwrap();
        assert_eq!(small.file, "ggml-small-q5_1.bin");
        let base = selected_whisper_manifest(Some("base")).unwrap();
        assert_eq!(base.file, "ggml-base-q5_1.bin");
    }

    #[test]
    fn selected_whisper_manifest_rejects_an_unknown_id() {
        assert!(selected_whisper_manifest(Some("does-not-exist")).is_err());
    }

    #[test]
    fn installs_offline_and_reuses_verified_copy() {
        let (dir, model, source) = fixture("install");
        let models = dir.join("models");
        let installed = install_bundled(&models, &model, || std::fs::File::open(&source)).unwrap();
        assert_eq!(std::fs::read(&installed).unwrap(), b"hello world");
        install_bundled(&models, &model, || panic!("must reuse valid cache")).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn corrupt_copy_is_repaired_from_bundle() {
        let (dir, model, source) = fixture("repair");
        let models = dir.join("models");
        std::fs::create_dir_all(&models).unwrap();
        std::fs::write(models.join(&model.file), b"wrong bytes").unwrap();
        let installed = install_bundled(&models, &model, || std::fs::File::open(&source)).unwrap();
        verify_model(&installed, &model).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_bundle_never_becomes_installed_model() {
        let (dir, model, source) = fixture("invalid");
        std::fs::write(&source, b"wrong bytes").unwrap();
        let models = dir.join("models");
        assert!(install_bundled(&models, &model, || std::fs::File::open(&source)).is_err());
        assert!(!models.join(&model.file).exists());
        assert!(!models.join("test.bin.part").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_bundle_does_not_accept_a_partial_download() {
        let (dir, model, source) = fixture("missing");
        let models = dir.join("models");
        std::fs::create_dir_all(&models).unwrap();
        std::fs::write(models.join("test.bin.part"), b"hello").unwrap();
        std::fs::remove_file(&source).unwrap();
        assert!(install_bundled(&models, &model, || std::fs::File::open(&source)).is_err());
        assert!(!models.join(&model.file).exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
