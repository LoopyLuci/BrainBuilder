import { DndProvider } from 'react-dnd';
import { HTML5Backend } from 'react-dnd-html5-backend';
import { Header } from './shell/Header';
import { SlotRenderer } from './widgets/SlotRenderer';
import { registerBuiltinWidgets } from './widgets/builtins';
import { restorePlugins } from './widgets/loader';
import { syncPreferredGpuToBackend } from './api/models';
import { TutorialOverlay } from './tutorial/TutorialOverlay';
import { ConfirmDialog } from './ui/ConfirmDialog';
import { useLayoutStore } from './state/layoutStore';

// Register the first-party panels once, at module load, before React renders.
// Everything the shell shows now comes from the widget registry — App.tsx no
// longer hardcodes which panels exist, so panels (built-in or hot-loaded
// plugins) can be added, removed, or swapped at runtime without editing here.
registerBuiltinWidgets();
// Re-load any runtime plugins the user had active at last shutdown, so their
// hot-loaded panels survive a restart. Fire-and-forget: failures are captured
// per-plugin in the loader's own error state, never fatal to boot.
void restorePlugins();
// Apply the persisted GPU preference to the backend so native ops route to the
// chosen device from the first run, without needing to reopen the picker.
syncPreferredGpuToBackend();

function App() {
  const sideRailVisible = useLayoutStore((s) => s.sideRailVisible);
  const rightRailVisible = useLayoutStore((s) => s.rightRailVisible);
  const paletteVisible = useLayoutStore((s) => s.paletteVisible);
  const bottomHeight = useLayoutStore((s) => s.bottomHeight);
  const sideWidth = useLayoutStore((s) => s.sideWidth);
  const rightWidth = useLayoutStore((s) => s.rightWidth);
  const setBottomHeight = useLayoutStore((s) => s.setBottomHeight);
  const setSideWidth = useLayoutStore((s) => s.setSideWidth);
  const setRightWidth = useLayoutStore((s) => s.setRightWidth);

  return (
    <DndProvider backend={HTML5Backend}>
      <div className="app-shell">
        <Header />
        <div className="app-layout">
          {paletteVisible && (
            <div className="palette" data-tutorial="palette">
              <SlotRenderer slot="palette" />
            </div>
          )}
          {paletteVisible && (
            <div
              className="resizer resizer--vertical"
              onMouseDown={(e) => {
                const startX = e.clientX;
                const startWidth = 230;
                const onMove = (ev: MouseEvent) => {
                  const next = startWidth + (ev.clientX - startX);
                  setSideWidth(next);
                };
                const onUp = () => {
                  window.removeEventListener('mousemove', onMove);
                  window.removeEventListener('mouseup', onUp);
                };
                window.addEventListener('mousemove', onMove);
                window.addEventListener('mouseup', onUp);
              }}
            />
          )}
          <div className="canvas-container" data-tutorial="canvas">
            <SlotRenderer slot="canvas" />
          </div>
          {sideRailVisible && (
            <>
              <div
                className="resizer resizer--vertical"
                onMouseDown={(e) => {
                  const startX = e.clientX;
                  const startWidth = sideWidth;
                  const onMove = (ev: MouseEvent) => {
                    const next = startWidth + (ev.clientX - startX);
                    setSideWidth(next);
                  };
                  const onUp = () => {
                    window.removeEventListener('mousemove', onMove);
                    window.removeEventListener('mouseup', onUp);
                  };
                  window.addEventListener('mousemove', onMove);
                  window.addEventListener('mouseup', onUp);
                }}
              />
              <div className="side-rail" style={{ width: sideWidth }}>
                <SlotRenderer slot="side" asTabs />
              </div>
            </>
          )}
          {rightRailVisible && (
            <>
              <div
                className="resizer resizer--vertical"
                onMouseDown={(e) => {
                  const startX = e.clientX;
                  const startWidth = rightWidth;
                  const onMove = (ev: MouseEvent) => {
                    const next = startWidth - (ev.clientX - startX);
                    setRightWidth(next);
                  };
                  const onUp = () => {
                    window.removeEventListener('mousemove', onMove);
                    window.removeEventListener('mouseup', onUp);
                  };
                  window.addEventListener('mousemove', onMove);
                  window.addEventListener('mouseup', onUp);
                }}
              />
              <div className="right-rail" style={{ width: rightWidth }}>
                <SlotRenderer slot="right" asTabs />
              </div>
            </>
          )}
        </div>
        <div className="bottom-rail" style={{ height: bottomHeight }}>
          <div
            className="resizer resizer--horizontal"
            onMouseDown={(e) => {
              const startY = e.clientY;
              const startHeight = bottomHeight;
              const onMove = (ev: MouseEvent) => {
                const next = startHeight - (ev.clientY - startY);
                setBottomHeight(next);
              };
              const onUp = () => {
                window.removeEventListener('mousemove', onMove);
                window.removeEventListener('mouseup', onUp);
              };
              window.addEventListener('mousemove', onMove);
              window.addEventListener('mouseup', onUp);
            }}
          />
          <SlotRenderer slot="bottom" asTabs />
        </div>
      </div>
      <TutorialOverlay />
      <ConfirmDialog />
    </DndProvider>
  );
}

export default App;
