# OmniForge – Windows 10 setup (run in PowerShell)
$ErrorActionPreference = "Stop"
Write-Host "==> OmniForge Windows setup" -ForegroundColor Cyan

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
  Write-Host "Node.js not found. Install LTS from https://nodejs.org and re-run." -ForegroundColor Red
  exit 1
}
Write-Host "Node: $(node -v)"

if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
  Write-Host "Rust not found. Install from https://rustup.rs then re-open this terminal." -ForegroundColor Red
  exit 1
}
Write-Host "Rust: $(rustc --version)"

Write-Host @"

Ensure these are installed:
  1. Visual Studio Build Tools 2022 – workload "Desktop development with C++"
  2. WebView2 Runtime (usually preinstalled on Win10)

"@ -ForegroundColor Yellow

if (Get-Command python -ErrorAction SilentlyContinue) {
  Write-Host "Python: $(python --version)"
} else {
  Write-Host "Python not on PATH – training uses demo fallbacks." -ForegroundColor Yellow
}

Set-Location (Join-Path $PSScriptRoot "..")
Write-Host "==> npm install"
npm install
Write-Host "==> Done. Run: npm run tauri dev" -ForegroundColor Green
