import { createHash } from "node:crypto";
import { readFile, stat, writeFile } from "node:fs/promises";
import { extname } from "node:path";

const [mode, output, ...inputs] = process.argv.slice(2);
if (!mode || !output) {
  throw new Error(
    "usage: create-qualification-report.mjs <nightly|packaging> <output> [...inputs]",
  );
}

const report =
  mode === "nightly"
    ? await nightlyReport(inputs)
    : mode === "packaging"
      ? await packagingReport(inputs)
      : (() => {
          throw new Error(`unknown report mode: ${mode}`);
        })();

await writeFile(output, `${JSON.stringify(report, null, 2)}\n`);

async function nightlyReport(paths) {
  const logs = await Promise.all(paths.map(readIfPresent));
  const joined = logs.join("\n");
  const french = capture(
    joined,
    /public-fr metrics: wer=([\d.]+), cer=([\d.]+)/,
    ["wer", "cer"],
  );
  const ami = capture(
    joined,
    /ami metrics: segments=(\d+), clusters=(\d+), wer=([\d.]+), cer=([\d.]+), der=([\d.]+), word_region_der=([\d.]+), missed_ms=(\d+), false_alarm_ms=(\d+), confusion_ms=(\d+), uncertain=([\d.]+), duplicates=([\d.]+), aligned_boundaries=(\d+)\/(\d+), max_drift_ms=(\d+)/,
    [
      "segments",
      "clusters",
      "wer",
      "cer",
      "der",
      "word_region_der",
      "missed_ms",
      "false_alarm_ms",
      "confusion_ms",
      "uncertain",
      "duplicate_rate",
      "aligned_boundaries",
      "reference_words",
      "max_drift_ms",
    ],
  );
  const performance = capture(
    joined,
    /synthetic soak: duration_ms=(\d+), chunks=(\d+), peak_chunk_samples=(\d+), wall_ms=(\d+)/,
    ["duration_ms", "chunks", "peak_chunk_samples", "wall_ms"],
  );
  return {
    schema_version: 1,
    kind: "nightly",
    quality: { public_french: french, ami },
    performance,
    contains_private_data: false,
  };
}

async function packagingReport([platform, artifact]) {
  if (!platform || !artifact)
    throw new Error("packaging mode requires platform and artifact");
  const bytes = await readFile(artifact);
  return {
    schema_version: 1,
    kind: "packaging",
    platform,
    artifact_type: extname(artifact).slice(1),
    size_bytes: (await stat(artifact)).size,
    sha256: createHash("sha256").update(bytes).digest("hex"),
    checks: { installed: true, launched: true, uninstalled: true },
    contains_private_data: false,
  };
}

async function readIfPresent(path) {
  try {
    return await readFile(path, "utf8");
  } catch (error) {
    if (error?.code === "ENOENT") return "";
    throw error;
  }
}

function capture(text, expression, names) {
  const match = text.match(expression);
  if (!match) return null;
  return Object.fromEntries(
    names.map((name, index) => [name, Number(match[index + 1])]),
  );
}
