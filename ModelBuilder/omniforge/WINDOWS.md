# OmniForge on Windows 10

## Prerequisites

| Component | Version | Notes |
|-----------|---------|-------|
| **Windows** | 10 20H2+ (or 11) | WebView2 required |
| **Node.js** | ≥ 18 LTS | https://nodejs.org |
| **Rust** | stable | https://rustup.rs |
| **MSVC Build Tools** | 2022 | “Desktop development with C++” workload |
| **WebView2** | Evergreen | Usually already installed |
| **Python** | ≥ 3.10 (optional) | Training + plugins; add to PATH |

## One-time setup

```powershell
cd omniforge
powershell -ExecutionPolicy Bypass -File .\scripts\setup-windows.ps1
```

Or manually:

```powershell
npm install
```

## Run (development)

```powershell
npm run tauri dev
```

Or double-click `scripts\run-windows.bat`.

## Production build

```powershell
npm run tauri build
```

Installers appear under:

```
src-tauri\target\release\bundle\msi\
src-tauri\target\release\bundle\nsis\
```

## Data locations

| Data | Path |
|------|------|
| Canvas + model registry | `%APPDATA%\OmniForge\omniforge.db` |
| Concierge memory | `%APPDATA%\OmniForge\concierge.db` |
| Models / KMs / output | `%APPDATA%\OmniForge\models` etc. |

Override with `OMNIFORGE_DATA_DIR`.

## Concierge integration

The desktop app embeds `concierge-core` as a path dependency. On startup:

1. `OmniForgeHost` opens the SQLite-backed platform (canvas, models)
2. `ConciergeAgent::with_host` receives a `SharedHost` that **delegates every PlatformHost tool** to that live host
3. Chat UI calls `concierge_chat` → ReAct loop → tools mutate the same canvas the React Flow view displays

Natural language therefore drives the same state as the visual editor.

Optional: set `OPENROUTER_API_KEY` for higher rate limits (free models work without a key).

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `link.exe not found` | Install VS Build Tools C++ workload |
| WebView2 errors | Install Evergreen Runtime |
| `python not found` | Install Python 3 and tick “Add to PATH” |
| Blank window | Delete `%APPDATA%\OmniForge` and retry; check `RUST_LOG=debug` |
| Antivirus locks `target\` | Exclude the project folder from real-time scan during build |

## Architecture check

```
React UI  --invoke-->  Tauri commands  --Arc-->  OmniForgeHost (SQLite)
                              |
                         ConciergeAgent
                              |
                    tools → SharedHost → same OmniForgeHost
```
