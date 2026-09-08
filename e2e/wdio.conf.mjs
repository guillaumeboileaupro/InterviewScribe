// Real Tauri E2E harness (docs/TEST_IMPLEMENTATION_PLAN.md section 2): drives
// the actual application through `tauri-driver` (WebDriver protocol over
// WebKitWebDriver on Linux, Microsoft Edge/WebView2 on Windows), never a
// simulated frontend. `tauri-driver` must already be running on
// 127.0.0.1:4444 before this config is used - see scripts/e2e-linux.sh and
// scripts/e2e-windows.ps1, which start it, wait for it, run the suite, then
// stop it (and the app process it spawned) regardless of outcome.
//
// Deliberately points at the *installed* binary (from the .deb/NSIS
// installer), not the raw `target/release/` build output: bundled resources
// (the Whisper model) are resolved by Tauri relative to the installed
// bundle layout in a release build - running the uninstalled executable
// directly leaves those lookups failing, which is what a real user's
// install never does. Verified on Linux: pointing this at
// target/release/interviewscribe made the model, and everything depending
// on it, fail to load in the same run that passes here.
// scripts/e2e-windows.ps1 always sets INTERVIEWSCRIBE_E2E_BINARY itself
// (the NSIS install path isn't fixed the way the Linux .deb path is), so
// the hardcoded fallback below only ever matters on Linux.
const applicationPath =
  process.env.INTERVIEWSCRIBE_E2E_BINARY ?? "/usr/bin/interviewscribe";

export const config = {
  hostname: "127.0.0.1",
  port: 4444,
  path: "/",
  specs: ["./specs/**/*.e2e.mjs"],
  maxInstances: 1,
  capabilities: [
    {
      "tauri:options": {
        application: applicationPath,
      },
    },
  ],
  logLevel: "warn",
  framework: "mocha",
  reporters: ["spec"],
  maxInstancesPerCapability: 1,
  mochaOpts: {
    ui: "bdd",
    // The workflow spec waits for a real Whisper transcription to finish
    // (up to 180s observed - see e2e/specs/workflow.e2e.mjs); this must
    // stay above that or mocha kills the test with a generic "Timeout"
    // before the more descriptive per-step waitUntil message can surface.
    timeout: 300000,
  },
};
