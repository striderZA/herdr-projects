# herdr runs every plugin command through this file on Windows, with its server's environment (which may lack the user's PATH).
# It mirrors bin/run.sh step for step, with the same messages and exit codes. It runs the program, the first of:
#   1. the one built from this checkout (target\release\herdr-projects.exe), when no source file is newer than it;
#   2. the downloaded one for this platform, bin\herdr-projects-windows-x86_64.exe, when it is this checkout's version
#      (Cargo.toml) or newer (`herdr-projects update` puts a newer one there); its version is in the .version file
#      beside it, read without running it;
#   3. that file, downloaded now from the GitHub release of this checkout's version: its checksum checked, renamed into
#      place in one step, one download at a time (a second start waits for the first);
#   4. failing that: an older build from this checkout, or an older download, with a line saying so. A failed
#      download is tried again ten minutes later, not on every command.
# When there is none it says on stderr what to do and exits 1: it never waits long, so it never holds herdr up.
# `run.ps1 --fetch` only gets the program (the plugin's build step), building it from source when it cannot download.
# HERDR_PROJECTS_RELEASES names another place for the releases (a fork, a mirror, a folder as file:///...).
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent $here

function Say([string]$msg) { [Console]::Error.WriteLine("herdr-projects: $msg") }

$rest = @($args)
$fetchOnly = $false
if ($rest.Count -gt 0 -and $rest[0] -eq '--fetch') { $fetchOnly = $true; $rest = @($rest | Select-Object -Skip 1) }
function Run-Program([string]$exe) {
  if ($fetchOnly) { exit 0 }
  & $exe @rest
  exit $LASTEXITCODE
}

function Sha256Of([string]$file) {
  $sha = [System.Security.Cryptography.SHA256]::Create()
  $stream = [System.IO.File]::OpenRead($file)
  try { return ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace('-', '').ToLower() }
  finally { $stream.Dispose(); $sha.Dispose() }
}

$built = Join-Path $root 'target\release\herdr-projects.exe'
function Fresh {
  if (-not (Test-Path -LiteralPath $built)) { return $false }
  $when = (Get-Item -LiteralPath $built).LastWriteTime
  $sources = @()
  foreach ($d in 'src', 'crates') { $p = Join-Path $root $d; if (Test-Path -LiteralPath $p) { $sources += Get-ChildItem -LiteralPath $p -Recurse -File } }
  foreach ($f in 'Cargo.toml', 'Cargo.lock') { $p = Join-Path $root $f; if (Test-Path -LiteralPath $p) { $sources += Get-Item -LiteralPath $p } }
  return -not ($sources | Where-Object { $_.LastWriteTime -gt $when } | Select-Object -First 1)
}
if (Fresh) { Run-Program $built }

$want = ''
$cargo = Join-Path $root 'Cargo.toml'
if (Test-Path -LiteralPath $cargo) {
  $m = Select-String -LiteralPath $cargo -Pattern '^version\s*=\s*"([^"]*)"' | Select-Object -First 1
  if ($m) { $want = $m.Matches[0].Groups[1].Value }
}
$os = 'windows'
$arch = switch ($env:PROCESSOR_ARCHITECTURE) { 'AMD64' { 'x86_64' } default { '' } }
$name = "herdr-projects-$os-$arch"
$prog = Join-Path $here "$name.exe"
$base = if ($env:HERDR_PROJECTS_RELEASES) { $env:HERDR_PROJECTS_RELEASES } else { 'https://github.com/eliasstravik/herdr-projects/releases' }
$base = $base.TrimEnd('/')

# AtLeast a b: a is b or newer, part by part as numbers (0.10.0 is newer than 0.9.1).
function Leading([string]$s) { $d = $s -replace '^([0-9]*).*$', '$1'; if ($d) { [int64]$d } else { [int64]0 } }
function AtLeast([string]$a, [string]$b) {
  $pa = @($a -split '\.'); $pb = @($b -split '\.')
  for ($i = 0; $i -lt [Math]::Max($pa.Count, $pb.Count); $i++) {
    $x = Leading $(if ($i -lt $pa.Count) { $pa[$i] } else { '' })
    $y = Leading $(if ($i -lt $pb.Count) { $pb[$i] } else { '' })
    if ($x -gt $y) { return $true }
    if ($x -lt $y) { return $false }
  }
  return $true
}
function Have {
  if (-not (Test-Path -LiteralPath $prog)) { return $false }
  if (-not (Test-Path -LiteralPath "$prog.version")) { return $false }
  $v = (Get-Content -LiteralPath "$prog.version" -Raw).Trim()
  return ($v -ne '') -and (AtLeast $v $want)
}
if ($arch -and (Have)) { Run-Program $prog }

$ghRepo = 'https://github.com/eliasstravik/herdr-projects/releases/download/'
function Fetch([string]$url, [string]$dest) {
  if ($url.StartsWith('file:')) {
    try { Copy-Item -LiteralPath ([uri]$url).LocalPath -Destination $dest -Force; return $true } catch { return $false }
  }
  try { Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $dest -TimeoutSec 300 | Out-Null; return $true } catch { }
  # A private repo's release is not handed out without a login: gh, logged in, fetches it.
  if (-not $url.StartsWith($ghRepo)) { return $false }
  if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { return $false }
  $tail = $url.Substring($ghRepo.Length)
  $tag = $tail.Split('/')[0]
  $file = $tail.Substring($tag.Length + 1)
  & gh release download $tag -R eliasstravik/herdr-projects -p $file -O $dest --clobber *> $null
  return ($LASTEXITCODE -eq 0)
}
$buildHint = "or build it here: cd '$root' && cargo build --release (Rust: https://rustup.rs)"
function Writable([string]$dir) {
  try { $f = Join-Path $dir ".write-test.$PID"; [IO.File]::WriteAllText($f, ''); Remove-Item -LiteralPath $f -Force; return $true } catch { return $false }
}

function Download {
  if (-not $arch) { Say "no release is made for Windows ${env:PROCESSOR_ARCHITECTURE}: build it here: cd '$root' && cargo build --release (Rust: https://rustup.rs)"; return $false }
  if (-not $want) { Say "cannot read the version in $root\Cargo.toml: $buildHint"; return $false }
  if (-not (Writable $here)) { Say "cannot write in $here to download the program: $buildHint"; return $false }
  $lock = "$prog.lock"; $n = 0
  while ($true) {
    try { New-Item -ItemType Directory -Path $lock -ErrorAction Stop | Out-Null; break } catch { }
    $holder = ''
    if (Test-Path -LiteralPath (Join-Path $lock 'pid')) { $holder = (Get-Content -LiteralPath (Join-Path $lock 'pid') -Raw).Trim() }
    $holderId = 0
    $alive = $holder -ne '' -and [int]::TryParse($holder, [ref]$holderId) -and (Get-Process -Id $holderId -ErrorAction SilentlyContinue)
    if (($holder -ne '' -and -not $alive) -or ($holder -eq '' -and $n -ge 5)) { Remove-Item -LiteralPath $lock -Recurse -Force -ErrorAction SilentlyContinue; continue }
    $n++
    if ($n -ge 150) { Say "another start is still downloading ${name}: try again in a minute"; return $false }
    Start-Sleep -Seconds 1
    if (Have) { return $true }
  }
  Set-Content -LiteralPath (Join-Path $lock 'pid') -Value $PID
  $tmp = Join-Path $here ".$name.$PID.download"
  $url = "$base/download/v$want/$name.exe"
  Say "downloading herdr-projects $want for $os $arch (once)"
  $ok = $false
  try {
    if (-not (Fetch $url $tmp) -or -not (Fetch "$url.sha256" "$tmp.sha256")) {
      Say "could not download $url (no network, no release v$want with this file yet, or a private repo: log in with gh auth login): try again later, $buildHint"
    } else {
      $sum = ((Get-Content -LiteralPath "$tmp.sha256" -Raw).Trim() -split '\s+')[0].ToLower()
      $got = Sha256Of $tmp
      if ($sum -ne $got) {
        Say "the download does not match its checksum, so it is not used: try again, $buildHint"
      } else {
        try {
          # Renamed over the old one, never written into: a running program keeps its file.
          Move-Item -LiteralPath $tmp -Destination $prog -Force -ErrorAction Stop
          Set-Content -LiteralPath "$prog.version.$PID" -Value $want
          Move-Item -LiteralPath "$prog.version.$PID" -Destination "$prog.version" -Force -ErrorAction Stop
          $ok = $true
        } catch {
          Say "could not put the download in place (the program may be running): $buildHint"
        }
      }
    }
  } finally {
    Remove-Item -LiteralPath $lock -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $tmp, "$tmp.sha256" -Force -ErrorAction SilentlyContinue
  }
  return $ok
}

# A download that failed is not tried again for ten minutes (but by the build step): every hook would wait for it.
$failed = "$prog.failed"
$recentlyFailed = (-not $fetchOnly) -and (Test-Path -LiteralPath $failed) -and ((Get-Item -LiteralPath $failed).LastWriteTime -gt (Get-Date).AddMinutes(-10))
if ($recentlyFailed) { }
elseif (Download) { Remove-Item -LiteralPath $failed -Force -ErrorAction SilentlyContinue; Run-Program $prog }
else { Set-Content -LiteralPath $failed -Value '' -ErrorAction SilentlyContinue }

if ($fetchOnly -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Say "building it from source instead: cargo build --release (a few minutes)"
  Push-Location $root
  & cargo build --release --locked
  $builtOk = ($LASTEXITCODE -eq 0)
  Pop-Location
  if ($builtOk) { exit 0 }
}
if (Test-Path -LiteralPath $built) { Say "using $built, which is older than its source: build it again (cd '$root' && cargo build --release)"; Run-Program $built }
if (Test-Path -LiteralPath $prog) {
  $v = ''
  if (Test-Path -LiteralPath "$prog.version") { $v = (Get-Content -LiteralPath "$prog.version" -Raw).Trim() }
  Say "using the downloaded herdr-projects $v, not $want"
  Run-Program $prog
}
exit 1
