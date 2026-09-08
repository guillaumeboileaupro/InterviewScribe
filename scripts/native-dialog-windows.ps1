# Native Win32 common file dialogs (class "#32770") live outside the
# webview's DOM, so WebDriver cannot reach them - this drives the real OS
# window directly via user32.dll, the Windows counterpart to
# e2e/helpers/native-dialog.mjs's xdotool approach on Linux. Searches
# broadly for the filename Edit control rather than assuming one exact
# child hierarchy, since that has been known to vary across Windows
# versions and dialog styles.
param(
    [Parameter(Mandatory = $true)][string]$Path,
    [int]$TimeoutMs = 15000
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

public class NativeDialog {
    [DllImport("user32.dll")]
    public static extern IntPtr FindWindow(string lpClassName, string lpWindowName);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll", CharSet = CharSet.Auto)]
    public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, string lParam);

    [DllImport("user32.dll")]
    public static extern int GetClassName(IntPtr hWnd, StringBuilder lpClassName, int nMaxCount);

    public delegate bool EnumChildProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")]
    public static extern bool EnumChildWindows(IntPtr hWndParent, EnumChildProc lpEnumFunc, IntPtr lParam);

    public static IntPtr FindEditDescendant(IntPtr root) {
        IntPtr found = IntPtr.Zero;
        EnumChildProc callback = (hWnd, lParam) => {
            var sb = new StringBuilder(256);
            GetClassName(hWnd, sb, sb.Capacity);
            if (sb.ToString() == "Edit") {
                found = hWnd;
                return false; // stop enumeration
            }
            return true;
        };
        EnumChildWindows(root, callback, IntPtr.Zero);
        return found;
    }
}
"@

$deadline = (Get-Date).AddMilliseconds($TimeoutMs)
$dialog = [IntPtr]::Zero
while ((Get-Date) -lt $deadline) {
    $dialog = [NativeDialog]::FindWindow("#32770", $null)
    if ($dialog -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 300
}
if ($dialog -eq [IntPtr]::Zero) {
    Write-Error "native dialog (#32770) never appeared within ${TimeoutMs}ms"
    exit 1
}

[NativeDialog]::SetForegroundWindow($dialog) | Out-Null
Start-Sleep -Milliseconds 300

$edit = [NativeDialog]::FindEditDescendant($dialog)
if ($edit -eq [IntPtr]::Zero) {
    Write-Error "could not find the filename Edit control in the native dialog"
    exit 1
}

$WM_SETTEXT = 0x000C
[NativeDialog]::SendMessage($edit, $WM_SETTEXT, [IntPtr]::Zero, $Path) | Out-Null
Start-Sleep -Milliseconds 300
[System.Windows.Forms.SendKeys]::SendWait("{ENTER}")
