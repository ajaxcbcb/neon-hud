$ErrorActionPreference = 'Stop'
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
} finally {
    # Only terminate the process created by this isolated CI smoke check.
    $process.Refresh()
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
}
