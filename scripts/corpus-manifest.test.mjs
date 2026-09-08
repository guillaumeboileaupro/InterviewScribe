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

test("quality thresholds are finite and release-blocking", async () => {
  const { quality_thresholds: thresholds } = JSON.parse(
    await readFile(manifestUrl, "utf8"),
  );
  for (const key of ["max_wer", "max_cer", "max_der", "max_duplicate_rate"]) {
    assert.ok(Number.isFinite(thresholds[key]));
    assert.ok(thresholds[key] >= 0 && thresholds[key] <= 1);
  }
  assert.ok(Number.isInteger(thresholds.max_timestamp_drift_ms));
  assert.ok(thresholds.max_timestamp_drift_ms > 0);
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
  for (const generated of manifest.planned_cases.filter(
    (entry) => entry.state === "generated-by-script",
  )) {
    assert.match(generated.sha256, /^[a-f0-9]{64}$/);
  }
  const sourceIds = new Set(manifest.sources.map((source) => source.id));
  for (const selected of manifest.planned_cases.filter(
    (entry) => entry.state === "selected",
  )) {
    assert.ok(
      sourceIds.has(selected.source),
      `unknown selected source: ${selected.source}`,
    );
  }
});
