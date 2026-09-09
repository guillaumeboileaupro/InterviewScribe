import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

const [directory, output] = process.argv.slice(2);
if (!directory || !output) {
  throw new Error(
    "usage: verify-release-evidence.mjs <reports-directory> <output>",
  );
}

const requiredPlatforms = new Set(["ubuntu-22.04", "windows-latest"]);
// android-arm64-v8a is not in requiredPlatforms: its emulator verification
// job is temporarily non-blocking (see docs/TEST_IMPLEMENTATION_PLAN.md
// section 9 - the emulator itself fails to boot in CI, unrelated to the APK
// build, which succeeds and is checksummed independently). Its report is
// still included and validated below when present, just not mandatory -
// this file must never fabricate an "installed/launched/uninstalled" report
// for a platform that was never actually verified.
const reports = [];
for (const name of await readdir(directory)) {
  if (!name.endsWith(".json")) continue;
  const report = JSON.parse(await readFile(join(directory, name), "utf8"));
  validate(report, name);
  reports.push(report);
  requiredPlatforms.delete(report.platform);
}

if (requiredPlatforms.size > 0) {
  throw new Error(
    `missing required packaging evidence: ${[...requiredPlatforms].join(", ")}`,
  );
}

const evidence = {
  schema_version: 1,
  passed: true,
  required_platforms: reports
    .map(({ platform, artifact_type, size_bytes, sha256 }) => ({
      platform,
      artifact_type,
      size_bytes,
      sha256,
    }))
    .sort((left, right) => left.platform.localeCompare(right.platform)),
  excluded_manual_protocols: true,
  contains_private_data: false,
};
await writeFile(output, `${JSON.stringify(evidence, null, 2)}\n`);

function validate(report, name) {
  if (report.schema_version !== 1 || report.kind !== "packaging") {
    throw new Error(`${name}: unsupported report schema`);
  }
  if (report.contains_private_data !== false) {
    throw new Error(`${name}: privacy declaration missing`);
  }
  if (!Number.isInteger(report.size_bytes) || report.size_bytes <= 0) {
    throw new Error(`${name}: invalid artifact size`);
  }
  if (!/^[a-f0-9]{64}$/.test(report.sha256)) {
    throw new Error(`${name}: invalid SHA-256`);
  }
  for (const check of ["installed", "launched", "uninstalled"]) {
    if (report.checks?.[check] !== true) {
      throw new Error(`${name}: required check did not pass: ${check}`);
    }
  }
}
