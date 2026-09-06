import { test } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { copyFile, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

for (const state of ["valid", "missing", "corrupt"]) {
  test(`packaging gate: ${state} bundled model`, async () => {
    const root = await mkdtemp(join(tmpdir(), "interviewscribe-package-test-"));
    try {
      await mkdir(join(root, "scripts"));
      const models = join(root, "src-tauri/resources/models");
      await mkdir(models, { recursive: true });
      const script = join(root, "scripts/prepare-model.mjs");
      await copyFile(new URL("./prepare-model.mjs", import.meta.url), script);
      const bytes = Buffer.from("test model");
      await writeFile(
        join(models, "manifest.json"),
        JSON.stringify({
          name: "Test",
          file: "test.bin",
          size_bytes: bytes.length,
          sha256: createHash("sha256").update(bytes).digest("hex"),
          url: "https://invalid.invalid/must-never-download",
        }),
      );
      if (state !== "missing")
        await writeFile(
          join(models, "test.bin"),
          state === "valid" ? bytes : Buffer.from("bad! model"),
        );
      const result = spawnSync(process.execPath, [script], {
        encoding: "utf8",
        timeout: 10000,
        env: { ...process.env, NODE_TEST_CONTEXT: undefined },
      });
      assert.equal(result.status, state === "valid" ? 0 : 1, result.stderr);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
}
