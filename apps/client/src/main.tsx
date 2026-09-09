import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

// E2E test builds only (VITE_WDIO_E2E=true, set by scripts/e2e-linux.sh and
// scripts/e2e-windows.ps1) - never true in a real build, so Vite's minifier
// dead-code-eliminates this whole branch (and never emits the dependency
// into dist/) for every other build, including release.yml. Registers the
// frontend half of tauri-plugin-wdio, which @wdio/tauri-service's command
// hooks require - see the dependency comment in src-tauri/Cargo.toml.
if (import.meta.env.VITE_WDIO_E2E === "true") {
  import("@wdio/tauri-plugin");
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
