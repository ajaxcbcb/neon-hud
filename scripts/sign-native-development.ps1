[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Executable,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$SourceCommit
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# This test certificate must never alter a user's certificate stores. The key
# lives only on an isolated GitHub Windows runner and is never exported.
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_OS -ne 'Windows' -or
    -not $env:RUNNER_TEMP -or -not $env:GITHUB_WORKSPACE) {
    throw 'Development signing is restricted to an isolated GitHub Windows runner.'
}
if ($SourceCommit -ne $env:GITHUB_SHA) { throw 'Source commit differs from the accepted checkout.' }
# DigiCert documents this RFC3161 endpoint for SignTool. The signed response is
# verified with /pa /all /v /tw; no arbitrary timestamp URL is accepted.
# https://knowledge.digicert.com/solution/troubleshooting-timestamping-problems
$timestampServer = 'http://timestamp.digicert.com'

$workspace = [IO.Path]::GetFullPath($env:GITHUB_WORKSPACE).TrimEnd('\') + '\'
$temporaryRoot = [IO.Path]::GetFullPath($env:RUNNER_TEMP).TrimEnd('\') + '\'
$exe = (Resolve-Path -LiteralPath $Executable).ProviderPath
$output = [IO.Path]::GetFullPath($OutputDirectory)
if (-not $exe.StartsWith($workspace, [StringComparison]::OrdinalIgnoreCase) -or
    [IO.Path]::GetFileName($exe) -ne 'neon-hud-native.exe' -or
    -not $output.StartsWith($temporaryRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Signing inputs must be the checkout binary and a fresh runner-temporary output.'
}
if (Test-Path -LiteralPath $output) { throw 'Signing output must not already exist.' }
if ((Get-AuthenticodeSignature -LiteralPath $exe).Status -ne 'NotSigned') {
    throw 'Expected a fresh unsigned build; refusing to replace an existing signature.'
}
$sdk = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
$signTool = Get-ChildItem -LiteralPath $sdk -Filter signtool.exe -Recurse |
    Where-Object { $_.Directory.Name -eq 'x64' -and $_.Directory.Parent.Name -match '^10\.\d+\.\d+\.\d+$' } |
    Sort-Object { [version]$_.Directory.Parent.Name } -Descending |
    Select-Object -First 1
if (-not $signTool) { throw 'Windows SDK x64 SignTool is required.' }

$bundleName = 'neon-hud-native-windows-x64-self-signed-development'
$bundle = Join-Path $output $bundleName
New-Item -ItemType Directory -Path $bundle -Force | Out-Null
$publicCertificate = Join-Path $bundle 'Neon-HUD-Development.cer'
$certificate = $null
$trustedForVerification = $false
$signature = $null
$certificateThumbprint = $null
try {
    $certificate = New-SelfSignedCertificate -Type CodeSigningCert `
        -Subject 'CN=Neon HUD Development' -CertStoreLocation 'Cert:\CurrentUser\My' `
        -KeyAlgorithm RSA -KeyLength 3072 -HashAlgorithm SHA256 `
        -KeyExportPolicy NonExportable -KeyUsage DigitalSignature `
        -Provider 'Microsoft Software Key Storage Provider' -NotAfter (Get-Date).AddMonths(6)
    $certificateThumbprint = $certificate.Thumbprint
    Export-Certificate -Cert $certificate -FilePath $publicCertificate -Type CERT | Out-Null
    & $signTool.FullName sign /fd SHA256 /sha1 $certificateThumbprint /s My `
        /tr $timestampServer /td SHA256 $exe
    if ($LASTEXITCODE -ne 0) { throw 'Authenticode signing or RFC3161 timestamp failed.' }

    # A self-signed development leaf is not publicly trusted. Temporary trust is
    # confined to this disposable CI account to verify the digest and timestamp.
    Import-Certificate -FilePath $publicCertificate -CertStoreLocation 'Cert:\CurrentUser\Root' | Out-Null
    $trustedForVerification = $true
    & $signTool.FullName verify /pa /all /v /tw $exe
    if ($LASTEXITCODE -ne 0) { throw 'Independent SignTool verification failed.' }
    $signature = Get-AuthenticodeSignature -LiteralPath $exe
    if ($signature.Status -ne 'Valid' -or
        $signature.SignerCertificate.Thumbprint -ne $certificateThumbprint -or
        -not $signature.TimeStamperCertificate) {
        throw 'Signer, file-integrity or timestamp verification failed.'
    }
}
finally {
    try {
        if ($trustedForVerification) {
            Remove-Item -LiteralPath "Cert:\CurrentUser\Root\$certificateThumbprint" -Force
        }
    }
    finally {
        if ($certificate) {
            Remove-Item -LiteralPath "Cert:\CurrentUser\My\$certificateThumbprint" -DeleteKey -Force
        }
    }
}
if ((Test-Path -LiteralPath "Cert:\CurrentUser\My\$certificateThumbprint") -or
    (Test-Path -LiteralPath "Cert:\CurrentUser\Root\$certificateThumbprint")) {
    throw 'Ephemeral certificate cleanup was not confirmed.'
}

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
    BuildKind = 'self-signed-development'
    ExecutableSHA256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    CertificateSHA256 = (Get-FileHash -LiteralPath $publicCertificate -Algorithm SHA256).Hash.ToLowerInvariant()
    SignerSubject = $signature.SignerCertificate.Subject
    SignerThumbprint = $certificateThumbprint
    FileDigest = 'SHA256'
    TimestampProtocol = 'RFC3161'
    TimestampDigest = 'SHA256'
    TimestampServer = $timestampServer
    TimestampSigner = $signature.TimeStamperCertificate.Subject
    Verification = 'SignTool /pa /all /v /tw and Get-AuthenticodeSignature Valid with temporary CI-only development trust'
    PubliclyTrustedPublisher = $false
    DefenderClearance = 'not established'
    PrivateKeyExported = $false
    TemporaryCertificateStoresCleaned = $true
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
    Write-Output "Signed bundle entries verified: $($archive.Entries.Count)"
}
finally { $archive.Dispose() }
Copy-Item -LiteralPath (Join-Path $bundle 'signing-receipt.json') -Destination $output
Get-ChildItem -LiteralPath $output -File | ForEach-Object {
    '{0}  {1}' -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.Name
} | Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS.txt') -Encoding ascii
Write-Output 'Self-signed development package verified; ephemeral certificate and key removed.'
$receipt | ConvertTo-Json -Depth 5 | Write-Output
Get-Content -LiteralPath (Join-Path $output 'SHA256SUMS.txt') | Write-Output
