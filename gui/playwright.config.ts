import { defineConfig, devices } from '@playwright/test';

// End-to-end tests for the pure-frontend flows only.
//
// HONEST SCOPE: these tests run against `vite preview` (the built app served
// as a plain static web page) in a real Chromium browser. There is no Tauri
// runtime here — no `tauri-driver`, no WebKitWebDriver/WinAppDriver, and no
// packaged desktop binary is launched or exercised. Anything that calls
// `invoke()` (see src/api/tauri.ts) would normally talk to the Rust backend
// over Tauri's IPC bridge (`window.__TAURI_IPC__`); since that bridge does
// not exist in a plain browser, invoke() would reject/hang forever. Each
// test stubs `window.__TAURI_IPC__` via `addInitScript` (see e2e/tauriMock.ts)
// so the frontend's existing error-handling paths don't block the UI, and so
// flows that only need component descriptors (not real training/execution)
// can proceed. This suite proves the React/Zustand/reactflow frontend wiring
// works in a browser — it does NOT prove Tauri IPC or the Rust core works.
export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: [['list']],
  use: {
    baseURL: 'http://localhost:4173',
    trace: 'retain-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: {
    command: 'npm run build && npm run preview -- --port 4173 --strictPort',
    url: 'http://localhost:4173',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
