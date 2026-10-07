param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$executablePath = (Resolve-Path -LiteralPath $Executable).Path
$stdout = Join-Path $OutputDirectory 'stdout.log'
$stderr = Join-Path $OutputDirectory 'stderr.log'
$screenshot = Join-Path $OutputDirectory 'native-window.png'
$receiptPath = Join-Path $OutputDirectory 'receipt.json'

Add-Type -ReferencedAssemblies System.Drawing @'
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Runtime.InteropServices;

public static class NativeHudCapture {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out RECT rect);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr window, int command);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr dc, uint flags);

    public static string Capture(IntPtr window, string path, out int width, out int height, out int distinctColors) {
        RECT rect;
        if (!GetWindowRect(window, out rect)) throw new InvalidOperationException("GetWindowRect failed");
        width = rect.Right - rect.Left;
        height = rect.Bottom - rect.Top;
        if (width < 64 || height < 32 || width > 4096 || height > 4096)
            throw new InvalidOperationException("Unexpected native window dimensions");
        using (var bitmap = new Bitmap(width, height)) {
            bool printed;
            using (var graphics = Graphics.FromImage(bitmap)) {
                IntPtr dc = graphics.GetHdc();
                try { printed = PrintWindow(window, dc, 2); }
                finally { graphics.ReleaseHdc(dc); }
            }
            string method = "PrintWindow";
            distinctColors = CountColors(bitmap);
            if (!printed || distinctColors < 3) {
                ShowWindowAsync(window, 9);
                SetForegroundWindow(window);
                System.Threading.Thread.Sleep(300);
                using (var graphics = Graphics.FromImage(bitmap)) {
                    graphics.CopyFromScreen(rect.Left, rect.Top, 0, 0, bitmap.Size);
                }
                method = "screen-copy";
                distinctColors = CountColors(bitmap);
            }
            bitmap.Save(path, System.Drawing.Imaging.ImageFormat.Png);
            return method;
        }
    }
    private static int CountColors(Bitmap bitmap) {
        var colors = new HashSet<int>();
        for (int y = 0; y < bitmap.Height; y += 3) {
            for (int x = 0; x < bitmap.Width; x += 3) {
                colors.Add(bitmap.GetPixel(x, y).ToArgb());
                if (colors.Count >= 3) return colors.Count;
            }
        }
        return colors.Count;
    }
}
'@

$started = Get-Date
$process = Start-Process -FilePath $executablePath -ArgumentList '--smoke' -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
try {
    $deadline = $started.AddSeconds(7)
    do {
        Start-Sleep -Milliseconds 200
        $process.Refresh()
        if ($process.HasExited) { throw "Native GUI exited before window creation (code $($process.ExitCode))" }
    } while (($process.MainWindowHandle -eq [IntPtr]::Zero -or -not [NativeHudCapture]::IsWindowVisible($process.MainWindowHandle)) -and (Get-Date) -lt $deadline)
    if ($process.MainWindowHandle -eq [IntPtr]::Zero -or -not [NativeHudCapture]::IsWindowVisible($process.MainWindowHandle)) {
        throw 'Native GUI did not expose a visible window within seven seconds'
    }
    $windowTitle = $process.MainWindowTitle

    $captureAt = $started.AddSeconds(4)
    $remaining = [int][Math]::Ceiling(($captureAt - (Get-Date)).TotalMilliseconds)
    if ($remaining -gt 0) { Start-Sleep -Milliseconds $remaining }
    $process.Refresh()
    if ($process.HasExited) { throw "Native GUI exited before the four-second capture (code $($process.ExitCode))" }

    $width = 0; $height = 0; $colors = 0
    $method = [NativeHudCapture]::Capture($process.MainWindowHandle, $screenshot, [ref]$width, [ref]$height, [ref]$colors)
    if ($colors -lt 3) { throw "Native window screenshot has only $colors sampled colors" }

    $tree = [System.Collections.Generic.List[object]]::new()
    $pending = [System.Collections.Generic.Queue[int]]::new()
    $pending.Enqueue($process.Id)
    while ($pending.Count -gt 0) {
        $parent = $pending.Dequeue()
        $node = Get-CimInstance Win32_Process -Filter "ProcessId = $parent"
        if ($null -ne $node) {
            $tree.Add([pscustomobject]@{ pid = [int]$node.ProcessId; parentPid = [int]$node.ParentProcessId; name = $node.Name })
            foreach ($child in @(Get-CimInstance Win32_Process -Filter "ParentProcessId = $parent")) {
                $pending.Enqueue([int]$child.ProcessId)
            }
        }
    }

    $exitDeadline = (Get-Date).AddSeconds(20)
    do {
        Start-Sleep -Milliseconds 200
        $process.Refresh()
    } while (-not $process.HasExited -and (Get-Date) -lt $exitDeadline)
    if (-not $process.HasExited) { throw 'Native --smoke did not autoexit within 20 seconds after capture' }
    if ($process.ExitCode -ne 0) { throw "Native --smoke exited with code $($process.ExitCode)" }

    $receipt = [ordered]@{
        executable = [IO.Path]::GetFileName($executablePath)
        argument = '--smoke'
        windowTitle = $windowTitle
        screenshot = [IO.Path]::GetFileName($screenshot)
        captureMethod = $method
        width = $width
        height = $height
        minimumDistinctColorsObserved = $colors
        processTreeAtCapture = @($tree)
        exitCode = $process.ExitCode
        strictSingleProcessProven = $false
    }
    $receipt | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $receiptPath
    Write-Output "Native GUI smoke passed: $($width)x$($height), $method, exit $($process.ExitCode); receipt $receiptPath"
} catch {
    [ordered]@{
        result = 'failed'
        failure = $_.Exception.Message
        screenshot = if (Test-Path -LiteralPath $screenshot) { [IO.Path]::GetFileName($screenshot) } else { $null }
    } | ConvertTo-Json | Set-Content -LiteralPath $receiptPath
    throw
} finally {
    $process.Refresh()
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
}
