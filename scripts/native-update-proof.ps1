param(
    [ValidatePattern('^v0\.2\.0-alpha\.[0-9]+$')][string]$FromTag = 'v0.2.0-alpha.3',
    [ValidatePattern('^v0\.2\.0-alpha\.[0-9]+$')][string]$ToTag = 'v0.2.0-alpha.4',
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [Parameter(Mandatory = $true)][string]$MesaDirectory
)
$ErrorActionPreference = 'Stop'
# This test uses a fresh hosted Windows account, never a developer/user profile.
if ($env:GITHUB_ACTIONS -ne 'true' -or -not $IsWindows) { throw 'Update proof requires a fresh Windows GitHub Actions runner' }
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
$tempPrefix = [IO.Path]::GetFullPath($env:RUNNER_TEMP).TrimEnd('\') + '\'
if (-not $outputPath.StartsWith($tempPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Proof output must be inside RUNNER_TEMP' }
if (Test-Path -LiteralPath $outputPath) { throw 'Proof output must be new' }
$originalProfile = Join-Path $env:APPDATA 'io.github.ajaxcbcb.neonhud'
if (Test-Path -LiteralPath $originalProfile) { throw 'Runner already has a HUD profile; refusing to change it' }
New-Item -ItemType Directory -Path $outputPath | Out-Null
$profilePath = Join-Path $originalProfile 'native-preview'
New-Item -ItemType Directory -Path $profilePath | Out-Null
$receiptPath = Join-Path $outputPath 'receipt.json'
$receipt = [ordered]@{ result = 'pending'; from = $FromTag; to = $ToTag; sourceCommit = $env:GITHUB_SHA }
$cliIndex = 0

function Invoke-HudCli([string]$Exe, [string[]]$Arguments, [switch]$Reject) {
    $script:cliIndex++
    $stdout = Join-Path $outputPath "cli-$script:cliIndex.stdout.log"
    $stderr = Join-Path $outputPath "cli-$script:cliIndex.stderr.log"
    foreach ($arg in $Arguments) { if ($arg.Contains('"') -or $arg.Contains("`n")) { throw 'Invalid CLI argument' } }
    $quoted = @($Arguments | ForEach-Object { '"' + $_ + '"' }) -join ' '
    $process = Start-Process -FilePath $Exe -ArgumentList $quoted -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    if (-not $process.WaitForExit(120000)) { throw 'Native CLI timed out; no process was forcibly stopped' }
    $process.Refresh()
    if ($Reject) {
        if ($process.ExitCode -eq 0) { throw 'Tampered release was accepted' }
        return [pscustomobject]@{ rejected = $true; exitCode = $process.ExitCode }
    }
    if ($process.ExitCode -ne 0) { throw "Native CLI failed: $(Get-Content -LiteralPath $stderr -Raw)" }
    return (Get-Content -LiteralPath $stdout -Raw | ConvertFrom-Json)
}
function Download-Release([string]$Tag, [string]$Name) {
    $dir = Join-Path $outputPath $Name
    New-Item -ItemType Directory -Path $dir | Out-Null
    & gh release download $Tag --repo ajaxcbcb/neon-hud --dir $dir --pattern 'native-update.json' --pattern 'native-update.json.sig' --pattern 'neon-hud-native-windows-x64-*.zip'
    if ($LASTEXITCODE -ne 0) { throw "Could not download public release $Tag" }
    $manifest = Get-Content -LiteralPath (Join-Path $dir 'native-update.json') -Raw | ConvertFrom-Json
    if ('v' + $manifest.version -ne $Tag) { throw 'Release/manifest version mismatch' }
    $zips = @(Get-ChildItem -LiteralPath $dir -Filter '*.zip' -File)
    if ($zips.Count -ne 1) { throw 'Expected one native Windows archive' }
    $payload = Join-Path $dir 'payload'
    Expand-Archive -LiteralPath $zips[0].FullName -DestinationPath $payload
    $executables = @(Get-ChildItem -LiteralPath $payload -Filter 'neon-hud-native.exe' -Recurse -File)
    if ($executables.Count -ne 1) { throw 'Expected one packaged native executable' }
    return [pscustomobject]@{ manifest = (Join-Path $dir 'native-update.json'); signature = (Join-Path $dir 'native-update.json.sig'); archive = $zips[0].FullName; executable = $executables[0].FullName; metadata = $manifest }
}
function Startup-Value {
    try { return (Get-ItemPropertyValue -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'Neon HUD Native Preview' -ErrorAction Stop) } catch { return $null }
}

$drawingReferences = @(Get-ChildItem -LiteralPath (Join-Path $PSHOME 'ref') -Filter '*.dll' -File | Select-Object -ExpandProperty FullName) + @([System.Drawing.Bitmap].Assembly.Location) + @(Get-ChildItem -LiteralPath $PSHOME -Filter 'System.Private.Windows.*.dll' -File | Select-Object -ExpandProperty FullName)
Add-Type -ReferencedAssemblies $drawingReferences @'
using System;
using System.Collections.Generic;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Text;
public static class UpdateProofDesktop {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    public class Window { public IntPtr Handle; public string Title; public RECT Rect; }
    public delegate bool Callback(IntPtr handle, IntPtr param);
    [DllImport("user32.dll")] static extern bool EnumWindows(Callback callback, IntPtr param);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr handle);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr handle, out uint pid);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr handle, out RECT rect);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr handle, StringBuilder text, int capacity);
    [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr handle);
    [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extra);
    public static List<Window> Visible(uint pid) {
        var result = new List<Window>();
        Callback callback = (handle, param) => {
            uint owner; GetWindowThreadProcessId(handle, out owner);
            if (owner != pid || !IsWindowVisible(handle)) return true;
            RECT rect; if (!GetWindowRect(handle, out rect)) return true;
            var text = new StringBuilder(256); GetWindowText(handle, text, text.Capacity);
            result.Add(new Window { Handle=handle, Title=text.ToString(), Rect=rect }); return true;
        };
        if (!EnumWindows(callback, IntPtr.Zero)) throw new Exception("Window enumeration failed");
        return result;
    }
    public static void Click(Window window, int x, int y, bool right) {
        SetForegroundWindow(window.Handle); System.Threading.Thread.Sleep(300);
        if (GetForegroundWindow() != window.Handle) throw new Exception("Own HUD window did not receive focus");
        if (!SetCursorPos(window.Rect.Left+x, window.Rect.Top+y)) throw new Exception("Cursor positioning failed");
        System.Threading.Thread.Sleep(80);
        mouse_event(right ? 8u : 2u, 0, 0, 0, UIntPtr.Zero);
        System.Threading.Thread.Sleep(100);
        mouse_event(right ? 16u : 4u, 0, 0, 0, UIntPtr.Zero);
    }
    public static void Capture(Window window, string path) {
        int width=window.Rect.Right-window.Rect.Left, height=window.Rect.Bottom-window.Rect.Top;
        using (var bitmap = new Bitmap(width, height)) {
            using (var graphics=Graphics.FromImage(bitmap)) graphics.CopyFromScreen(window.Rect.Left, window.Rect.Top, 0, 0, bitmap.Size);
            var colors=new HashSet<int>();
            for (int y=0;y<height;y+=3) for(int x=0;x<width;x+=3) colors.Add(bitmap.GetPixel(x,y).ToArgb());
            if(colors.Count<3) throw new Exception("Blank native capture");
            bitmap.Save(path, System.Drawing.Imaging.ImageFormat.Png);
        }
    }
    public static void CaptureIcon(string exe, string path) {
        using(var icon=Icon.ExtractAssociatedIcon(exe)) {
            if(icon==null) throw new Exception("Missing program icon");
            using(var bitmap=icon.ToBitmap()) bitmap.Save(path, System.Drawing.Imaging.ImageFormat.Png);
        }
    }
}
'@
function Wait-Window([uint32]$PidValue, [string]$Title) {
    $deadline = (Get-Date).AddSeconds(6)
    do {
        $window = [UpdateProofDesktop]::Visible($PidValue) | Where-Object { $_.Title -eq $Title } | Select-Object -First 1
        if ($null -ne $window) { return $window }
        Start-Sleep -Milliseconds 100
    } while ((Get-Date) -lt $deadline)
    throw "Own application window did not appear: $Title"
}
function Open-Controls([uint32]$PidValue) {
    $hud = Wait-Window $PidValue 'Neon HUD Native'
    [UpdateProofDesktop]::Click($hud, 40, 25, $true)
    return (Wait-Window $PidValue 'Neon HUD · Controls')
}

try {
    $before = Download-Release $FromTag 'from'
    $after = Download-Release $ToTag 'to'
    $installedExe = $before.executable
    $oldHash = (Get-FileHash -LiteralPath $installedExe -Algorithm SHA256).Hash.ToLowerInvariant()
    $newHash = (Get-FileHash -LiteralPath $after.executable -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($oldHash -eq $newHash) { throw 'Update proof requires distinct published binaries' }
    foreach ($release in @($before, $after)) {
        $verified = Invoke-HudCli $installedExe @('--verify-native-manifest', $release.manifest, $release.signature)
        if (-not $verified.verified) { throw 'Manifest was not verified' }
        $verified = Invoke-HudCli $installedExe @('--verify-native-archive', $release.manifest, $release.signature, $release.archive)
        if (-not $verified.verified) { throw 'Archive was not verified' }
    }
    $receipt.signedReleasesVerified = $true
    $badManifest = Join-Path $outputPath 'tampered-manifest.json'
    $bytes = [IO.File]::ReadAllBytes($after.manifest); $bytes[16] = $bytes[16] -bxor 1
    [IO.File]::WriteAllBytes($badManifest, $bytes)
    $receipt.tamperedManifest = Invoke-HudCli $installedExe @('--verify-native-manifest', $badManifest, $after.signature) -Reject
    $badArchive = Join-Path $outputPath 'tampered-archive.zip'
    Copy-Item -LiteralPath $after.archive -Destination $badArchive
    $stream = [IO.File]::Open($badArchive, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite)
    try { $value = $stream.ReadByte(); $stream.Position = 0; $stream.WriteByte($value -bxor 1) } finally { $stream.Dispose() }
    $receipt.tamperedArchive = Invoke-HudCli $installedExe @('--verify-native-archive', $after.manifest, $after.signature, $badArchive) -Reject
    # Only synthetic preferences are created; provider connections stay disabled.
    $seed = [ordered]@{ completed=$true; step=3; theme='aurora'; motion='quiet'; size='compressed'; corner='middle-right'; windowPosition=@{x=80;y=120;monitor=$null}; launchAtLogin=$false; codexEnabled=$false; storageDriveIds=@('proof-drive'); resources=@{adaptive=$true;samplingMs=1000} }
    $settingsFile = Join-Path $profilePath 'settings.json'
    $seed | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $settingsFile -Encoding utf8NoBOM
    $preferencesFile = Join-Path $profilePath 'updater-settings.json'
    '{"automaticChecks":false,"automaticDownloads":false}' | Set-Content -LiteralPath $preferencesFile -Encoding utf8NoBOM
    $sharedFile = Join-Path $originalProfile 'settings.json'
    '{"syntheticSharedProfile":"must-stay-unchanged"}' | Set-Content -LiteralPath $sharedFile -Encoding utf8NoBOM
    $sharedHash = (Get-FileHash -LiteralPath $sharedFile).Hash
    $preferencesHash = (Get-FileHash -LiteralPath $preferencesFile).Hash
    $startupBefore = Startup-Value
    $dllHashes = [ordered]@{}
    foreach ($name in @('opengl32.dll','libglapi.dll','libgallium_wgl.dll')) {
        $destination = Join-Path (Split-Path -Parent $installedExe) $name
        Copy-Item -LiteralPath (Join-Path $MesaDirectory $name) -Destination $destination
        $dllHashes[$name] = (Get-FileHash -LiteralPath $destination).Hash
    }
    $offer = Invoke-HudCli $installedExe @('--update-check')
    if ($offer.status -ne 'available' -or $offer.version -ne $after.metadata.version -or -not $offer.signed -or $offer.installedVersion -ne $before.metadata.version) { throw 'Live public channel did not offer the expected newer signed release' }
    $receipt.beforeCheck = $offer
    $install = Invoke-HudCli $installedExe @('--update-install')
    if ($install.status -ne 'restart_requested' -or $install.version -ne $after.metadata.version) { throw 'Update helper was not requested' }
    $deadline = (Get-Date).AddSeconds(65)
    do {
        Start-Sleep -Milliseconds 300
        $acks = @(Get-ChildItem -LiteralPath (Join-Path $profilePath 'native-updates') -Filter 'startup.ack' -File -Recurse)
        $sameHash = (Test-Path -LiteralPath $installedExe) -and (Get-FileHash -LiteralPath $installedExe -Algorithm SHA256).Hash.ToLowerInvariant() -eq $newHash
    } while (($acks.Count -ne 1 -or -not $sameHash) -and (Get-Date) -lt $deadline)
    if ($acks.Count -ne 1 -or -not $sameHash) { throw 'Replacement app did not acknowledge startup with the exact published binary' }
    $ticketFile = Join-Path $acks[0].DirectoryName 'ticket.json'
    $ticket = Get-Content -LiteralPath $ticketFile -Raw | ConvertFrom-Json
    if ($ticket.old_sha256 -ne $oldHash -or $ticket.version -ne $after.metadata.version) { throw 'Update ticket does not match the tested versions' }
    $guiProcesses = @(Get-CimInstance Win32_Process -Filter "Name = 'neon-hud-native.exe'" | Where-Object { $_.ExecutablePath -eq $installedExe })
    if ($guiProcesses.Count -ne 1) { throw 'Expected exactly one updated native GUI' }
    $guiPid = [uint32]$guiProcesses[0].ProcessId
    $hud = Wait-Window $guiPid 'Neon HUD Native'
    Start-Sleep -Milliseconds 500
    [UpdateProofDesktop]::Capture($hud, (Join-Path $outputPath 'updated-hud.png'))
    [UpdateProofDesktop]::CaptureIcon($installedExe, (Join-Path $outputPath 'program-icon.png'))
    $menu = Open-Controls $guiPid
    [UpdateProofDesktop]::Capture($menu, (Join-Path $outputPath 'updated-right-click.png'))
    [UpdateProofDesktop]::Click($menu, 120, 58, $false)
    $settings = Wait-Window $guiPid 'Neon HUD · Settings'
    Start-Sleep -Milliseconds 300
    [UpdateProofDesktop]::Capture($settings, (Join-Path $outputPath 'updated-settings.png'))
    # Positions are from the native layout and its hosted screenshot, not a web UI.
    [UpdateProofDesktop]::Click($settings, 433, 97, $false)
    [UpdateProofDesktop]::Click($settings, 170, 179, $false)
    [UpdateProofDesktop]::Capture($settings, (Join-Path $outputPath 'update-settings-before-check.png'))
    [UpdateProofDesktop]::Click($settings, 63, 373, $false)
    Start-Sleep -Seconds 15
    [UpdateProofDesktop]::Capture($settings, (Join-Path $outputPath 'update-settings-after-check.png'))
    $receipt.guiCheckButtonClicked = $true
    [UpdateProofDesktop]::Click($settings, ($settings.Rect.Right-$settings.Rect.Left-27), 29, $false)
    Start-Sleep -Milliseconds 300
    $current = Invoke-HudCli $installedExe @('--update-check')
    if ($current.status -ne 'current' -or $current.version -ne $after.metadata.version) { throw 'Updated binary did not report the current release' }
    $receipt.afterCheck = $current
    $menu = Open-Controls $guiPid
    [UpdateProofDesktop]::Click($menu, 120, (58+36*5), $false)
    $guiProcess = Get-Process -Id $guiPid -ErrorAction SilentlyContinue
    if ($null -ne $guiProcess -and -not $guiProcess.WaitForExit(15000)) { throw 'Right-click Quit did not close the updated application normally' }
    $saved = Get-Content -LiteralPath $settingsFile -Raw | ConvertFrom-Json
    foreach ($name in @('completed','theme','motion','size','launchAtLogin','codexEnabled')) { if ($saved.$name -ne $seed[$name]) { throw "Preference changed during update: $name" } }
    if (@($saved.storageDriveIds).Count -ne 1 -or $saved.storageDriveIds[0] -ne 'proof-drive' -or $saved.resources.samplingMs -ne 1000 -or -not $saved.resources.adaptive) { throw 'Drive/resource preferences changed during update' }
    if ($saved.windowPosition.x -ne 80 -or $saved.windowPosition.y -ne 120) { throw 'Saved HUD position changed during update' }
    if ((Get-FileHash -LiteralPath $sharedFile).Hash -ne $sharedHash -or (Get-FileHash -LiteralPath $preferencesFile).Hash -ne $preferencesHash -or (Startup-Value) -ne $startupBefore) { throw 'Shared profile, update preferences or startup setting changed' }
    foreach ($name in $dllHashes.Keys) { if ((Get-FileHash -LiteralPath (Join-Path (Split-Path -Parent $installedExe) $name)).Hash -ne $dllHashes[$name]) { throw 'Installer modified an unrelated DLL' } }
    $backups = @(Get-ChildItem -LiteralPath (Split-Path -Parent $installedExe) -File | Where-Object { $_.Name -like '*previous*' -and (Get-FileHash -LiteralPath $_.FullName).Hash.ToLowerInvariant() -eq $oldHash })
    if ($backups.Count -ne 1) { throw 'Exact previous binary backup was not retained' }
    if (Test-Path -LiteralPath (Join-Path $profilePath 'native-updates/last-update-error.txt')) { throw 'Updater left a failure notice' }
    $receipt.result = 'passed'
    $receipt.binaryBefore = $oldHash; $receipt.binaryAfter = $newHash
    $receipt.startupAcknowledged = $true; $receipt.profileRetained = $true; $receipt.sharedProfileRetained = $true
    $receipt.updatePreferencesRetained = $true; $receipt.startupPreferenceRetained = $true; $receipt.previousBinaryRetained = $true
    $receipt.rightClickSettingsAndQuit = $true; $receipt.unrelatedDllsRetained = $true; $receipt.updatedGuiCount = 1
} catch {
    $receipt.result = 'failed'; $receipt.failure = $_.Exception.Message
    throw
} finally {
    $receipt | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $receiptPath -Encoding utf8NoBOM
}
