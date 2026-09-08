// Real Tauri E2E harness (docs/TEST_IMPLEMENTATION_PLAN.md section 2): drives
// the actual application via @wdio/tauri-service's embedded WebDriver
// provider (tauri-plugin-wdio-webdriver, registered only under the
// `wdio-e2e` Cargo feature - see src-tauri/Cargo.toml/lib.rs). The plugin
// runs a real W3C WebDriver server *inside* the app itself, so there is no
// external tauri-driver/msedgedriver process to install, version-match, or
// manage - a deliberate switch away from driving tauri-driver directly
// (Tauri's current docs frame that as the advanced/fallback path); see
// scripts/e2e-linux.sh and scripts/e2e-windows.ps1 for what's left to set up
// (an isolated app-data profile) now that the driver lifecycle is handled
// for us.
//
// Deliberately points at the *installed* binary (from the .deb/NSIS
// installer), not the raw `target/release/` build output: bundled resources
// (the Whisper model) are resolved by Tauri relative to the installed
// bundle layout in a release build - running the uninstalled executable
// directly leaves those lookups failing, which is what a real user's
// install never does. Verified on Linux: pointing this at
// target/release/interviewscribe made the model, and everything depending
// on it, fail to load in the same run that passes here.
const applicationPath =
  process.env.INTERVIEWSCRIBE_E2E_BINARY ?? "/usr/bin/interviewscribe";

export const config = {
  specs: ["./specs/**/*.e2e.mjs"],
  maxInstances: 1,
  services: [
    [
      "tauri",
      {
        appBinaryPath: applicationPath,
        driverProvider: "embedded",
        // The embedded server has been observed to need more than the
        // library default under real CI load (see docs/TEST_IMPLEMENTATION_PLAN.md
        // item 2.6) - Tauri's own docs call out "containerised Windows
        // runners" by name for this setting.
        statusPollTimeout: 8000,
        startTimeout: 90000,
      },
    ],
  ],
  logLevel: "warn",
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: {
    ui: "bdd",
    // The workflow spec waits for a real Whisper transcription to finish
    // (up to 180s observed - see e2e/specs/workflow.e2e.mjs); this must
    // stay above that or mocha kills the test with a generic "Timeout"
    // before the more descriptive per-step waitUntil message can surface.
    timeout: 300000,
  },
};
