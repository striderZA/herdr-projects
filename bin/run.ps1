# herdr runs plugin commands with its server environment, which may not include the user's PATH.
# The executable is downloaded beside this script and every command uses its absolute path.
$ErrorActionPreference = 'Stop'
$here = (Resolve-Path -LiteralPath $PSScriptRoot).Path
$name = 'herdr-projects-windows-x86_64.exe'
$program = Join-Path $here $name
$launcher = Join-Path $here 'herdr-projects.exe'
$repoRoot = Split-Path -Parent $here
$versionFile = Join-Path $repoRoot 'Cargo.toml'
$versionMatch = Select-String -LiteralPath $versionFile -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
$version = if ($versionMatch) { $versionMatch.Matches[0].Groups[1].Value } else { '' }
$base = $env:HERDR_PROJECTS_RELEASES
if ([string]::IsNullOrWhiteSpace($base)) { $base = 'https://github.com/eliasstravik/herdr-projects/releases' }
$base = $base.TrimEnd('/')

function Fetch-Asset([string]$url, [string]$destination) {
    if ($url -match '^file://') {
        $uri = [Uri]$url
        Copy-Item -LiteralPath $uri.LocalPath -Destination $destination -Force
    } elseif (Test-Path -LiteralPath $url) {
        Copy-Item -LiteralPath $url -Destination $destination -Force
    } else {
        Invoke-WebRequest -Uri $url -OutFile $destination -UseBasicParsing
    }
}
function Get-ReleaseAsset([string]$asset) {
    $url = if ($base -match '^https?://') { "$base/download/v$version/$asset" } else { Join-Path (Join-Path (Join-Path $base 'download') "v$version") $asset }
    return $url
}
if (-not (Get-Command Get-FileHash -ErrorAction SilentlyContinue)) {
    function Get-FileHash {
        param([string]$LiteralPath, [string]$Algorithm)
        $sha = [System.Security.Cryptography.SHA256]::Create()
        try {
            $hex = [BitConverter]::ToString($sha.ComputeHash([IO.File]::ReadAllBytes($LiteralPath))).Replace('-', '').ToLowerInvariant()
            New-Object PSObject -Property @{ Hash = $hex }
        } finally { $sha.Dispose() }
    }
}
function Install-Release {
    if (-not $version) { throw "Cannot read version in $versionFile" }
    $temporary = Join-Path $here ('.' + $name + '.' + [guid]::NewGuid().ToString('N') + '.download')
    $checksumFile = "$temporary.sha256"
    $old = "$program.old"
    try {
        Fetch-Asset (Get-ReleaseAsset $name) $temporary
        Fetch-Asset (Get-ReleaseAsset "$name.sha256") $checksumFile
        $expected = ((Get-Content -LiteralPath $checksumFile -Raw).Trim() -split '\s+')[0].ToLowerInvariant()
        $actual = (Get-FileHash -LiteralPath $temporary -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($expected -notmatch '^[0-9a-f]{64}$' -or $actual -ne $expected) { throw 'The release does not match its SHA-256 checksum.' }
        if (Test-Path -LiteralPath $program) { Move-Item -LiteralPath $program -Destination $old -Force }
        try { Move-Item -LiteralPath $temporary -Destination $program -Force }
        catch { if ((Test-Path -LiteralPath $old) -and -not (Test-Path -LiteralPath $program)) { Move-Item -LiteralPath $old -Destination $program -Force }; throw }
        Set-Content -LiteralPath "$program.version" -Value $version -NoNewline
        if (Test-Path -LiteralPath $old) { Remove-Item -LiteralPath $old -Force }
    } finally {
        Remove-Item -LiteralPath $temporary, $checksumFile -Force -ErrorAction SilentlyContinue
    }
}

$command = if ($args.Count) { [string]$args[0] } else { 'help' }
$remaining = if ($args.Count -gt 1) { $args[1..($args.Count - 1)] } else { @() }
if ($command -eq '--fetch') { Install-Release; exit 0 }
if ($command -eq 'help') { if (-not (Test-Path -LiteralPath $program)) { Install-Release }; & $program 'help' @remaining; exit $LASTEXITCODE }
if ($command -eq 'install') {
    if (-not (Test-Path -LiteralPath $program)) { Install-Release }
    & $program 'install' @remaining
    exit $LASTEXITCODE
}
if (-not (Test-Path -LiteralPath $program)) { Install-Release }
if ($command -eq 'open') {
    $herdr = if ($env:HERDR_BIN_PATH) { $env:HERDR_BIN_PATH } else { 'herdr' }
    $result = (& $herdr tab create --cwd $here --label 'Work board' --focus | Out-String)
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $pane = [regex]::Match($result, '"pane_id":"([^"]+)"').Groups[1].Value
    if (-not $pane) { throw 'herdr tab create did not return a root pane id.' }
    & $herdr pane run $pane "& `"$program`" board"
    exit $LASTEXITCODE
}
if (@('hook', 'board', 'open-coordinator', 'approve-current') -notcontains $command) { $remaining = @($command) + $remaining; $command = 'help' }
& $program $command @remaining
exit $LASTEXITCODE
