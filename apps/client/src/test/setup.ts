import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";

// A real Tauri webview injects this for a synchronous unlisten fast-path;
// jsdom has no such thing, so @tauri-apps/api/event's listen()/unlisten()
// would otherwise throw during effect cleanup in any test using events.
(
  window as unknown as {
    __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => void };
  }
).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
  unregisterListener: () => {},
};

afterEach(() => {
  cleanup();
});
