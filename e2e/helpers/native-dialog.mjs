import { spawnSync } from "node:child_process";

function run(cmd, args) {
  const result = spawnSync(cmd, args, { encoding: "utf8" });
  return result.stdout.trim();
}

/**
 * Native GTK/portal file dialogs live outside the webview's DOM, so
 * WebDriver cannot reach them directly - this drives the real OS window via
 * `xdotool` instead: find it by title, focus it, open the location bar
 * (Ctrl+L, standard GTK file chooser shortcut), type the path, confirm.
 * Verified against the real "Open File" and "Enregistrer sous" dialogs this
 * app actually shows (Tauri's dialog plugin over xdg-desktop-portal).
 */
export async function submitPathToNativeDialog(
  path,
  { titlePattern = "^Open File$", timeoutMs = 15000 } = {},
) {
  let windowId = "";
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const ids = run("xdotool", ["search", "--name", titlePattern])
      .split("\n")
      .filter(Boolean);
    if (ids.length > 0) {
      windowId = ids[ids.length - 1];
      break;
    }
    await new Promise((resolve) => setTimeout(resolve, 300));
  }
  if (!windowId) {
    throw new Error(
      `native dialog matching /${titlePattern}/ never appeared within ${timeoutMs}ms`,
    );
  }

  run("xdotool", ["windowactivate", "--sync", windowId]);
  await new Promise((resolve) => setTimeout(resolve, 300));
  run("xdotool", ["key", "--window", windowId, "ctrl+l"]);
  await new Promise((resolve) => setTimeout(resolve, 300));
  run("xdotool", ["type", "--window", windowId, "--delay", "20", path]);
  await new Promise((resolve) => setTimeout(resolve, 300));
  run("xdotool", ["key", "--window", windowId, "Return"]);
}
