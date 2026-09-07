fn main() {
    tauri_build::build();

    // `src-tauri/gen/android/` is gitignored and regenerated from scratch by
    // `pnpm tauri android init` (including in CI, every run) - editing its
    // AndroidManifest.xml directly would never survive that. `build.rs`
    // itself reruns on every build instead, so this is the place that
    // persists Android manifest customizations, using Tauri's own public
    // mechanism for exactly this (also used internally for file-association
    // intent filters). Idempotent: safe to run on every build.
    //
    // `#[cfg(target_os = "android")]` would be wrong here: build scripts
    // always compile for and run on the *host*, never the cross-compilation
    // target, so that attribute would check this machine's OS, not
    // Android's. `CARGO_CFG_TARGET_OS` is the env var Cargo sets to the
    // actual target for exactly this situation.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        tauri_utils::build::update_android_manifest(
            "record-audio",
            "manifest",
            "<uses-permission android:name=\"android.permission.RECORD_AUDIO\" />".into(),
        )
        .expect("failed to add the RECORD_AUDIO permission to AndroidManifest.xml");
    }
}
