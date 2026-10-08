[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Executable,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$SourceCommit,
    [switch]$CleanupOnly
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Never create keys or alter trust on a user's PC or a self-hosted runner.
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_OS -ne 'Windows' -or
    $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or $PSVersionTable.PSVersion.Major -lt 7 -or
    -not $env:RUNNER_TEMP -or -not $env:GITHUB_WORKSPACE -or $env:GITHUB_RUN_ID -notmatch '^\d+$') {
    throw 'Development signing is restricted to a disposable GitHub-hosted Windows runner with PowerShell 7.'
}
if ($SourceCommit -ne $env:GITHUB_SHA) { throw 'Source commit differs from the accepted checkout.' }
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
try {
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'An already elevated hosted runner is required; elevation must not be requested.'
    }
}
finally { $identity.Dispose() }

$workspace = [IO.Path]::GetFullPath($env:GITHUB_WORKSPACE).TrimEnd('\') + '\'
$temporaryRoot = [IO.Path]::GetFullPath($env:RUNNER_TEMP).TrimEnd('\') + '\'
$exe = if ([IO.Path]::IsPathRooted($Executable)) { [IO.Path]::GetFullPath($Executable) }
    else { [IO.Path]::GetFullPath((Join-Path $workspace $Executable)) }
$output = [IO.Path]::GetFullPath($OutputDirectory)
if (-not $exe.StartsWith($workspace, [StringComparison]::OrdinalIgnoreCase) -or
    [IO.Path]::GetFileName($exe) -ne 'neon-hud-native.exe' -or
    -not $output.StartsWith($temporaryRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Signing inputs must be the checkout binary and a runner-temporary output.'
}

$diagnostics = Join-Path $output 'diagnostics'
$journalPath = Join-Path $diagnostics 'certificates.json'
$stagesPath = Join-Path $diagnostics 'stages.json'
$records = [Collections.Generic.List[object]]::new()
$stages = [Collections.Generic.List[object]]::new()
if ($CleanupOnly) {
    if (-not (Test-Path -LiteralPath $journalPath)) {
        Write-Host 'No development certificate journal exists; no trust or key mutation attempted.'
        return
    }
    foreach ($record in @(Get-Content -LiteralPath $journalPath -Raw | ConvertFrom-Json)) { $records.Add($record) }
    if (Test-Path -LiteralPath $stagesPath) {
        foreach ($stage in @(Get-Content -LiteralPath $stagesPath -Raw | ConvertFrom-Json)) { $stages.Add($stage) }
    }
}
else {
    if (Test-Path -LiteralPath $output) { throw 'Signing output must not already exist.' }
    New-Item -ItemType Directory -Path $diagnostics | Out-Null
}

function Write-Stage {
    param([string]$Stage, [string]$State, [object]$ExitCode = $null, [double]$DurationMs = 0)
    $record = [ordered]@{
        Stage = $Stage; State = $State; Utc = [DateTime]::UtcNow.ToString('o')
        ExitCode = $ExitCode; DurationMs = [Math]::Round($DurationMs, 1)
    }
    $stages.Add($record)
    ConvertTo-Json -InputObject $stages.ToArray() -Depth 5 |
        Set-Content -LiteralPath $stagesPath -Encoding utf8
    Write-Host ("[{0}] {1}: {2}; exit={3}; duration_ms={4}" -f
        $record.Utc, $Stage, $State, $ExitCode, $record.DurationMs)
}

function Invoke-BoundedTool {
    param(
        [string]$Stage, [string]$FilePath, [string[]]$Arguments,
        [ValidateRange(1000, 120000)][int]$TimeoutMs = 120000
    )
    if ($Stage -notmatch '^[a-z0-9-]+$') { throw 'Invalid diagnostic stage name.' }
    Write-Stage $Stage 'started'
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $process = [Diagnostics.Process]::new()
    $started = $false
    try {
        $process.StartInfo = [Diagnostics.ProcessStartInfo]::new()
        $process.StartInfo.FileName = $FilePath
        $process.StartInfo.UseShellExecute = $false
        $process.StartInfo.CreateNoWindow = $true
        $process.StartInfo.RedirectStandardOutput = $true
        $process.StartInfo.RedirectStandardError = $true
        foreach ($argument in $Arguments) { $process.StartInfo.ArgumentList.Add($argument) }
        $started = $process.Start()
        if (-not $started) { throw "Could not start CI stage $Stage." }
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        $timedOut = -not $process.WaitForExit($TimeoutMs)
        if ($timedOut) {
            # Only this script's own child tree on the disposable hosted VM.
            $process.Kill($true)
            if (-not $process.WaitForExit(5000)) { throw "Could not stop timed-out CI child $Stage." }
        }
        $stdout = $stdoutTask.GetAwaiter().GetResult()
        $stderr = $stderrTask.GetAwaiter().GetResult()
        [IO.File]::WriteAllText((Join-Path $diagnostics "$Stage.stdout.txt"), $stdout)
        [IO.File]::WriteAllText((Join-Path $diagnostics "$Stage.stderr.txt"), $stderr)
        $exitCode = $process.ExitCode
        $state = if ($timedOut) { 'timed-out' } elseif ($exitCode -eq 0) { 'passed' } else { 'failed' }
        Write-Stage $Stage $state $exitCode $watch.Elapsed.TotalMilliseconds
        if ($stdout) { Write-Host $stdout.TrimEnd() }
        if ($stderr) { Write-Host $stderr.TrimEnd() }
        if ($timedOut -or $exitCode -ne 0) {
            $failure = [InvalidOperationException]::new("CI stage $Stage ended with $state (exit $exitCode).")
            $failure.Data['Stage'] = $Stage
            $failure.Data['ExitCode'] = $exitCode
            $failure.Data['TimedOut'] = $timedOut
            throw $failure
        }
        return [pscustomobject]@{ Stdout = $stdout; ExitCode = $exitCode }
    }
    finally {
        if ($started -and -not $process.HasExited) {
            $process.Kill($true)
            $process.WaitForExit(5000) | Out-Null
        }
        $process.Dispose()
    }
}

function Remove-DevelopmentCertificate {
    param([object]$Record)
    if ($Record.SourceCommit -ne $SourceCommit -or $Record.RunId -ne $env:GITHUB_RUN_ID -or
        $Record.Thumbprint -notmatch '^[0-9A-Fa-f]{40}$' -or
        $Record.MyStore -ne 'Cert:\CurrentUser\My' -or $Record.RootStore -ne 'Cert:\LocalMachine\Root' -or
        $Record.KeyProvider -ne 'Microsoft Software Key Storage Provider' -or
        -not $Record.KeyName -or $Record.KeyName.Length -gt 256 -or
        $Record.Subject -notin @('CN=Neon HUD Development', 'CN=Neon HUD Development Cleanup Test')) {
        throw 'Certificate cleanup journal does not match this accepted CI run.'
    }
    $stage = "cleanup-$($Record.Thumbprint.ToLowerInvariant())"
    Write-Stage $stage 'started'
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $rootPath = "$($Record.RootStore)\$($Record.Thumbprint)"
    $myPath = "$($Record.MyStore)\$($Record.Thumbprint)"
    try {
        try {
            if (Test-Path -LiteralPath $rootPath) {
                if ((Get-Item -LiteralPath $rootPath).Subject -ne $Record.Subject) { throw 'Root identity mismatch.' }
                Remove-Item -LiteralPath $rootPath -Force
            }
        }
        finally {
            if (Test-Path -LiteralPath $myPath) {
                if ((Get-Item -LiteralPath $myPath).Subject -ne $Record.Subject) { throw 'Signing identity mismatch.' }
                Remove-Item -LiteralPath $myPath -DeleteKey -Force
            }
        }
        $provider = [Security.Cryptography.CngProvider]::new($Record.KeyProvider)
        if ((Test-Path -LiteralPath $rootPath) -or (Test-Path -LiteralPath $myPath) -or
            [Security.Cryptography.CngKey]::Exists($Record.KeyName, $provider)) {
            throw 'Ephemeral certificate or persisted CNG key cleanup was not confirmed.'
        }
        Write-Stage $stage 'passed' 0 $watch.Elapsed.TotalMilliseconds
    }
    catch {
        Write-Stage $stage 'failed' $null $watch.Elapsed.TotalMilliseconds
        throw
    }
}

function Remove-AllDevelopmentCertificates {
    $failures = [Collections.Generic.List[string]]::new()
    foreach ($record in $records) {
        try { Remove-DevelopmentCertificate $record }
        catch { $failures.Add($_.Exception.Message); Write-Host $_.Exception.Message }
    }
    if ($failures.Count) { throw "Development cleanup failed: $($failures -join '; ')" }
}

if ($CleanupOnly) {
    Remove-AllDevelopmentCertificates
    Write-Host 'Journaled CI certificates and persisted CNG keys are absent.'
    return
}

function New-DevelopmentCertificate {
    param([string]$Subject, [string]$PublicPath, [string]$Stage)
    Write-Stage $Stage 'started'
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $certificate = $null
    $privateHandle = $null
    try {
        $parameters = @{
            Type = 'CodeSigningCert'; Subject = $Subject; CertStoreLocation = 'Cert:\CurrentUser\My'
            KeyAlgorithm = 'RSA'; KeyLength = 3072; HashAlgorithm = 'SHA256'
            KeyExportPolicy = 'NonExportable'; KeyUsage = 'DigitalSignature'
            Provider = 'Microsoft Software Key Storage Provider'; NotAfter = (Get-Date).AddMonths(6)
        }
        $certificate = New-SelfSignedCertificate @parameters
        $privateHandle = [Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPrivateKey($certificate)
        if ($privateHandle -isnot [Security.Cryptography.RSACng]) { throw 'Expected a persisted non-exportable CNG key.' }
        $record = [pscustomobject]@{
            SourceCommit = $SourceCommit; RunId = $env:GITHUB_RUN_ID
            Thumbprint = $certificate.Thumbprint; Subject = $certificate.Subject
            MyStore = 'Cert:\CurrentUser\My'; RootStore = 'Cert:\LocalMachine\Root'
            KeyName = $privateHandle.Key.KeyName; KeyProvider = $privateHandle.Key.Provider.Provider
        }
        # Public identity metadata, written before any verification trust import.
        $records.Add($record)
        ConvertTo-Json -InputObject $records.ToArray() -Depth 5 |
            Set-Content -LiteralPath $journalPath -Encoding utf8
        Export-Certificate -Cert $certificate -FilePath $PublicPath -Type CERT | Out-Null
        Write-Stage $Stage 'passed' 0 $watch.Elapsed.TotalMilliseconds
        return $record
    }
    catch {
        if ($certificate) {
            Remove-Item -LiteralPath "Cert:\CurrentUser\My\$($certificate.Thumbprint)" -DeleteKey -Force
        }
        Write-Stage $Stage 'failed' $null $watch.Elapsed.TotalMilliseconds
        throw
    }
    finally {
        if ($privateHandle) { $privateHandle.Dispose() }
        if ($certificate) { $certificate.Dispose() }
    }
}

$pwsh = (Get-Process -Id $PID).Path
$inspectScript = Join-Path $diagnostics 'inspect-authenticode.ps1'
@'
param([Parameter(Mandatory)][string]$Path)
$ErrorActionPreference = 'Stop'
$signature = Get-AuthenticodeSignature -LiteralPath $Path
[ordered]@{
    Status = $signature.Status.ToString()
    SignerThumbprint = if ($signature.SignerCertificate) { $signature.SignerCertificate.Thumbprint } else { $null }
    SignerSubject = if ($signature.SignerCertificate) { $signature.SignerCertificate.Subject } else { $null }
    TimestampThumbprint = if ($signature.TimeStamperCertificate) { $signature.TimeStamperCertificate.Thumbprint } else { $null }
    TimestampSubject = if ($signature.TimeStamperCertificate) { $signature.TimeStamperCertificate.Subject } else { $null }
} | ConvertTo-Json -Compress
'@ | Set-Content -LiteralPath $inspectScript -Encoding utf8
$sdk = Join-Path ([Environment]::GetFolderPath('ProgramFilesX86')) 'Windows Kits\10\bin'
$signTool = Get-ChildItem -LiteralPath $sdk -Filter signtool.exe -Recurse |
    Where-Object { $_.Directory.Name -eq 'x64' -and $_.Directory.Parent.Name -match '^10\.\d+\.\d+\.\d+$' } |
    Sort-Object { [version]$_.Directory.Parent.Name } -Descending | Select-Object -First 1
if (-not $signTool) { throw 'Windows SDK x64 SignTool is required.' }
$certutil = Join-Path $env:SystemRoot 'System32\certutil.exe'
# DigiCert's documented RFC3161 endpoint; the signed response is independently verified.
$timestampServer = 'http://timestamp.digicert.com'
$bundleName = 'neon-hud-native-windows-x64-self-signed-development'
$bundle = Join-Path $output $bundleName
New-Item -ItemType Directory -Path $bundle | Out-Null
$publicCertificate = Join-Path $bundle 'Neon-HUD-Development.cer'
$failureCleanupChecks = [Collections.Generic.List[object]]::new()
$signature = $null
$certificateThumbprint = $null
try {
    $unsigned = (Invoke-BoundedTool 'inspect-unsigned' $pwsh @(
        '-NoProfile', '-NonInteractive', '-File', $inspectScript, '-Path', $exe)).Stdout | ConvertFrom-Json
    if ($unsigned.Status -ne 'NotSigned') { throw 'Expected a fresh unsigned build; refusing to replace a signature.' }

    foreach ($scenario in @('nonzero', 'timeout')) {
        $testCertificate = New-DevelopmentCertificate 'CN=Neon HUD Development Cleanup Test' (Join-Path $diagnostics "cleanup-$scenario.cer") "create-test-$scenario"
        $observedFailure = $null
        try {
            Invoke-BoundedTool "trust-test-$scenario" $certutil @(
                '-f', '-addstore', 'Root', (Join-Path $diagnostics "cleanup-$scenario.cer")) | Out-Null
            if (-not (Test-Path -LiteralPath "$($testCertificate.RootStore)\$($testCertificate.Thumbprint)")) {
                throw 'Temporary CI test trust was not established.'
            }
            $testStage = "test-$scenario-verifier"
            try {
                if ($scenario -eq 'nonzero') {
                    Invoke-BoundedTool $testStage $pwsh @('-NoProfile', '-NonInteractive', '-Command', 'exit 7') | Out-Null
                }
                else {
                    Invoke-BoundedTool $testStage $pwsh @('-NoProfile', '-NonInteractive', '-Command', 'Start-Sleep -Seconds 10') -TimeoutMs 1000 | Out-Null
                }
                throw 'Injected verifier unexpectedly succeeded.'
            }
            catch {
                if ($_.Exception.Data['Stage'] -ne $testStage -or
                    ($scenario -eq 'nonzero' -and $_.Exception.Data['ExitCode'] -ne 7) -or
                    ($scenario -eq 'timeout' -and $_.Exception.Data['TimedOut'] -ne $true)) { throw }
                $observedFailure = $_.Exception
            }
        }
        finally { Remove-DevelopmentCertificate $testCertificate }
        $failureCleanupChecks.Add([ordered]@{
            Scenario = $scenario; ExitCode = $observedFailure.Data['ExitCode']
            TimedOut = $observedFailure.Data['TimedOut']; CertificateAndKeyRemoved = $true
        })
        Write-Stage "negative-cleanup-$scenario" 'passed' 0
    }

    $certificate = New-DevelopmentCertificate 'CN=Neon HUD Development' $publicCertificate 'create-signing-certificate'
    $certificateThumbprint = $certificate.Thumbprint
    Invoke-BoundedTool 'sign-timestamp' $signTool.FullName @(
        'sign', '/fd', 'SHA256', '/sha1', $certificateThumbprint, '/s', 'My',
        '/tr', $timestampServer, '/td', 'SHA256', $exe) | Out-Null
    # Machine trust on an already elevated disposable hosted VM avoids the
    # interactive CurrentUser Root import used by the previous stalled attempt.
    # https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/certutil
    Invoke-BoundedTool 'trust-signing-certificate' $certutil @('-f', '-addstore', 'Root', $publicCertificate) | Out-Null
    if (-not (Test-Path -LiteralPath "$($certificate.RootStore)\$certificateThumbprint")) {
        throw 'Temporary CI signing trust was not established.'
    }
    Invoke-BoundedTool 'verify-signtool' $signTool.FullName @('verify', '/pa', '/all', '/v', '/tw', $exe) | Out-Null
    $signature = (Invoke-BoundedTool 'verify-authenticode' $pwsh @(
        '-NoProfile', '-NonInteractive', '-File', $inspectScript, '-Path', $exe)).Stdout | ConvertFrom-Json
    if ($signature.Status -ne 'Valid' -or $signature.SignerThumbprint -ne $certificateThumbprint -or
        -not $signature.TimestampThumbprint) { throw 'Signer, file-integrity or timestamp verification failed.' }
}
finally { Remove-AllDevelopmentCertificates }

Write-Stage 'package' 'started'
Copy-Item -LiteralPath $exe -Destination $bundle
foreach ($notice in 'LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE', 'THIRD-PARTY-NOTICES.md') {
    Copy-Item -LiteralPath (Join-Path $workspace $notice) -Destination $bundle
}
Copy-Item -LiteralPath (Join-Path $workspace 'src-native\licenses') -Destination $bundle -Recurse
@'
SELF-SIGNED DEVELOPMENT BUILD. Publisher: Neon HUD Development.
The public certificate is included for inspection, not for installing trust.
This certificate is not issued by a publicly trusted certification authority.
Signing does not resolve a Microsoft Defender behavior detection or prove safety.
No security exclusions, protection changes or certificate imports are required.
This is a separate test artifact; it is not published to the automatic-update feed.
Nook: Preferences > Instruments > HUD layout > Nook. Existing choices are retained.
'@ | Set-Content -LiteralPath (Join-Path $bundle 'DEVELOPMENT.txt') -Encoding utf8
$receipt = [ordered]@{
    SourceCommit = $SourceCommit
    RunId = $env:GITHUB_RUN_ID
    BuildKind = 'self-signed-development'
    ExecutableSHA256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    CertificateSHA256 = (Get-FileHash -LiteralPath $publicCertificate -Algorithm SHA256).Hash.ToLowerInvariant()
    SignerSubject = $signature.SignerSubject
    SignerThumbprint = $certificateThumbprint
    FileDigest = 'SHA256'
    TimestampProtocol = 'RFC3161'
    TimestampDigest = 'SHA256'
    TimestampServer = $timestampServer
    TimestampSigner = $signature.TimestampSubject
    Verification = 'SignTool /pa /all /v /tw and Get-AuthenticodeSignature Valid with temporary hosted-VM LocalMachine trust'
    PubliclyTrustedPublisher = $false
    DefenderClearance = 'not established'
    PrivateKeyExported = $false
    TemporaryCertificateStoresCleaned = $true
    PersistedCngKeysRemoved = $true
    FailureCleanupChecks = $failureCleanupChecks.ToArray()
    SignedUtc = [DateTime]::UtcNow.ToString('o')
}
$receipt | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $bundle 'signing-receipt.json') -Encoding utf8
Compress-Archive -LiteralPath $bundle -DestinationPath (Join-Path $output "$bundleName.zip")
$archive = [IO.Compression.ZipFile]::OpenRead((Join-Path $output "$bundleName.zip"))
try {
    foreach ($binding in @{
        'neon-hud-native.exe' = $receipt.ExecutableSHA256
        'Neon-HUD-Development.cer' = $receipt.CertificateSHA256
    }.GetEnumerator()) {
        $entry = $archive.GetEntry("$bundleName/$($binding.Key)")
        if (-not $entry) { throw "Signed bundle is missing $($binding.Key)" }
        $stream = $entry.Open()
        try {
            $archiveHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($stream)).ToLowerInvariant()
            if ($archiveHash -ne $binding.Value) { throw "Signed bundle hash differs for $($binding.Key)" }
        }
        finally { $stream.Dispose() }
    }
    Write-Host "Signed bundle entries verified: $($archive.Entries.Count)"
}
finally { $archive.Dispose() }
Copy-Item -LiteralPath (Join-Path $bundle 'signing-receipt.json') -Destination $output
Get-ChildItem -LiteralPath $output -File | ForEach-Object {
    '{0}  {1}' -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.Name
} | Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS.txt') -Encoding ascii
Write-Stage 'package' 'passed' 0
Write-Host 'Self-signed development package verified; ephemeral certificates and persisted CNG keys removed.'
$receipt | ConvertTo-Json -Depth 5 | Write-Output
Get-Content -LiteralPath (Join-Path $output 'SHA256SUMS.txt') | Write-Output
