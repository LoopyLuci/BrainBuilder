# BrainBuilder Manual Test Checklist

## Launch
- [ ] `cd Z:/Projects/BrainBuilder/gui && npx tauri dev`
- [ ] App window opens without crashing
- [ ] Dev server reachable at `http://localhost:5174`

## Core Load
- [ ] Title/banner shows **BrainBuilder**
- [ ] Top tab bar renders all tabs
- [ ] Component palette loads (linear, gelu, relu, conv2d, dropout)
- [ ] Canvas area renders without blank/white screen
- [ ] Theme toggle button responds

## Build Canvas
- [ ] Drag a component from palette to canvas
- [ ] Double-click palette item to add to canvas
- [ ] Select node → Inspector updates
- [ ] `Validate` button executes
- [ ] `Export & Train` button executes
- [ ] `New` clears canvas
- [ ] `Save…` and `Load…` persist/restore graph
- [ ] Undo/Redo buttons enable after mutations

## Templates
- [ ] Open `Templates` tab
- [ ] Click `Use` on an enabled preset
- [ ] Graph populates with preset nodes/edges
- [ ] Canvas fit-view after template load

## Data Panel
- [ ] Open `Data` tab
- [ ] Add or load dataset
- [ ] Column selector updates
- [ ] Preview table renders rows
- [ ] Assign column to node input

## Inspector
- [ ] Select any node on canvas
- [ ] Properties panel shows node config
- [ ] Edit a numeric/text field
- [ ] Changes reflect on canvas or on re-select

## Training Metrics (Learn tab)
- [ ] Open `Learn` tab
- [ ] `Start` buttons are present
- [ ] Trigger a training start
- [ ] Metrics panel updates or shows placeholder state

## Console / Feedback
- [ ] Open `Console` tab
- [ ] Trigger an action that should log
- [ ] Console shows output without layout break
- [ ] No visible JS error overlays

## Cross-Tab Stability
- [ ] Switch rapidly through all tabs
- [ ] Return to `Build`; canvas state preserved
- [ ] Theme toggle works in every tab
- [ ] No blank panels after tab switches

## Native Window (Tauri/WebView2)
- [ ] Window title = BrainBuilder
- [ ] Resize/maximize/restore works
- [ ] No Windows “app has crashed” dialog after 5 minutes idle
- [ ] Ctrl+R / hot reload does not crash app
