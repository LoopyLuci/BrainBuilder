import { registerWidget } from './registry';
import { ComponentPalette } from '../palette/ComponentPalette';
import { InfiniteCanvas } from '../canvas/InfiniteCanvas';
import { Inspector } from '../inspector/Inspector';
import { Console } from '../console/Console';
import { TrainingDashboard } from '../training/TrainingDashboard';
import { DataPanel } from '../data/DataPanel';
import { PredictPanel } from '../predict/PredictPanel';
import { ClusterConsole } from '../cluster/ClusterConsole';
import { LLMAuthor } from '../llm/LLMAuthor';
import { ModelHub } from '../models/ModelHub';
import { IntentPanel } from '../intent/IntentPanel';
import { TemplatesPanel } from '../templates/TemplatesPanel';
import { SynthesizePanel } from '../synthesis/SynthesizePanel';
import { AgentPanel } from '../agent/AgentPanel';
import { PluginsPanel } from './PluginsPanel';

// Registers all first-party panels as widgets. This replaces the hardcoded
// panel wiring that used to live inline in App.tsx — every panel is now a
// registry entry, so the same mechanism that mounts these mounts runtime
// plugins. Order values leave gaps so plugins can slot between built-ins.
export function registerBuiltinWidgets() {
  registerWidget({ id: 'palette', title: 'Components', slot: 'palette', component: ComponentPalette, order: 10 });
  registerWidget({ id: 'canvas', title: 'Canvas', slot: 'canvas', component: InfiniteCanvas, order: 10 });

  // Side rail (tabs).
  registerWidget({ id: 'build', title: 'Build', slot: 'side', component: IntentPanel, order: 10 });
  registerWidget({ id: 'templates', title: 'Templates', slot: 'side', component: TemplatesPanel, order: 15 });
  registerWidget({ id: 'inspector', title: 'Inspector', slot: 'side', component: Inspector, order: 20 });
  registerWidget({ id: 'data', title: 'Data', slot: 'side', component: DataPanel, order: 30 });
  registerWidget({ id: 'author', title: 'Author', slot: 'side', component: LLMAuthor, order: 40 });
  registerWidget({ id: 'synthesize', title: 'Synthesize', slot: 'side', component: SynthesizePanel, order: 45 });
  registerWidget({ id: 'models', title: 'Models', slot: 'side', component: ModelHub, order: 50 });
  registerWidget({ id: 'agent', title: 'Agent', slot: 'side', component: AgentPanel, order: 55 });
  registerWidget({ id: 'plugins', title: 'Plugins', slot: 'side', component: PluginsPanel, order: 60 });

  // Bottom rail (tabs).
  registerWidget({ id: 'metrics', title: 'Metrics', slot: 'bottom', component: TrainingDashboard, order: 10 });
  registerWidget({ id: 'predict', title: 'Predict', slot: 'bottom', component: PredictPanel, order: 20 });
  registerWidget({ id: 'cluster', title: 'Cluster', slot: 'bottom', component: ClusterConsole, order: 30 });
  registerWidget({ id: 'console', title: 'Console', slot: 'bottom', component: Console, order: 40 });
}
