import { DndProvider } from 'react-dnd';
import { HTML5Backend } from 'react-dnd-html5-backend';
import { InfiniteCanvas } from './canvas/InfiniteCanvas';
import { ComponentPalette } from './palette/ComponentPalette';
import { Inspector } from './inspector/Inspector';
import { Console } from './console/Console';
import { TrainingDashboard } from './training/TrainingDashboard';
import { DataPanel } from './data/DataPanel';
import { PredictPanel } from './predict/PredictPanel';
import { ClusterConsole } from './cluster/ClusterConsole';
import { LLMAuthor } from './llm/LLMAuthor';
import { ModelHub } from './models/ModelHub';
import { Header } from './shell/Header';
import { Tabs } from './ui/Tabs';

function App() {
  return (
    <DndProvider backend={HTML5Backend}>
      <div className="app-shell">
        <Header />
        <div className="app-layout">
          <div className="palette"><ComponentPalette /></div>
          <div className="canvas-container"><InfiniteCanvas /></div>
          <div className="side-rail">
            <Tabs
              tabs={[
                { id: 'inspector', label: 'Inspector', content: <Inspector /> },
                { id: 'data', label: 'Data', content: <DataPanel /> },
                { id: 'author', label: 'Author', content: <LLMAuthor /> },
                { id: 'models', label: 'Models', content: <ModelHub /> },
              ]}
            />
          </div>
          <div className="bottom-rail">
            <Tabs
              tabs={[
                { id: 'metrics', label: 'Metrics', content: <TrainingDashboard /> },
                { id: 'predict', label: 'Predict', content: <PredictPanel /> },
                { id: 'cluster', label: 'Cluster', content: <ClusterConsole /> },
                { id: 'console', label: 'Console', content: <Console /> },
              ]}
            />
          </div>
        </div>
      </div>
    </DndProvider>
  );
}

export default App;
