$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class HudWindow {
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr window, int command);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
}
'@
$executable = Resolve-Path 'src-tauri/target/x86_64-pc-windows-msvc/release/neon-hud.exe'
$logDirectory = Join-Path $env:RUNNER_TEMP 'neon-hud-launch'
New-Item -ItemType Directory -Force -Path $logDirectory | Out-Null
$stderr = Join-Path $logDirectory 'stderr.log'
$stdout = Join-Path $logDirectory 'stdout.log'
$process = Start-Process -FilePath $executable -PassThru -RedirectStandardError $stderr -RedirectStandardOutput $stdout
try {
    $deadline = (Get-Date).AddSeconds(30)
    do {
        Start-Sleep -Milliseconds 500
        $process.Refresh()
        if ($process.HasExited) {
            $detail = Get-Content -LiteralPath $stderr -Raw -ErrorAction SilentlyContinue
            throw "Neon HUD exited during startup ($($process.ExitCode)): $detail"
        }
    } while ($process.MainWindowHandle -eq 0 -and (Get-Date) -lt $deadline)
    if ($process.MainWindowHandle -eq 0) {
        throw 'Neon HUD did not create a window within 30 seconds'
    }
    Start-Sleep -Seconds 10
    $process.Refresh()
    if ($process.HasExited) {
        $detail = Get-Content -LiteralPath $stderr -Raw -ErrorAction SilentlyContinue
        throw "Neon HUD exited after window creation ($($process.ExitCode)): $detail"
    }
    Write-Output "Launch passed: $($process.MainWindowTitle), visible window $($process.MainWindowHandle)"
    $window = $process.MainWindowHandle
    [HudWindow]::ShowWindowAsync($window, 6) | Out-Null
    $hideDeadline = (Get-Date).AddSeconds(5)
    do { Start-Sleep -Milliseconds 100 } while ([HudWindow]::IsWindowVisible($window) -and (Get-Date) -lt $hideDeadline)
    $process.Refresh()
    if ($process.HasExited -or [HudWindow]::IsWindowVisible($window)) {
        throw 'Minimize did not hide the window while keeping Neon HUD running in the tray'
    }
    Write-Output 'Minimize passed: window hidden, tray process remains running'
} finally {
    # Only terminate the process created by this isolated CI smoke check.
    $process.Refresh()
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
}
