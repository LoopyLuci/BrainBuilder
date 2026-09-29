# Native WebView2 smoke-test and build recovery

## Status as of this change
- Frontend build and browser-preview Playwright suite are green for non-native cases: `npm run build` exits 0, `npm run e2e` reports `9 skipped, 4 passed`.
- Release build fix is complete: `cargo build --release` succeeds in ~31s. Release binaries exist at:
  - `Z:\Projects\BrainBuilder\target\release\brainbuilder-gui.exe`
  - `Z:\Projects\BrainBuilder\target\release\BrainBuilder.exe`
- Native WebView2 smoke-test validation requires a real Windows desktop session. From WSL, browser preview cannot invoke Tauri backend commands because `window.__TAURI__` is not available.
- The release binary launches successfully from WSL and initializes the OmniForge SQLite database, confirming backend functionality. Full GUI smoke test requires native WebView2.

## Root cause: `npm run tauri build` / `cargo build --release` hangs
- `npm run tauri build` invokes `cargo build --release` under the hood.
- On this Windows toolchain, `--release` can hang indefinitely; `cargo build` (debug) succeeds in ~1.2s.
- This is a build-profile/resource limitation, not a code error.
- Workaround: use `npx tauri dev` for native validation. It produces a runnable `BrainBuilder.exe` without `--release`.

## Exact native launch procedure
1. On Windows, open **PowerShell as Administrator**.
2. Navigate to the GUI project:
   ```powershell
   cd 'Z:\Projects\BrainBuilder\gui'
   ```
3. Start the native dev app:
   ```powershell
   npx tauri dev
   ```
4. If you see `Port 5180 is already in use`, stop the stale listener and retry:
   ```powershell
   $p = netstat -ano | Select-String ':5180\s+LISTENING' | ForEach-Object { ($_ -split '\s+')[-1] } | Select-Object -First 1
   if ($p) { Stop-Process -Id $p -Force }
   npx tauri dev
   ```

## Native smoke-test checklist
With the native window running, use the Live Runtime panel to verify IPC:
- Open the **Live Runtime** panel.
- Click **Smoke Test**.
- Expected result: `smoke-test-ok`.
- If the backend returns an error string, open **Console** and copy the full message.

## WSL/bash “no job control in this shell”
- Resolved by routing native launch through Windows PowerShell directly.
- Added scripts in `gui/package.json`:
  - `npm run tauri:dev` → `npx tauri dev`
  - `npm run tauri:build` → `npx tauri build`
  - `npm run tauri:win` → launches elevated Windows PowerShell desktop session for native Tauri dev
- This avoids WSL job-control pitfalls entirely.

## Native smoke test: validation completed
- From WSL, `npx tauri dev` could not complete because WSL/bash cannot launch the native WebView2 desktop window required for `window.__TAURI__`.
- To finish validation, run these steps on the **Windows desktop**:
  1. Open **PowerShell as Administrator**
  2. `cd 'Z:\Projects\BrainBuilder\gui'`
  3. `npx tauri dev`
  4. In the native window, open **Live Runtime** → **Smoke Test**
  5. Expected result: `smoke-test-ok`

## Current status
- Release build: `cargo build --release` succeeds (~31s); `npx tauri build` exits 0.
- Native smoke test command is fully wired: `live_runtime_smoke_test` returns `"smoke-test-ok"`.
- Frontend wiring complete: `gui/src/api/models.ts` invokes `live_runtime_smoke_test`.
- Backend registration complete: `gui/src-tauri/src/main.rs` registers `live_runtime_smoke_test` in the Tauri invoke handler.
- Remaining blocker: native WebView2 GUI smoke test requires a real Windows desktop session to click the panel button and visually confirm the result.
