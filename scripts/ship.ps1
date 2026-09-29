# BrainBuilder one-command ship gate.
# Runs everything verify.ps1 runs, PLUS the contract/acceptance/e2e checks
# that live under testing/ and gui/e2e/, and — only if every one of those
# passes — kicks off a real `tauri build` to produce a packaged binary.
#
# Some of the extra checks below are being built by parallel efforts and may
# not exist on disk yet at the moment this script runs; those specific steps
# are skipped with a warning (not a failure) when the file/script is simply
# missing. Anything that DOES exist must pass, same as every other step.
#
# Usage:  pwsh scripts/ship.ps1
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
$fail = 0

function Step($name, $block) {
  Write-Host "`n=== $name ===" -ForegroundColor Cyan
  try { & $block; Write-Host "PASS: $name" -ForegroundColor Green }
  catch { Write-Host "FAIL: $name -> $_" -ForegroundColor Red; $script:fail++ }
}

# Like Step, but if $path doesn't exist yet, skip with a warning instead of
# failing the whole gate — used for the pieces still landing under testing/
# and gui/e2e/ from parallel work.
function OptionalStep($name, $path, $block) {
  if (-not (Test-Path $path)) {
    Write-Host "`n=== $name ===" -ForegroundColor Cyan
    Write-Host "SKIP: $name -> '$path' not found yet" -ForegroundColor Yellow
    return
  }
  Step $name $block
}

# ---- Everything verify.ps1 runs ----
Step "core lib tests" { cargo test -p brainbuilder-core --lib --quiet }
Step "wgpu adapter tests" { cargo test -p brainbuilder-core --lib --features wgpu,pollster adapter_tests --quiet }
Step "frontend typecheck" { Push-Location gui; npx tsc --noEmit; Pop-Location }
Step "frontend unit tests (vitest)" { Push-Location gui; npx vitest run; Pop-Location }
Step "DSpark engine smoke test" { $env:PYTHONPATH = $root; python "$root/dspark_system/test_smoke.py" }
Step "DSpark serve harness runs" { $env:PYTHONPATH = $root; python -m dspark_system.serve --new-tokens 4 }

# ---- Extra ship-gate checks (contracts, acceptance bench, e2e) ----
OptionalStep "worker protocol contract check" `
  "$root/testing/contracts/check_worker_protocol.py" `
  { python "$root/testing/contracts/check_worker_protocol.py" }

OptionalStep "DSpark acceptance-rate regression bench" `
  "$root/testing/bench/dspark_acceptance.py" `
  { $env:PYTHONPATH = $root; python "$root/testing/bench/dspark_acceptance.py" }

OptionalStep "GUI end-to-end tests (playwright)" `
  "$root/gui/e2e" `
  { Push-Location gui; npm run e2e; Pop-Location }

if ($fail -gt 0) {
  Write-Host "`n================ SHIP: FAIL ================" -ForegroundColor Red
  Write-Host "$fail gate step(s) failed. Not building a release binary." -ForegroundColor Red
  Pop-Location
  exit 1
}

Write-Host "`nAll gate steps passed. Building the packaged app..." -ForegroundColor Green

# ---- Build the real packaged binary ----
# Genuinely slow, and needs the full native Tauri build toolchain (Rust MSVC
# target + WebView2 on Windows, webkit2gtk on Linux, Xcode CLT on macOS).
# We still attempt it here so a missing toolchain is caught immediately as a
# real, actionable failure rather than surfacing only when someone tries to
# cut a release.
$buildFailed = $false
Push-Location gui
try {
  npm run tauri build
} catch {
  Write-Host "FAIL: tauri build -> $_" -ForegroundColor Red
  $buildFailed = $true
}
Pop-Location

Pop-Location

if ($buildFailed) {
  Write-Host "`n================ SHIP: FAIL ================" -ForegroundColor Red
  Write-Host "Gate passed but the packaged build failed. See output above." -ForegroundColor Red
  exit 1
}

Write-Host "`n================ SHIP: PASS ================" -ForegroundColor Green
Write-Host "Packaged binary(ies) under: gui/src-tauri/target/release/bundle/" -ForegroundColor Green
