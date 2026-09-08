// Real Tauri E2E harness (docs/TEST_IMPLEMENTATION_PLAN.md section 2): drives
// the actual application through `tauri-driver` (WebDriver protocol over
// WebKitWebDriver on Linux), never a simulated frontend. `tauri-driver` must
// already be running on 127.0.0.1:4444 before this config is used - see
// scripts/e2e-linux.sh, which starts it, waits for it, runs the suite, then
// stops it regardless of outcome.
//
// Deliberately points at the *installed* binary (from the .deb), not the raw
// `target/release/` build output: bundled resources (the Whisper model) are
// resolved by Tauri relative to the installed bundle layout in a release
// build - running the uninstalled executable directly leaves those lookups
// failing, which is what a real user's install never does. Verified: pointing
// this at target/release/interviewscribe made the model, and everything
// depending on it, fail to load in the same run that passes here.
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
  mochaOpts: {
    ui: "bdd",
    timeout: 60000,
  },
};
