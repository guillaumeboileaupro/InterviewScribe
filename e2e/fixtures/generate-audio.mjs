import { writeFileSync } from "node:fs";

// Generates a short synthetic WAV at test-run time - never committed to Git
// (this project never commits real or synthetic audio). A formant-ish mix
// rather than a pure tone, closer to voiced sound so it exercises the real
// import -> decode -> Whisper pipeline rather than being silently dropped
// as non-speech by VAD-adjacent logic.
export function generateSyntheticWav(path, { durationSeconds = 3 } = {}) {
  const sampleRate = 16000;
  const sampleCount = sampleRate * durationSeconds;
  const dataSize = sampleCount * 2;

  const header = Buffer.alloc(44);
  header.write("RIFF", 0);
  header.writeUInt32LE(36 + dataSize, 4);
  header.write("WAVE", 8);
  header.write("fmt ", 12);
  header.writeUInt32LE(16, 16);
  header.writeUInt16LE(1, 20); // PCM
  header.writeUInt16LE(1, 22); // mono
  header.writeUInt32LE(sampleRate, 24);
  header.writeUInt32LE(sampleRate * 2, 28); // byte rate
  header.writeUInt16LE(2, 32); // block align
  header.writeUInt16LE(16, 34); // bits per sample
  header.write("data", 36);
  header.writeUInt32LE(dataSize, 40);

  const data = Buffer.alloc(dataSize);
  let seed = 42;
  const random = () => {
    seed = (seed * 1103515245 + 12345) & 0x7fffffff;
    return seed / 0x7fffffff;
  };
  for (let i = 0; i < sampleCount; i++) {
    const t = i / sampleRate;
    const value =
      0.25 * Math.sin(2 * Math.PI * 180 * t) +
      0.15 * Math.sin(2 * Math.PI * 500 * t) +
      0.1 * Math.sin(2 * Math.PI * 1200 * t) +
      0.05 * (random() - 0.5);
    const clamped = Math.max(-1, Math.min(1, value));
    data.writeInt16LE(Math.round(clamped * 32767 * 0.6), i * 2);
  }

  writeFileSync(path, Buffer.concat([header, data]));
}
