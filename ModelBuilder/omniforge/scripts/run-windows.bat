@echo off
cd /d "%~dp0\.."
if not exist node_modules (
  echo First-time setup: npm install...
  call npm install
)
set RUST_LOG=info
npm run tauri dev
