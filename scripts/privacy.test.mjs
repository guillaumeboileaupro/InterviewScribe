import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import { extname, join } from "node:path";
import { test } from "node:test";

const root = new URL("../", import.meta.url);

async function filesBelow(relativeDirectory, extensions) {
  const directory = new URL(relativeDirectory, root);
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(
    entries.map(async (entry) => {
      const relativePath = join(relativeDirectory, entry.name);
      if (entry.isDirectory())
        return filesBelow(`${relativePath}/`, extensions);
      return extensions.has(extname(entry.name)) ? [relativePath] : [];
    }),
  );
  return nested.flat();
}

test("production CSP permits local Tauri IPC but no remote connection", async () => {
  const config = JSON.parse(
    await readFile(new URL("src-tauri/tauri.conf.json", root), "utf8"),
  );
  const csp = config.app?.security?.csp;
  assert.equal(typeof csp, "string", "the production CSP must be enabled");
  assert.match(csp, /default-src 'self'/);
  assert.match(csp, /connect-src ipc: http:\/\/ipc\.localhost/);

  const connectSources = csp
    .split(";")
    .map((directive) => directive.trim())
    .find((directive) => directive.startsWith("connect-src "));
  assert.equal(
    connectSources,
    "connect-src ipc: http://ipc.localhost",
    "production must not allow a remote HTTP or WebSocket endpoint",
  );
});

test("production frontend contains no network client or console logging", async () => {
  const files = (
    await filesBelow("apps/client/src/", new Set([".ts", ".tsx"]))
  ).filter((path) => !path.includes(".test.") && !path.includes("/test/"));
  const forbidden =
    /\b(?:fetch|WebSocket|XMLHttpRequest)\s*\(|\bconsole\.(?:log|info|debug|warn|error)\s*\(/;
  for (const file of files) {
    const source = await readFile(new URL(file, root), "utf8");
    assert.doesNotMatch(
      source,
      forbidden,
      `${file} exposes network or console output`,
    );
  }
});

test("production Rust contains no stdout, stderr or debug logging", async () => {
  const files = await filesBelow("src-tauri/src/", new Set([".rs"]));
  const forbidden = /\b(?:println|eprintln|dbg)!|\b(?:log|tracing)::/;
  for (const file of files) {
    const source = await readFile(new URL(file, root), "utf8");
    // Test/QA modules may report aggregate measurements. They are excluded
    // from release builds and must remain below the first cfg(test) boundary.
    const productionSource = source.split("#[cfg(test)]", 1)[0];
    assert.doesNotMatch(
      productionSource,
      forbidden,
      `${file} logs in production code`,
    );
  }
});
