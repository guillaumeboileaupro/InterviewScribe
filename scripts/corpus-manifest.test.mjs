import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const manifestUrl = new URL("../tests/corpus/manifest.json", import.meta.url);

test("corpus sources are public, licensed and integrity-pinned", async () => {
  const manifest = JSON.parse(await readFile(manifestUrl, "utf8"));
  assert.equal(manifest.policy.committed_audio, false);
  assert.equal(manifest.policy.contains_private_data, false);
  assert.ok(manifest.sources.length > 0);
  for (const source of manifest.sources) {
    assert.match(source.url, /^https:\/\//);
    assert.match(source.dataset_page, /^https:\/\//);
    assert.match(source.license_url, /^https:\/\//);
    assert.match(source.sha256, /^[a-f0-9]{64}$/);
    assert.ok(Number.isSafeInteger(source.size_bytes) && source.size_bytes > 0);
    assert.ok(source.redistributable);
    assert.ok(source.speaker_count >= 1 && source.speaker_count <= 5);
  }
});

test("the manifest tracks every required acoustic case", async () => {
  const manifest = JSON.parse(await readFile(manifestUrl, "utf8"));
  const ids = new Set(manifest.planned_cases.map((entry) => entry.id));
  for (const required of [
    "single-clean-fr",
    "multi-clean-4",
    "silence",
    "noise",
    "music",
    "overlap",
  ]) {
    assert.ok(ids.has(required), `missing corpus case: ${required}`);
  }
});
