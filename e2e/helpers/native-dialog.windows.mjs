import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(
  new URL("../../scripts/native-dialog-windows.ps1", import.meta.url),
);

/**
 * Windows counterpart to submitPathToNativeDialogLinux: the native common
 * file dialog (class "#32770") lives outside the webview's DOM too, driven
 * here via a PowerShell/user32.dll script instead of xdotool - see
 * scripts/native-dialog-windows.ps1.
 */
export async function submitPathToNativeDialogWindows(
  path,
  { timeoutMs = 15000 } = {},
) {
  const result = spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      scriptPath,
      "-Path",
      path,
      "-TimeoutMs",
      String(timeoutMs),
    ],
    { encoding: "utf8" },
  );
  if (result.status !== 0) {
    throw new Error(
      `native-dialog-windows.ps1 failed (exit ${result.status}): ${result.stderr}`,
    );
  }
}
