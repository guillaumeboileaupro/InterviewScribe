fn main() {
    // `wdio-e2e.json` declares permissions (`wdio:default`, `wdio-webdriver:default`)
    // that only resolve when the matching plugins are actually compiled in
    // (the `wdio-e2e` feature - see Cargo.toml). tauri-build's default glob
    // (`./capabilities/**/*`) validates every file under `capabilities/`
    // unconditionally, regardless of `app.security.capabilities` in
    // tauri.conf.json - so with the default glob, that file's mere presence
    // on disk breaks `cargo test`/`cargo clippy`/`cargo build` even without
    // the feature (verified: "Permission wdio:default not found" from a
    // plain `cargo test` before this fix). Restrict the glob to exclude it
    // unless the feature is active, so a normal build never sees it.
    println!("cargo:rerun-if-changed=capabilities");
    let capabilities_pattern = if cfg!(feature = "wdio-e2e") {
        "./capabilities/**/*"
    } else {
        "./capabilities/default.json"
    };
    tauri_build::try_build(
        tauri_build::Attributes::new().capabilities_path_pattern(capabilities_pattern),
    )
    .expect("failed to run tauri-build");

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
