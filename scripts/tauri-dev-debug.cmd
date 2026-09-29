@echo off
REM Launches the real Tauri dev app with WebView2's remote-debugging port open,
REM so an external Chrome DevTools Protocol client can attach to the actual
REM native window's webview content (not just the plain browser tab that
REM `npm run dev` alone serves). Debug-only; never used in production builds.
set WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222
cd /d "%~dp0.."
call npm --prefix gui run tauri -- dev
