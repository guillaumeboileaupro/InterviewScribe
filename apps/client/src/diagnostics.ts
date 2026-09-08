import { clientLog } from "./api";

/**
 * Breadcrumb for the local diagnostics log (see docs/ARCHITECTURE.md
 * "Diagnostics locaux") - never pass audio, transcript text or a speaker
 * name here, only technical/operational context.
 */
export function breadcrumb(message: string): void {
  clientLog("INFO", message).catch(() => {});
}

/**
 * Installs global handlers so a JS error or rejected promise - including
 * one that happens before any backend command is even reached - still
 * leaves a trace in the same file the user can copy from Reglages. Call
 * once at app startup.
 */
export function installGlobalErrorLogging(): void {
  window.addEventListener("error", (event) => {
    clientLog("ERROR", `erreur non interceptee: ${event.message}`).catch(
      () => {},
    );
  });
  window.addEventListener("unhandledrejection", (event) => {
    clientLog("ERROR", `promesse rejetee: ${String(event.reason)}`).catch(
      () => {},
    );
  });
}
