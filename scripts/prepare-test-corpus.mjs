import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { basename, resolve } from "node:path";

const sourcePath = resolve(process.argv[2] ?? "/tmp/ES2002a.Mix-Headset.wav");
const manifestPath = new URL("../tests/corpus/manifest.json", import.meta.url);
const outputDirectory = new URL("../tests/corpus/generated/", import.meta.url);
const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
const source = manifest.sources.find(
  (entry) => entry.id === "ami-es2002a-mix-headset",
);
if (!source) throw new Error("source AMI absente du manifeste");

const input = await readFile(sourcePath);
const digest = sha256(input);
if (input.length !== source.size_bytes || digest !== source.sha256) {
  throw new Error(
    `source AMI invalide: taille=${input.length}, sha256=${digest}`,
  );
}
const wav = parsePcm16MonoWav(input);
const startSample = 120 * wav.sampleRate;
const sampleCount = 10 * wav.sampleRate;
if (startSample + sampleCount > wav.samples.length)
  throw new Error("source AMI trop courte");
const clean = wav.samples.slice(startSample, startSample + sampleCount);
const silence = new Int16Array(sampleCount);
const noise = transform(
  clean,
  (sample, index) => sample + deterministicNoise(index) * 1800,
);
const music = transform(clean, (sample, index) => {
  const time = index / wav.sampleRate;
  return (
    sample +
    Math.sin(time * Math.PI * 440) * 1800 +
    Math.sin(time * Math.PI * 660) * 900
  );
});
const shift = Math.round(wav.sampleRate * 0.7);
const overlap = transform(
  clean,
  (sample, index) => sample + (clean[index + shift] ?? 0) * 0.7,
);

await mkdir(outputDirectory, { recursive: true });
const cases = { "multi-clean-4": clean, silence, noise, music, overlap };
const report = {
  source: basename(sourcePath),
  source_sha256: digest,
  cases: {},
};
for (const [id, samples] of Object.entries(cases)) {
  const bytes = encodePcm16MonoWav(samples, wav.sampleRate);
  const outputDigest = sha256(bytes);
  const expected = manifest.planned_cases.find(
    (entry) => entry.id === id,
  )?.sha256;
  if (expected && outputDigest !== expected) {
    throw new Error(
      `fixture ${id} non deterministe: attendu=${expected}, obtenu=${outputDigest}`,
    );
  }
  await writeFile(new URL(`${id}.wav`, outputDirectory), bytes);
  report.cases[id] = {
    sha256: outputDigest,
    size_bytes: bytes.length,
    duration_ms: 10_000,
  };
}
await writeFile(
  new URL("report.json", outputDirectory),
  `${JSON.stringify(report, null, 2)}\n`,
);
console.log(JSON.stringify(report, null, 2));

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function parsePcm16MonoWav(bytes) {
  if (
    bytes.toString("ascii", 0, 4) !== "RIFF" ||
    bytes.toString("ascii", 8, 12) !== "WAVE"
  ) {
    throw new Error("source non WAV");
  }
  let offset = 12;
  let sampleRate;
  let data;
  while (offset + 8 <= bytes.length) {
    const id = bytes.toString("ascii", offset, offset + 4);
    const size = bytes.readUInt32LE(offset + 4);
    const body = offset + 8;
    if (id === "fmt ") {
      if (
        bytes.readUInt16LE(body) !== 1 ||
        bytes.readUInt16LE(body + 2) !== 1 ||
        bytes.readUInt16LE(body + 14) !== 16
      ) {
        throw new Error("seul le PCM mono 16 bits est accepte");
      }
      sampleRate = bytes.readUInt32LE(body + 4);
    } else if (id === "data") {
      data = bytes.subarray(body, body + size);
    }
    offset = body + size + (size % 2);
  }
  if (!sampleRate || !data) throw new Error("WAV incomplet");
  return {
    sampleRate,
    samples: new Int16Array(data.buffer, data.byteOffset, data.length / 2),
  };
}

function transform(samples, callback) {
  return Int16Array.from(samples, (sample, index) =>
    Math.max(-32768, Math.min(32767, Math.round(callback(sample, index)))),
  );
}

function deterministicNoise(index) {
  let value = (index + 1) * 1664525 + 1013904223;
  value = (value >>> 0) / 0xffffffff;
  return value * 2 - 1;
}

function encodePcm16MonoWav(samples, sampleRate) {
  const output = Buffer.alloc(44 + samples.length * 2);
  output.write("RIFF", 0);
  output.writeUInt32LE(output.length - 8, 4);
  output.write("WAVEfmt ", 8);
  output.writeUInt32LE(16, 16);
  output.writeUInt16LE(1, 20);
  output.writeUInt16LE(1, 22);
  output.writeUInt32LE(sampleRate, 24);
  output.writeUInt32LE(sampleRate * 2, 28);
  output.writeUInt16LE(2, 32);
  output.writeUInt16LE(16, 34);
  output.write("data", 36);
  output.writeUInt32LE(samples.length * 2, 40);
  for (let index = 0; index < samples.length; index += 1)
    output.writeInt16LE(samples[index], 44 + index * 2);
  return output;
}
