[CmdletBinding()]
param(
    [string]$ExecutablePath = "",
    [int]$TimeoutSec = 10
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($ExecutablePath)) {
    $ExecutablePath = Join-Path $PSScriptRoot "..\target\release\codewarp.exe"
}

if (-not (Test-Path -LiteralPath $ExecutablePath -PathType Leaf)) {
    throw "Release executable not found: $ExecutablePath"
}

if ($null -eq ("CodeWarpGuiSmokeNative" -as [type])) {
    Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class CodeWarpGuiSmokeNative {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern IntPtr SetFocus(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

    [DllImport("user32.dll")]
    public static extern bool AttachThreadInput(uint attach, uint attachTo, bool attachState);

    [DllImport("user32.dll")]
    public static extern bool BringWindowToTop(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern IntPtr SetActiveWindow(IntPtr hWnd);

    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int command);

    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);

    [DllImport("user32.dll")]
    public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);

    [DllImport("user32.dll")]
    public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extraInfo);

    public static void Key(byte key) {
        keybd_event(key, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, 2, UIntPtr.Zero);
    }

    public static void CtrlKey(byte key) {
        keybd_event(0x11, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, 0, UIntPtr.Zero);
        keybd_event(key, 0, 2, UIntPtr.Zero);
        keybd_event(0x11, 0, 2, UIntPtr.Zero);
    }

    public static void Click(int x, int y) {
        SetCursorPos(x, y);
        mouse_event(2, 0, 0, 0, UIntPtr.Zero);
        mouse_event(4, 0, 0, 0, UIntPtr.Zero);
    }

    public static bool FocusWindow(IntPtr hWnd) {
        var foreground = GetForegroundWindow();
        uint ignoredProcessId;
        var foregroundThread = GetWindowThreadProcessId(foreground, out ignoredProcessId);
        var targetThread = GetWindowThreadProcessId(hWnd, out ignoredProcessId);
        var attached = foregroundThread != 0 && targetThread != 0
            && foregroundThread != targetThread
            && AttachThreadInput(targetThread, foregroundThread, true);
        ShowWindow(hWnd, 9);
        BringWindowToTop(hWnd);
        SetForegroundWindow(hWnd);
        SetActiveWindow(hWnd);
        SetFocus(hWnd);
        if (attached) {
            AttachThreadInput(targetThread, foregroundThread, false);
        }
        return GetForegroundWindow() == hWnd;
    }
}
"@
}

$process = $null
try {
    $process = Start-Process -FilePath (Resolve-Path -LiteralPath $ExecutablePath) -PassThru
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    do {
        Start-Sleep -Milliseconds 200
        $process.Refresh()
        $handle = $process.MainWindowHandle
    } while ($handle -eq 0 -and (Get-Date) -lt $deadline)

    if ($handle -eq 0) {
        throw "CodeWarp window did not open within $TimeoutSec seconds"
    }

    $rect = New-Object CodeWarpGuiSmokeNative+RECT
    if (-not [CodeWarpGuiSmokeNative]::GetWindowRect($handle, [ref]$rect)) {
        throw "CodeWarp window bounds could not be read"
    }

    [CodeWarpGuiSmokeNative]::FocusWindow($handle) | Out-Null
    Start-Sleep -Milliseconds 300
    if ([CodeWarpGuiSmokeNative]::GetForegroundWindow() -ne $handle) {
        throw "CodeWarp window is not foreground; run this smoke test on an interactive Windows desktop"
    }
    [CodeWarpGuiSmokeNative]::Key(0x1B)
    Start-Sleep -Milliseconds 250
    [CodeWarpGuiSmokeNative]::Click(
        [int](($rect.Left + $rect.Right) / 2),
        $rect.Bottom - 72
    )
    Start-Sleep -Milliseconds 250

    $probe = [string]::Concat(
        [char]0xD55C, [char]0xAE00, " ",
        [char]0xC785, [char]0xB825, " ",
        [char]0xD83D, [char]0xDE0A, "`r`n",
        [char]0xB450, " ", [char]0xBC88, [char]0xC9F8, " ", [char]0xC904
    )
    Set-Clipboard -Value $probe
    [CodeWarpGuiSmokeNative]::CtrlKey(0x56)
    Start-Sleep -Milliseconds 500
    Set-Clipboard -Value "codewarp-gui-smoke-sentinel"
    [CodeWarpGuiSmokeNative]::CtrlKey(0x41)
    [CodeWarpGuiSmokeNative]::CtrlKey(0x43)
    Start-Sleep -Milliseconds 250

    $roundTrip = Get-Clipboard -Raw
    if ($roundTrip -cne $probe) {
        throw ("GUI text round-trip mismatch: expected [" + $probe + "], got [" + $roundTrip + "]")
    }

    Write-Host "GUI smoke passed: Unicode paste, newline, and text order preserved."
}
finally {
    if ($null -ne $process) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
}
