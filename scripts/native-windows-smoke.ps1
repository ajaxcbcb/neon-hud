param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [ValidateRange(0, 3)][int]$SettingsPage = 0,
    [switch]$HoverEdges,
    [switch]$ContextMenu,
    [switch]$QuitMenu,
    [switch]$Nook,
    [ValidateRange(-1, 10)][int]$NookTab = -1,
    [switch]$NookInteraction
)
$ErrorActionPreference = 'Stop'
if ($QuitMenu -and -not $ContextMenu) { throw 'QuitMenu requires ContextMenu' }
if (($NookTab -ge 0 -or $NookInteraction) -and -not $Nook) { throw 'NookTab and NookInteraction require Nook' }
if ($Nook -and ($HoverEdges -or $ContextMenu)) { throw 'Nook interaction has its own scope; legacy Pill checks remain separate' }
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
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
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
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT point);
    [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT point);
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr window, int command);
    [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr window, uint flags);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern bool GetMonitorInfo(IntPtr monitor, ref MONITORINFO info);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern short VkKeyScan(char character);
    [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)] public static extern uint ExtractIconEx(string file, int index, IntPtr[] large, IntPtr[] small, uint count);
    [DllImport("user32.dll")] public static extern bool DestroyIcon(IntPtr icon);

    public static void Click(bool secondary) {
        mouse_event(secondary ? 0x0008u : 0x0002u, 0, 0, 0, UIntPtr.Zero);
        System.Threading.Thread.Sleep(50);
        mouse_event(secondary ? 0x0010u : 0x0004u, 0, 0, 0, UIntPtr.Zero);
    }
    public static void SecondaryButton(bool down) {
        mouse_event(down ? 0x0008u : 0x0010u, 0, 0, 0, UIntPtr.Zero);
    }
    public static void Escape() {
        keybd_event(0x1B, 0, 0, UIntPtr.Zero);
        keybd_event(0x1B, 0, 2, UIntPtr.Zero);
    }
    public static void TypeText(string value) {
        foreach (char character in value) {
            short mapped = VkKeyScan(character);
            if (mapped == -1 || (mapped & 0xFE00) != 0) throw new InvalidOperationException("Unsupported smoke input character");
            bool shift = (mapped & 0x0100) != 0;
            if (shift) keybd_event(0x10, 0, 0, UIntPtr.Zero);
            keybd_event((byte)mapped, 0, 0, UIntPtr.Zero);
            keybd_event((byte)mapped, 0, 2, UIntPtr.Zero);
            if (shift) keybd_event(0x10, 0, 2, UIntPtr.Zero);
            System.Threading.Thread.Sleep(15);
        }
    }
    public static void Enter() {
        keybd_event(0x0D, 0, 0, UIntPtr.Zero);
        keybd_event(0x0D, 0, 2, UIntPtr.Zero);
    }
    public static bool ProgramIcon(string executable, string path) {
        var large = new IntPtr[1]; var small = new IntPtr[1];
        var count = ExtractIconEx(executable, 0, large, small, 1);
        try {
            if (count == 0 || large[0] == IntPtr.Zero || small[0] == IntPtr.Zero) return false;
            using (var icon = Icon.FromHandle(large[0]))
            using (var bitmap = icon.ToBitmap()) {
                if (CountColors(bitmap) < 3) return false;
                bitmap.Save(path, System.Drawing.Imaging.ImageFormat.Png);
            }
            return true;
        } finally {
            if (large[0] != IntPtr.Zero) DestroyIcon(large[0]);
            if (small[0] != IntPtr.Zero) DestroyIcon(small[0]);
        }
    }

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
if ($ContextMenu) { $arguments += '--smoke-menu' }
if ($Nook) { $arguments += '--smoke-nook' }
if ($NookTab -ge 0) { $arguments += "--smoke-nook-tab=$NookTab" }
if ($NookInteraction) { $arguments += '--smoke-nook-interaction' }
# This CI-only foreground capture requires visible windows for real cursor input.
$process = Start-Process -FilePath $executablePath -ArgumentList $arguments -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
try {
    $deadline = $started.AddSeconds(7)
    $expected = [ordered]@{
        hud = 'Neon HUD Native'
        settings = 'Neon HUD · Settings'
    }
    if ($HoverEdges -or $ContextMenu -or $Nook) { $expected.Remove('settings') }
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

    $hudWindow = $visible | Where-Object { $_.Title -eq 'Neon HUD Native' } | Select-Object -First 1
    $programIconVerified = [NativeHudCapture]::ProgramIcon($executablePath, (Join-Path $OutputDirectory 'native-program-icon.png'))
    if (-not $programIconVerified) { throw 'Executable has no extractable Neon HUD program icon' }
    $windowIconVerified = [NativeHudCapture]::SendMessage($hudWindow.Handle, 0x7F, [IntPtr]1, [IntPtr]::Zero) -ne [IntPtr]::Zero
    if (-not $windowIconVerified) { throw 'Native HUD window has no application icon' }

    $nookChecks = @()
    if ($NookInteraction) {
        function Wait-NookState([string]$Phase, [int]$Width, [int]$Height, [string]$Tab = 'Nook') {
            $stateDeadline = (Get-Date).AddSeconds(2)
            do {
                Start-Sleep -Milliseconds 60
                $process.Refresh()
                if ($process.HasExited) { throw 'Nook exited during physical interaction' }
                $window = [NativeHudCapture]::VisibleWindows([uint32]$process.Id) | Where-Object { $_.Title -eq 'Neon HUD Native' } | Select-Object -First 1
                $trace = Get-Content -LiteralPath $stderr -Raw
                $states = [regex]::Matches($trace, 'NEON_NOOK phase=\w+ tab=[^\r\n]+')
                $lastState = if ($states.Count -gt 0) { $states[$states.Count-1].Value } else { '' }
                $ready = $null -ne $window -and $window.Rect.Right-$window.Rect.Left -eq $Width -and $window.Rect.Bottom-$window.Rect.Top -eq $Height -and $lastState -eq "NEON_NOOK phase=$Phase tab=$Tab"
            } while (-not $ready -and (Get-Date) -lt $stateDeadline)
            if (-not $ready) { throw "Nook did not settle in $Phase / $Tab ($Width x $Height); last trace: $lastState" }
            return $window
        }
        function Save-NookFrame($Window, [string]$Name) {
            $width=0; $height=0; $colors=0
            $method = [NativeHudCapture]::Capture($Window.Handle, (Join-Path $OutputDirectory "native-nook-$Name.png"), $true, [ref]$width, [ref]$height, [ref]$colors)
            if ($colors -lt 3) { throw "Nook $Name screenshot is blank" }
            return [pscustomobject]@{ state=$Name; width=$width; height=$height; screenshot="native-nook-$Name.png"; captureMethod=$method; realPointerInput=$true }
        }
        $hudWindow = Wait-NookState 'collapsed' 240 40
        $nookChecks += Save-NookFrame $hudWindow 'collapsed'
        [NativeHudCapture]::SetCursorPos($hudWindow.Rect.Left+80, $hudWindow.Rect.Top+20) | Out-Null
        $hudWindow = Wait-NookState 'peek' 520 112
        $nookChecks += Save-NookFrame $hudWindow 'peek'
        [NativeHudCapture]::SetCursorPos($hudWindow.Rect.Left+80, $hudWindow.Rect.Top+20) | Out-Null
        [NativeHudCapture]::Click($false)
        $hudWindow = Wait-NookState 'expanded' 900 192
        $nookChecks += Save-NookFrame $hudWindow 'expanded'
        [NativeHudCapture]::SetCursorPos($hudWindow.Rect.Right-54, $hudWindow.Rect.Top+21) | Out-Null
        [NativeHudCapture]::Click($false)
        $hudWindow = Wait-NookState 'pinned' 900 192
        [NativeHudCapture]::SetCursorPos([NativeHudCapture]::GetSystemMetrics(76)+10, [NativeHudCapture]::GetSystemMetrics(77)+[NativeHudCapture]::GetSystemMetrics(79)-10) | Out-Null
        Start-Sleep -Milliseconds 750
        $hudWindow = Wait-NookState 'pinned' 900 192
        $nookChecks += Save-NookFrame $hudWindow 'pinned-outside'
        [NativeHudCapture]::Escape()
        $hudWindow = Wait-NookState 'collapsed' 240 40
        $nookChecks += Save-NookFrame $hudWindow 'escape'
        # Assert that Controls opens before secondary release. Opening on press
        # can prevent the hover morph; this does not claim a resize occurred.
        [NativeHudCapture]::SetCursorPos($hudWindow.Rect.Left+40, $hudWindow.Rect.Top+25) | Out-Null
        Start-Sleep -Milliseconds 50
        [NativeHudCapture]::SecondaryButton($true)
        try {
            $controlsDeadline = (Get-Date).AddSeconds(2)
            do {
                $controlsWindow = [NativeHudCapture]::VisibleWindows([uint32]$process.Id) | Where-Object { $_.Title -eq 'Neon HUD · Controls' } | Select-Object -First 1
                if ($null -ne $controlsWindow) { break }
                Start-Sleep -Milliseconds 80
            } while ((Get-Date) -lt $controlsDeadline)
            if ($null -eq $controlsWindow) { throw 'Nook Controls did not open before secondary release' }
            Start-Sleep -Milliseconds 500
            $nookChecks += Save-NookFrame $controlsWindow 'secondary-press-before-release'
        } finally {
            [NativeHudCapture]::SecondaryButton($false)
        }
        [NativeHudCapture]::Escape()
        [NativeHudCapture]::SetCursorPos([NativeHudCapture]::GetSystemMetrics(76)+10, [NativeHudCapture]::GetSystemMetrics(77)+[NativeHudCapture]::GetSystemMetrics(79)-10) | Out-Null
        [NativeHudCapture]::SetForegroundWindow($hudWindow.Handle) | Out-Null
        $hudWindow = Wait-NookState 'collapsed' 240 40
        [NativeHudCapture]::SetCursorPos($hudWindow.Rect.Left+80, $hudWindow.Rect.Top+20) | Out-Null
        [NativeHudCapture]::Click($false)
        $hudWindow = Wait-NookState 'expanded' 900 192
        [NativeHudCapture]::SetCursorPos($hudWindow.Rect.Right-54, $hudWindow.Rect.Top+21) | Out-Null
        [NativeHudCapture]::Click($false)
        $hudWindow = Wait-NookState 'pinned' 900 192
        $tabs = @('Nook','Tray','Instruments','AI','Notes','Tasks','Timer','Media','Calendar','Mirror','Quick actions')
        $tabCenters = @(54,114,175,213,251,289,327,365,403,441,479)
        foreach ($tabIndex in 0..10) {
            [NativeHudCapture]::SetCursorPos($hudWindow.Rect.Left+$tabCenters[$tabIndex], $hudWindow.Rect.Top+21) | Out-Null
            Start-Sleep -Milliseconds 80
            [NativeHudCapture]::Click($false)
            $hudWindow = Wait-NookState 'pinned' 900 192 $tabs[$tabIndex]
            Start-Sleep -Milliseconds 200
            $nookChecks += Save-NookFrame $hudWindow ($tabs[$tabIndex].ToLowerInvariant().Replace(' ','-'))
            if ($tabs[$tabIndex] -eq 'Notes' -or $tabs[$tabIndex] -eq 'Tasks') {
                [NativeHudCapture]::SetCursorPos($hudWindow.Rect.Left+45, $hudWindow.Rect.Top+61) | Out-Null
                Start-Sleep -Milliseconds 80
                [NativeHudCapture]::Click($false)
                [NativeHudCapture]::TypeText($(if ($tabs[$tabIndex] -eq 'Notes') { 'Nook cloud note' } else { 'Nook cloud task' }))
                if ($tabs[$tabIndex] -eq 'Tasks') { [NativeHudCapture]::Enter() }
                Start-Sleep -Milliseconds 700
            }
            if ($tabs[$tabIndex] -eq 'Timer') {
                [NativeHudCapture]::SetCursorPos($hudWindow.Rect.Right-31, $hudWindow.Rect.Top+85) | Out-Null
                Start-Sleep -Milliseconds 80
                [NativeHudCapture]::Click($false)
                Start-Sleep -Milliseconds 700
                $nookChecks += Save-NookFrame $hudWindow 'timer-running'
            }
        }
        $utilityPath = Join-Path ([IO.Path]::GetTempPath()) "neon-native-smoke-$($process.Id)/nook-productivity.json"
        $utilityState = Get-Content -LiteralPath $utilityPath -Raw | ConvertFrom-Json
        if ($utilityState.note -ne 'Nook cloud note' -or @($utilityState.tasks | Where-Object text -eq 'Nook cloud task').Count -ne 1 -or $utilityState.timer.status -ne 'running') {
            throw 'Physical Nook note/task/timer actions were not retained in the saved utility state'
        }
        $nookChecks += [pscustomobject]@{ state='saved-controls'; realPointerInput=$true; realKeyboardInput=$true; noteRetained=$true; taskRetained=$true; timerRunning=$true }
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

    $menuChecks = @()
    if ($ContextMenu) {
        $monitor = [NativeHudCapture+MONITORINFO]::new()
        $monitor.Size = [Runtime.InteropServices.Marshal]::SizeOf($monitor)
        if (-not [NativeHudCapture]::GetMonitorInfo([NativeHudCapture]::MonitorFromWindow($hudWindow.Handle, 2), [ref]$monitor)) { throw 'Controls monitor unavailable' }
        $work = $monitor.Work
        $hudWidth = $hudWindow.Rect.Right - $hudWindow.Rect.Left
        $hudHeight = $hudWindow.Rect.Bottom - $hudWindow.Rect.Top
        $xs = @{ left=$work.Left+20; center=[int](($work.Left+$work.Right-$hudWidth)/2); right=$work.Right-$hudWidth-20 }
        $ys = @{ top=$work.Top+20; center=[int](($work.Top+$work.Bottom-$hudHeight)/2); bottom=$work.Bottom-$hudHeight-20 }
        $menuEdges = if ($QuitMenu) { @('right') } else { @('top','bottom','left','right','top-left','top-right','bottom-left','bottom-right') }
        foreach ($edge in $menuEdges) {
            $xKey = if ($edge.Contains('left')) { 'left' } elseif ($edge.Contains('right')) { 'right' } else { 'center' }
            $yKey = if ($edge.Contains('top')) { 'top' } elseif ($edge.Contains('bottom')) { 'bottom' } else { 'center' }
            if (-not [NativeHudCapture]::SetWindowPos($hudWindow.Handle, [IntPtr]::Zero, $xs[$xKey], $ys[$yKey], 0, 0, 0x15)) { throw 'Could not position CI HUD for right click' }
            [NativeHudCapture]::SetForegroundWindow($hudWindow.Handle) | Out-Null
            # Allow Windows move/focus events to reach egui before delivering input.
            Start-Sleep -Milliseconds 300
            # Exercise metric, grip and background areas, rather than a synthetic menu command.
            $hitX = if ($edge -eq 'top') { 10 } elseif ($edge -eq 'bottom') { 140 } else { 42 }
            $hitY = if ($edge -eq 'bottom') { 3 } else { 25 }
            if (-not [NativeHudCapture]::SetCursorPos($xs[$xKey]+$hitX, $ys[$yKey]+$hitY)) { throw 'Could not move to CI right-click target' }
            Start-Sleep -Milliseconds 200
            $currentHud = [NativeHudCapture]::VisibleWindows([uint32]$process.Id) | Where-Object { $_.Handle -eq $hudWindow.Handle } | Select-Object -First 1
            $cursor = [NativeHudCapture+POINT]::new()
            if ($null -eq $currentHud -or -not [NativeHudCapture]::GetCursorPos([ref]$cursor)) { throw 'CI right-click target is unavailable' }
            if ($currentHud.Rect.Left -ne $xs[$xKey] -or $currentHud.Rect.Top -ne $ys[$yKey] -or [NativeHudCapture]::GetForegroundWindow() -ne $hudWindow.Handle -or [NativeHudCapture]::WindowFromPoint($cursor) -ne $hudWindow.Handle) { throw "CI right-click target was not ready at $edge" }
            [NativeHudCapture]::Click($true)
            $menuDeadline = (Get-Date).AddSeconds(2)
            do {
                Start-Sleep -Milliseconds 80
                $menus = @([NativeHudCapture]::VisibleWindows([uint32]$process.Id) | Where-Object { $_.Title -eq 'Neon HUD · Controls' })
                if ($menus.Count -gt 1) { throw "Duplicate Controls windows at $edge" }
                $menu = if ($menus.Count -eq 1) { $menus[0] } else { $null }
                $safe = $false
                if ($null -ne $menu) {
                    $safe = $menu.Rect.Left -ge $work.Left -and $menu.Rect.Top -ge $work.Top -and $menu.Rect.Right -le $work.Right -and $menu.Rect.Bottom -le $work.Bottom
                    if ($xKey -eq 'left') { $safe = $safe -and $menu.Rect.Left -ge $xs[$xKey]+$hudWidth+4 }
                    if ($xKey -eq 'right') { $safe = $safe -and $menu.Rect.Right -le $xs[$xKey]-4 }
                    if ($yKey -eq 'top') { $safe = $safe -and $menu.Rect.Top -ge $ys[$yKey]+$hudHeight+4 }
                    if ($yKey -eq 'bottom') { $safe = $safe -and $menu.Rect.Bottom -le $ys[$yKey]-4 }
                }
            } while (-not $safe -and (Get-Date) -lt $menuDeadline)
            if ($null -eq $menu) { throw "Real right click did not open controls at $edge" }
            if (-not $safe) { throw "Controls were clipped or opened outward at $edge" }
            $width=0; $height=0; $colors=0
            $screenshot = Join-Path $OutputDirectory "native-controls-$edge.png"
            $method = [NativeHudCapture]::Capture($menu.Handle, $screenshot, $true, [ref]$width, [ref]$height, [ref]$colors)
            if ($colors -lt 3) { throw 'Controls screenshot is blank' }
            [NativeHudCapture]::Escape()
            $dismissDeadline = (Get-Date).AddSeconds(2)
            do {
                Start-Sleep -Milliseconds 80
                $remaining = @([NativeHudCapture]::VisibleWindows([uint32]$process.Id) | Where-Object { $_.Title -eq 'Neon HUD · Controls' })
            } while ($remaining.Count -gt 0 -and (Get-Date) -lt $dismissDeadline)
            if ($remaining.Count -gt 0) { throw 'Escape did not dismiss controls' }
            $menuChecks += [pscustomobject]@{ edge=$edge; opensInward=$safe; escapeDismissed=$true; screenshot=[IO.Path]::GetFileName($screenshot); captureMethod=$method }
        }
        # Verify real menu actions without bypassing the click handlers.
        $actions = if ($QuitMenu) { @('quit') } else { @('expand','compress','pause','resume','reset','settings','hide') }
        foreach ($action in $actions) {
            $hudWindow = [NativeHudCapture]::VisibleWindows([uint32]$process.Id) | Where-Object { $_.Title -eq 'Neon HUD Native' } | Select-Object -First 1
            if ($null -eq $hudWindow) { throw "HUD disappeared before $action" }
            [NativeHudCapture]::SetForegroundWindow($hudWindow.Handle) | Out-Null
            Start-Sleep -Milliseconds 200
            [NativeHudCapture]::SetCursorPos($hudWindow.Rect.Left+40, $hudWindow.Rect.Top+25) | Out-Null
            [NativeHudCapture]::Click($true)
            $actionDeadline = (Get-Date).AddSeconds(2)
            do {
                Start-Sleep -Milliseconds 80
                $menu = [NativeHudCapture]::VisibleWindows([uint32]$process.Id) | Where-Object { $_.Title -eq 'Neon HUD · Controls' } | Select-Object -First 1
            } while ($null -eq $menu -and (Get-Date) -lt $actionDeadline)
            if ($null -eq $menu) { throw "Controls did not reopen for $action" }
            # A visible HWND can precede its first egui frame and final position.
            # Capture the real rendered menu, then read its settled coordinates
            # before physical input. Keep the action assertions below unchanged.
            $width=0; $height=0; $colors=0
            [NativeHudCapture]::Capture($menu.Handle, (Join-Path $OutputDirectory "native-controls-before-$action.png"), $true, [ref]$width, [ref]$height, [ref]$colors) | Out-Null
            if ($colors -lt 3) { throw "Controls were not rendered before $action" }
            $menu = [NativeHudCapture]::VisibleWindows([uint32]$process.Id) | Where-Object { $_.Handle -eq $menu.Handle } | Select-Object -First 1
            if ($null -eq $menu) { throw "Controls disappeared before $action input" }
            $row = switch ($action) {
                'settings' { 0 }
                { $_ -in 'compress','expand' } { 1 }
                { $_ -in 'pause','resume' } { 2 }
                'hide' { 3 }
                'reset' { 4 }
                'quit' { 5 }
            }
            [NativeHudCapture]::SetCursorPos($menu.Rect.Left+120, $menu.Rect.Top+58+36*$row) | Out-Null
            Start-Sleep -Milliseconds 150
            $cursor = [NativeHudCapture+POINT]::new()
            if (-not [NativeHudCapture]::GetCursorPos([ref]$cursor) -or [NativeHudCapture]::WindowFromPoint($cursor) -ne $menu.Handle -or [NativeHudCapture]::GetForegroundWindow() -ne $menu.Handle) { throw "Controls pointer/focus was not ready for $action" }
            [NativeHudCapture]::Click($false)
            $actionDeadline = (Get-Date).AddSeconds(2)
            do {
                Start-Sleep -Milliseconds 80
                $windows = @([NativeHudCapture]::VisibleWindows([uint32]$process.Id))
                $hudWindow = $windows | Where-Object { $_.Title -eq 'Neon HUD Native' } | Select-Object -First 1
                $settingsWindow = $windows | Where-Object { $_.Title -eq 'Neon HUD · Settings' } | Select-Object -First 1
                $trace = if (Test-Path -LiteralPath $stderr) { Get-Content -LiteralPath $stderr -Raw } else { '' }
                $process.Refresh()
                $passed = switch ($action) {
                    'settings' { $null -ne $settingsWindow }
                    'compress' { $null -ne $hudWindow -and $hudWindow.Rect.Right-$hudWindow.Rect.Left -eq 160 }
                    'expand' { $null -ne $hudWindow -and $hudWindow.Rect.Right-$hudWindow.Rect.Left -eq 280 }
                    'pause' { $trace -match 'NEON_CONTROL index=2 paused=true hidden=false' }
                    'resume' { $trace -match 'NEON_CONTROL index=2 paused=false hidden=false' }
                    'reset' { $null -ne $hudWindow -and $hudWindow.Rect.Left -eq 60 -and $hudWindow.Rect.Top -eq 60 }
                    'hide' { $null -eq $hudWindow -and $trace -match 'NEON_CONTROL index=3 paused=false hidden=true' }
                    'quit' { $process.HasExited -and $process.ExitCode -eq 0 -and $trace -match 'NEON_CONTROL index=5 paused=false hidden=false' }
                }
                $passed = $passed -and @($windows | Where-Object { $_.Title -eq 'Neon HUD · Controls' }).Count -eq 0
            } while (-not $passed -and (Get-Date) -lt $actionDeadline)
            if (-not $passed) { throw "Right-click $action action failed" }
            $menuChecks += [pscustomobject]@{ action=$action; passed=$true }
            if ($action -eq 'settings') {
                $width=0; $height=0; $colors=0
                [NativeHudCapture]::Capture($settingsWindow.Handle, (Join-Path $OutputDirectory 'native-controls-settings.png'), $true, [ref]$width, [ref]$height, [ref]$colors) | Out-Null
                if ($colors -lt 3) { throw 'Settings opened from right click is blank' }
                [NativeHudCapture]::SetCursorPos($settingsWindow.Rect.Right-27, $settingsWindow.Rect.Top+29) | Out-Null
                [NativeHudCapture]::Click($false)
                $closeDeadline = (Get-Date).AddSeconds(2)
                do {
                    Start-Sleep -Milliseconds 80
                    $settings = @([NativeHudCapture]::VisibleWindows([uint32]$process.Id) | Where-Object { $_.Title -eq 'Neon HUD · Settings' })
                } while ($settings.Count -gt 0 -and (Get-Date) -lt $closeDeadline)
                if ($settings.Count -gt 0) { throw 'Could not close CI Settings before testing Hide' }
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
    $hiddenLifecycle = $null
    if ($ContextMenu -and -not $QuitMenu) {
        $trace = Get-Content -LiteralPath $stderr -Raw
        $hiddenTicks = @([regex]::Matches($trace, 'NEON_HIDDEN_TICK elapsedMs=(\d+)') | ForEach-Object { [long]$_.Groups[1].Value })
        if ($hiddenTicks.Count -lt 2 -or $hiddenTicks.Count -gt 100) { throw 'Hidden callbacks were absent or unbounded' }
        $hiddenSpan = $hiddenTicks[-1]-$hiddenTicks[0]
        if ($hiddenSpan -lt 1000) { throw 'Hidden background callbacks did not continue after Hide' }
        $hiddenLifecycle = [ordered]@{ callbacks=$hiddenTicks.Count; spanMs=$hiddenSpan; normalExit=$true }
    }

    $receipt = [ordered]@{
        executable = [IO.Path]::GetFileName($executablePath)
        argument = '--smoke'
        settingsPage = $SettingsPage
        virtualDesktop = [ordered]@{ x = [NativeHudCapture]::GetSystemMetrics(76); y = [NativeHudCapture]::GetSystemMetrics(77); width = [NativeHudCapture]::GetSystemMetrics(78); height = [NativeHudCapture]::GetSystemMetrics(79) }
        visibleWindowsAtCapture = @($visible | ForEach-Object { [pscustomobject]@{ title = $_.Title; rectangle = [pscustomobject]@{ left = $_.Rect.Left; top = $_.Rect.Top; right = $_.Rect.Right; bottom = $_.Rect.Bottom } } })
        captures = @($captures)
        hoverChecks = @($hoverChecks)
        menuChecks = @($menuChecks)
        nookChecks = @($nookChecks)
        nookTab = $NookTab
        hiddenLifecycle = $hiddenLifecycle
        programIconVerified = $programIconVerified
        windowIconVerified = $windowIconVerified
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
        elapsedSeconds = [math]::Round(((Get-Date)-$started).TotalSeconds, 2)
        processExited = $process.HasExited
        successfulMenuEdges = @($menuChecks)
        visibleWindows = @([NativeHudCapture]::VisibleWindows([uint32]$process.Id) | ForEach-Object { [pscustomobject]@{ title=$_.Title; handle=$_.Handle.ToInt64(); rectangle=[pscustomobject]@{ left=$_.Rect.Left; top=$_.Rect.Top; right=$_.Rect.Right; bottom=$_.Rect.Bottom } } })
        foregroundWindow = [NativeHudCapture]::GetForegroundWindow().ToInt64()
        screenshots = @(Get-ChildItem -LiteralPath $OutputDirectory -Filter 'native-*.png' -File | Select-Object -ExpandProperty Name)
    } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $receiptPath
    throw
} finally {
    $process.Refresh()
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
}
