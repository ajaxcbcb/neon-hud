param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [ValidateRange(0, 3)][int]$SettingsPage = 0,
    [switch]$HoverEdges
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$executablePath = (Resolve-Path -LiteralPath $Executable).Path
$stdout = Join-Path $OutputDirectory 'stdout.log'
$stderr = Join-Path $OutputDirectory 'stderr.log'
$receiptPath = Join-Path $OutputDirectory 'receipt.json'

$drawingReferences = @(
    Get-ChildItem -LiteralPath (Join-Path $PSHOME 'ref') -Filter '*.dll' -File | Select-Object -ExpandProperty FullName
) + @([System.Drawing.Bitmap].Assembly.Location) + @(
    Get-ChildItem -LiteralPath $PSHOME -Filter 'System.Private.Windows.*.dll' -File | Select-Object -ExpandProperty FullName
)
Add-Type -ReferencedAssemblies $drawingReferences @'
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Text;

public static class NativeHudCapture {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct MONITORINFO { public int Size; public RECT Monitor, Work; public int Flags; }
    public class WindowInfo { public IntPtr Handle; public string Title; public RECT Rect; }
    public delegate bool EnumWindowsCallback(IntPtr window, IntPtr param);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsCallback callback, IntPtr param);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out RECT rect);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextLength(IntPtr window);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr window, StringBuilder title, int maxCount);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr window, int command);
    [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr window, uint flags);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern bool GetMonitorInfo(IntPtr monitor, ref MONITORINFO info);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);

    public static List<WindowInfo> VisibleWindows(uint launchedPid) {
        var windows = new List<WindowInfo>();
        EnumWindowsCallback callback = (window, param) => {
            uint ownerPid;
            GetWindowThreadProcessId(window, out ownerPid);
            if (ownerPid != launchedPid || !IsWindowVisible(window)) return true;
            RECT rect;
            if (!GetWindowRect(window, out rect)) return true;
            var title = new StringBuilder(GetWindowTextLength(window) + 1);
            GetWindowText(window, title, title.Capacity);
            windows.Add(new WindowInfo { Handle = window, Title = title.ToString(), Rect = rect });
            return true;
        };
        if (!EnumWindows(callback, IntPtr.Zero)) throw new InvalidOperationException("EnumWindows failed");
        return windows;
    }

    public static string Capture(IntPtr window, string path, bool activate, out int width, out int height, out int distinctColors) {
        if (activate) {
            ShowWindowAsync(window, 9);
            SetForegroundWindow(window);
        }
        System.Threading.Thread.Sleep(350);
        if (activate && GetForegroundWindow() != window)
            throw new InvalidOperationException("Native window could not be brought to the foreground");
        RECT rect;
        if (!GetWindowRect(window, out rect)) throw new InvalidOperationException("GetWindowRect failed");
        width = rect.Right - rect.Left;
        height = rect.Bottom - rect.Top;
        if (width < 64 || height < 32 || width > 4096 || height > 4096)
            throw new InvalidOperationException("Unexpected native window dimensions");
        int screenX = GetSystemMetrics(76), screenY = GetSystemMetrics(77);
        int screenWidth = GetSystemMetrics(78), screenHeight = GetSystemMetrics(79);
        if (rect.Left < screenX || rect.Top < screenY || rect.Right > screenX + screenWidth || rect.Bottom > screenY + screenHeight)
            throw new InvalidOperationException("Native window extends outside the CI desktop; capture would be clipped");
        var monitor = new MONITORINFO { Size = Marshal.SizeOf<MONITORINFO>() };
        if (!GetMonitorInfo(MonitorFromWindow(window, 2), ref monitor))
            throw new InvalidOperationException("Could not inspect native window work area");
        if (rect.Left < monitor.Work.Left || rect.Top < monitor.Work.Top || rect.Right > monitor.Work.Right || rect.Bottom > monitor.Work.Bottom)
            throw new InvalidOperationException("Native window extends outside the work area; taskbar could obscure capture");
        using (var bitmap = new Bitmap(width, height)) {
            using (var graphics = Graphics.FromImage(bitmap)) {
                graphics.CopyFromScreen(rect.Left, rect.Top, 0, 0, bitmap.Size);
            }
            distinctColors = CountColors(bitmap);
            bitmap.Save(path, System.Drawing.Imaging.ImageFormat.Png);
            return activate ? "foreground-screen-copy" : "screen-copy";
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
$arguments = @('--smoke', "--smoke-page=$SettingsPage")
if ($HoverEdges) { $arguments += '--smoke-hover' }
# This CI-only foreground capture requires visible windows for real cursor input.
$process = Start-Process -FilePath $executablePath -ArgumentList $arguments -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
try {
    $deadline = $started.AddSeconds(7)
    $expected = [ordered]@{
        hud = 'Neon HUD Native'
        settings = 'Neon HUD · Settings'
    }
    if ($HoverEdges) { $expected.Remove('settings') }
    elseif ($SettingsPage -eq 0) { $expected.instruments = 'Neon HUD · Instruments' }
    do {
        Start-Sleep -Milliseconds 200
        $process.Refresh()
        if ($process.HasExited) { throw "Native GUI exited before window creation (code $($process.ExitCode))" }
        $visible = @([NativeHudCapture]::VisibleWindows([uint32]$process.Id))
        $titles = @($visible | ForEach-Object { $_.Title })
        $missing = @($expected.Values | Where-Object { $_ -notin $titles })
    } while ($missing.Count -gt 0 -and (Get-Date) -lt $deadline)
    if ($missing.Count -gt 0) {
        throw "Native GUI did not expose all expected windows within seven seconds: $($missing -join ', ')"
    }

    $captureAt = $started.AddSeconds(4)
    $remaining = [int][Math]::Ceiling(($captureAt - (Get-Date)).TotalMilliseconds)
    if ($remaining -gt 0) { Start-Sleep -Milliseconds $remaining }
    $process.Refresh()
    if ($process.HasExited) { throw "Native GUI exited before the four-second capture (code $($process.ExitCode))" }

    $visible = @([NativeHudCapture]::VisibleWindows([uint32]$process.Id))
    $captures = [System.Collections.Generic.List[object]]::new()
    foreach ($role in $expected.Keys) {
        $window = $visible | Where-Object { $_.Title -eq $expected[$role] } | Select-Object -First 1
        if ($null -eq $window) { throw "Native $role window disappeared before capture" }
        $screenshot = Join-Path $OutputDirectory "native-$role.png"
        $width = 0; $height = 0; $colors = 0
        $method = [NativeHudCapture]::Capture($window.Handle, $screenshot, $true, [ref]$width, [ref]$height, [ref]$colors)
        if ($colors -lt 3) { throw "Native $role window screenshot has only $colors sampled colors" }
        $captures.Add([pscustomobject]@{
            role = $role
            title = $window.Title
            screenshot = [IO.Path]::GetFileName($screenshot)
            rectangle = [pscustomobject]@{ left = $window.Rect.Left; top = $window.Rect.Top; right = $window.Rect.Right; bottom = $window.Rect.Bottom }
            captureMethod = $method
            width = $width
            height = $height
            minimumDistinctColorsObserved = $colors
        })
    }

    $hoverChecks = @()
    if ($HoverEdges) {
        $hudWindow = $visible | Where-Object { $_.Title -eq 'Neon HUD Native' } | Select-Object -First 1
        $monitor = [NativeHudCapture+MONITORINFO]::new()
        $monitor.Size = [Runtime.InteropServices.Marshal]::SizeOf($monitor)
        if (-not [NativeHudCapture]::GetMonitorInfo([NativeHudCapture]::MonitorFromWindow($hudWindow.Handle, 2), [ref]$monitor)) { throw 'Hover monitor unavailable' }
        $work = $monitor.Work
        $hudWidth = $hudWindow.Rect.Right - $hudWindow.Rect.Left
        $hudHeight = $hudWindow.Rect.Bottom - $hudWindow.Rect.Top
        $xs = @{ left=$work.Left+20; center=[int](($work.Left+$work.Right-$hudWidth)/2); right=$work.Right-$hudWidth-20 }
        $ys = @{ top=$work.Top+20; center=[int](($work.Top+$work.Bottom-$hudHeight)/2); bottom=$work.Bottom-$hudHeight-20 }
        foreach ($edge in @('top','bottom','left','right','top-left','top-right','bottom-left','bottom-right')) {
            $xKey = if ($edge.Contains('left')) { 'left' } elseif ($edge.Contains('right')) { 'right' } else { 'center' }
            $yKey = if ($edge.Contains('top')) { 'top' } elseif ($edge.Contains('bottom')) { 'bottom' } else { 'center' }
            if (-not [NativeHudCapture]::SetWindowPos($hudWindow.Handle, [IntPtr]::Zero, $xs[$xKey], $ys[$yKey], 0, 0, 0x15)) { throw 'Could not position CI HUD' }
            if (-not [NativeHudCapture]::SetCursorPos($xs[$xKey]+40, $ys[$yKey]+25)) { throw 'Could not hover CI HUD' }
            $hoverDeadline = (Get-Date).AddSeconds(2)
            do {
                Start-Sleep -Milliseconds 100
                $windows = @([NativeHudCapture]::VisibleWindows([uint32]$process.Id))
                $hud = $windows | Where-Object { $_.Title -eq 'Neon HUD Native' } | Select-Object -First 1
                $hover = $windows | Where-Object { $_.Title -eq 'Neon HUD reading' } | Select-Object -First 1
                $valid = $null -ne $hud -and $null -ne $hover
                if ($valid) {
                    if ($xKey -eq 'left') { $valid = $valid -and $hover.Rect.Left -ge $hud.Rect.Right+4 }
                    if ($xKey -eq 'right') { $valid = $valid -and $hover.Rect.Right -le $hud.Rect.Left-4 }
                    if ($yKey -eq 'top') { $valid = $valid -and $hover.Rect.Top -ge $hud.Rect.Bottom+4 }
                    if ($yKey -eq 'bottom') { $valid = $valid -and $hover.Rect.Bottom -le $hud.Rect.Top-4 }
                    $valid = $valid -and $hover.Rect.Left -ge $work.Left -and $hover.Rect.Top -ge $work.Top -and $hover.Rect.Right -le $work.Right -and $hover.Rect.Bottom -le $work.Bottom
                }
            } while (-not $valid -and (Get-Date) -lt $hoverDeadline)
            if (-not $valid) { throw "Hover did not open inward and remain visible at $edge" }
            $screenshot = Join-Path $OutputDirectory "native-hover-$edge.png"
            $width=0; $height=0; $colors=0
            $method = [NativeHudCapture]::Capture($hover.Handle, $screenshot, $false, [ref]$width, [ref]$height, [ref]$colors)
            if ($colors -lt 3) { throw "Hover $edge screenshot has only $colors sampled colors" }
            $hoverChecks += [pscustomobject]@{
                edge=$edge; screenshot=[IO.Path]::GetFileName($screenshot); captureMethod=$method
                hud=$hud.Rect; hover=$hover.Rect; opensInward=$valid; minimumDistinctColorsObserved=$colors
            }
        }
    }

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

    $exitDeadline = (Get-Date).AddSeconds(35)
    do {
        Start-Sleep -Milliseconds 200
        $process.Refresh()
    } while (-not $process.HasExited -and (Get-Date) -lt $exitDeadline)
    if (-not $process.HasExited) { throw 'Native --smoke did not autoexit within 35 seconds after capture' }
    if ($process.ExitCode -ne 0) { throw "Native --smoke exited with code $($process.ExitCode)" }

    $receipt = [ordered]@{
        executable = [IO.Path]::GetFileName($executablePath)
        argument = '--smoke'
        settingsPage = $SettingsPage
        virtualDesktop = [ordered]@{ x = [NativeHudCapture]::GetSystemMetrics(76); y = [NativeHudCapture]::GetSystemMetrics(77); width = [NativeHudCapture]::GetSystemMetrics(78); height = [NativeHudCapture]::GetSystemMetrics(79) }
        visibleWindowsAtCapture = @($visible | ForEach-Object { [pscustomobject]@{ title = $_.Title; rectangle = [pscustomobject]@{ left = $_.Rect.Left; top = $_.Rect.Top; right = $_.Rect.Right; bottom = $_.Rect.Bottom } } })
        captures = @($captures)
        hoverChecks = @($hoverChecks)
        processTreeAtCapture = @($tree)
        exitCode = $process.ExitCode
        strictSingleProcessProven = $false
    }
    $receipt | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $receiptPath
    Write-Output "Native GUI smoke passed: $($captures.Count) windows captured, exit $($process.ExitCode); receipt $receiptPath"
} catch {
    [ordered]@{
        result = 'failed'
        failure = $_.Exception.Message
        screenshots = @(Get-ChildItem -LiteralPath $OutputDirectory -Filter 'native-*.png' -File | Select-Object -ExpandProperty Name)
    } | ConvertTo-Json | Set-Content -LiteralPath $receiptPath
    throw
} finally {
    $process.Refresh()
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
}
