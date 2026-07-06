import { DndProvider } from 'react-dnd';
import { HTML5Backend } from 'react-dnd-html5-backend';
import { Header } from './shell/Header';
import { SlotRenderer } from './widgets/SlotRenderer';
import { registerBuiltinWidgets } from './widgets/builtins';

// Register the first-party panels once, at module load, before React renders.
// Everything the shell shows now comes from the widget registry — App.tsx no
// longer hardcodes which panels exist, so panels (built-in or hot-loaded
// plugins) can be added, removed, or swapped at runtime without editing here.
registerBuiltinWidgets();

function App() {
  return (
    <DndProvider backend={HTML5Backend}>
      <div className="app-shell">
        <Header />
        <div className="app-layout">
          <div className="palette">
            <SlotRenderer slot="palette" />
          </div>
          <div className="canvas-container">
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
    </DndProvider>
  );
}

export default App;
