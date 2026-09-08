import { submitPathToNativeDialogLinux } from "./native-dialog.linux.mjs";
import { submitPathToNativeDialogWindows } from "./native-dialog.windows.mjs";

/**
 * Native OS file dialogs (Open/Save) live outside the webview's DOM, so
 * WebDriver cannot reach them on either platform - each implementation
 * drives the real OS window directly (xdotool on Linux, Win32 interop via
 * PowerShell on Windows). `titlePattern` only applies on Linux, where
 * dialogs are found by window title; Windows finds them by the standard
 * common-dialog window class instead, so no title matching is needed there.
 */
export async function submitPathToNativeDialog(path, options = {}) {
  if (process.platform === "win32") {
    return submitPathToNativeDialogWindows(path, options);
  }
  return submitPathToNativeDialogLinux(path, options);
}
