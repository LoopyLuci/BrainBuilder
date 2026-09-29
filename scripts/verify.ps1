# BrainBuilder one-shot verification.
# Runs everything that can be checked without hardware/GUI, then prints the
# short manual checklist for the paths that genuinely need the native app.
# Usage:  pwsh scripts/verify.ps1
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
$fail = 0

function Step($name, $block) {
  Write-Host "`n=== $name ===" -ForegroundColor Cyan
  try { & $block; Write-Host "PASS: $name" -ForegroundColor Green }
  catch { Write-Host "FAIL: $name -> $_" -ForegroundColor Red; $script:fail++ }
}

Step "core lib tests" { cargo test -p brainbuilder-core --lib --quiet }
Step "wgpu adapter tests" { cargo test -p brainbuilder-core --lib --features wgpu,pollster adapter_tests --quiet }
Step "frontend typecheck" { Push-Location gui; npx tsc --noEmit; Pop-Location }
Step "frontend unit tests (vitest)" { Push-Location gui; npx vitest run; Pop-Location }
Step "DSpark engine smoke test" { $env:PYTHONPATH = $root; python "$root/dspark_system/test_smoke.py" }
Step "DSpark serve harness runs" { $env:PYTHONPATH = $root; python -m dspark_system.serve --new-tokens 4 }

Write-Host "`n================ MANUAL (native app only) ================" -ForegroundColor Yellow
Write-Host @"
These need the real BrainBuilder.exe (Tauri IPC / hardware) — see docs/VERIFICATION.md:
  1. OpenCode inference: set key in Models, author a graph with an opencode-go/* model.
  2. Agent (propose mode): run a task, review the diff, confirm the test gate + merge.
  3. Nervous System audit: validate a graph, watch a Racket 'allowed' row appear in
     Console -> Nervous System.
  4. GPU (RX 7900 XTX): Models -> GPU -> pick the card -> Test this GPU (expect 'bound').
  5. Auto-tune: run it on a small graph, confirm trials rank + the winner applies.
"@ -ForegroundColor Yellow

Pop-Location
if ($fail -gt 0) { Write-Host "`n$fail automated step(s) failed." -ForegroundColor Red; exit 1 }
Write-Host "`nAll automated checks passed." -ForegroundColor Green
