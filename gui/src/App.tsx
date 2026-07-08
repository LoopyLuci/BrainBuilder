import { DndProvider } from 'react-dnd';
import { HTML5Backend } from 'react-dnd-html5-backend';
import { Header } from './shell/Header';
import { SlotRenderer } from './widgets/SlotRenderer';
import { registerBuiltinWidgets } from './widgets/builtins';
import { restorePlugins } from './widgets/loader';
import { syncPreferredGpuToBackend } from './api/models';
import { TutorialOverlay } from './tutorial/TutorialOverlay';
import { ConfirmDialog } from './ui/ConfirmDialog';

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
  return (
    <DndProvider backend={HTML5Backend}>
      <div className="app-shell">
        <Header />
        <div className="app-layout">
          <div className="palette" data-tutorial="palette">
            <SlotRenderer slot="palette" />
          </div>
          <div className="canvas-container" data-tutorial="canvas">
            <SlotRenderer slot="canvas" />
          </div>
          <div className="side-rail">
            <SlotRenderer slot="side" asTabs />
          </div>
          <div className="bottom-rail">
            <SlotRenderer slot="bottom" asTabs />
          </div>
        </div>
      </div>
      <TutorialOverlay />
      <ConfirmDialog />
    </DndProvider>
  );
}

export default App;
