import { registerWidget } from './registry';
import { ComponentPalette } from '../palette/ComponentPalette';
import { InfiniteCanvas } from '../canvas/InfiniteCanvas';
import { Inspector } from '../inspector/Inspector';
import { Console } from '../console/Console';
import { TrainingDashboard } from '../training/TrainingDashboard';
import { DataPanel } from '../data/DataPanel';
import { PredictPanel } from '../predict/PredictPanel';
import { ExperimentsPanel } from '../experiments/ExperimentsPanel';
import { ClusterConsole } from '../cluster/ClusterConsole';
import { LLMAuthor } from '../llm/LLMAuthor';
import { ModelHub } from '../models/ModelHub';
import { IntentPanel } from '../intent/IntentPanel';
import { TemplatesPanel } from '../templates/TemplatesPanel';
import { SynthesizePanel } from '../synthesis/SynthesizePanel';
import { AgentPanel } from '../agent/AgentPanel';
import { PluginsPanel } from './PluginsPanel';
import { LearnPanel } from '../tutorial/LearnPanel';
import { ModelMistressPanel } from '../model-mistress/ModelMistressPanel';
// BrainBuilder/OmniForge ModelBuilder widgets — integrated into gui/src/omniforge-components
// wired into BrainBuilder's widget registry so they can be toggled at
// runtime alongside the built-in panels.
import ConciergePanel from '../omniforge-components/ConciergePanel';
import GenerativePanel from '../omniforge-components/GenerativePanel';
import NodePalette from '../omniforge-components/NodePalette';
import InferenceSandbox from '../omniforge-components/InferenceSandbox';
import KnowledgeModuleManager from '../omniforge-components/KnowledgeModuleManager';
import PluginManager from '../omniforge-components/PluginManager';
import ModelExporter from '../omniforge-components/ModelExporter';
import SelfEditPanel from '../omniforge-components/SelfEditPanel';
import PropertiesPanel from '../omniforge-components/PropertiesPanel';
// External model integrations: PowerManager + CompressionAgent
import { PowerManagerPanel } from '../power-manager/PowerManagerPanel';
import { CompressionAgentPanel } from '../compression-agent/CompressionAgentPanel';
// Next-gen model panels
import { RetrievalAugmentedGenerationPanel } from '../next-gen/RetrievalAugmentedGenerationPanel';
import { AdaptiveReasoningPanel } from '../next-gen/AdaptiveReasoningPanel';
import { KnowledgeGraphPanel } from '../next-gen/KnowledgeGraphPanel';
import { FlowAnalyzerPanel } from '../next-gen/FlowAnalyzerPanel';
import { CounterfactualExplainerPanel } from '../next-gen/CounterfactualExplainerPanel';
import { ContinualLearningPanel } from '../next-gen/ContinualLearningPanel';
import { SymbolicReasoningPanel } from '../next-gen/SymbolicReasoningPanel';
import { TemporalPointProcessPanel } from '../next-gen/TemporalPointProcessPanel';
import { InteractiveExplainabilityPanel } from '../next-gen/InteractiveExplainabilityPanel';
import { CompositionalReasoningPanel } from '../next-gen/CompositionalReasoningPanel';
import { NeuroSymbolicProverPanel } from '../next-gen/NeuroSymbolicProverPanel';
import { FederatedLearningPanel } from '../next-gen/FederatedLearningPanel';
import { PmiAnalyzerPanel } from '../next-gen/PmiAnalyzerPanel';
import { UncertaintyQuantificationPanel } from '../next-gen/UncertaintyQuantificationPanel';
import { HyperparameterOptimizerPanel } from '../next-gen/HyperparameterOptimizerPanel';
import LuciPanel from '../next-gen/LuciPanel';
import LiveRuntimePanel from '../next-gen/LiveRuntimePanel';
import { BotDashboardPanel } from '../bot/BotDashboardPanel';
import { HomeDashboard } from '../home/HomeDashboard';

export function registerBuiltinWidgets() {
  registerWidget({ id: 'palette', title: 'Components', slot: 'palette', component: ComponentPalette, order: 10 });
  registerWidget({ id: 'canvas', title: 'Canvas', slot: 'canvas', component: InfiniteCanvas, order: 10 });

  // Side rail (tabs) - organized by user workflow
  // 1. Start here
  registerWidget({ id: 'learn', title: 'Learn', slot: 'side', component: LearnPanel, order: 5 });
  registerWidget({ id: 'home', title: 'Dashboard', slot: 'side', component: HomeDashboard, order: 6 });
  registerWidget({ id: 'templates', title: 'Templates', slot: 'side', component: TemplatesPanel, order: 7 });
  
  // 2. Build & design
  registerWidget({ id: 'build', title: 'Build', slot: 'side', component: IntentPanel, order: 10 });
  registerWidget({ id: 'inspector', title: 'Inspector', slot: 'side', component: Inspector, order: 15 });
  registerWidget({ id: 'properties', title: 'Properties', slot: 'right', component: PropertiesPanel, order: 10 });
  
  // 3. Data & models
  registerWidget({ id: 'data', title: 'Data', slot: 'side', component: DataPanel, order: 20 });
  registerWidget({ id: 'author', title: 'Author', slot: 'side', component: LLMAuthor, order: 25 });
  registerWidget({ id: 'models', title: 'Models', slot: 'side', component: ModelHub, order: 30 });
  registerWidget({ id: 'synthesize', title: 'Synthesize', slot: 'side', component: SynthesizePanel, order: 35 });
  
  // 4. Advanced features
  registerWidget({ id: 'agent', title: 'Agent', slot: 'side', component: AgentPanel, order: 40 });
  registerWidget({ id: 'bot', title: 'Bot Server', slot: 'side', component: BotDashboardPanel, order: 45 });
  registerWidget({ id: 'luci', title: 'Luci', slot: 'side', component: LuciPanel, order: 50 });
  registerWidget({ id: 'model-mistress', title: 'ModelMistress', slot: 'side', component: ModelMistressPanel, order: 55 });
  registerWidget({ id: 'power-manager', title: 'Power Manager', slot: 'side', component: PowerManagerPanel, order: 56 });
  registerWidget({ id: 'compression-agent', title: 'Compression', slot: 'side', component: CompressionAgentPanel, order: 57 });
  
  // 5. Research & analysis
  registerWidget({ id: 'retrieval-augmented-generation', title: 'RAG', slot: 'side', component: RetrievalAugmentedGenerationPanel, order: 120 });
  registerWidget({ id: 'adaptive-reasoning', title: 'Adaptive Reasoning', slot: 'side', component: AdaptiveReasoningPanel, order: 121 });
  registerWidget({ id: 'knowledge-graph', title: 'Knowledge Graph', slot: 'side', component: KnowledgeGraphPanel, order: 122 });
  registerWidget({ id: 'flow-analyzer', title: 'Flow Analyzer', slot: 'side', component: FlowAnalyzerPanel, order: 123 });
  registerWidget({ id: 'counterfactual-explainer', title: 'Counterfactual', slot: 'side', component: CounterfactualExplainerPanel, order: 124 });
  registerWidget({ id: 'continual-learning', title: 'Continual Learning', slot: 'side', component: ContinualLearningPanel, order: 125 });
  registerWidget({ id: 'symbolic-reasoning', title: 'Symbolic', slot: 'side', component: SymbolicReasoningPanel, order: 126 });
  registerWidget({ id: 'temporal-point-process', title: 'Temporal PP', slot: 'side', component: TemporalPointProcessPanel, order: 127 });
  registerWidget({ id: 'interactive-explainability', title: 'Explainability', slot: 'side', component: InteractiveExplainabilityPanel, order: 128 });
  registerWidget({ id: 'compositional-reasoning', title: 'Compositional', slot: 'side', component: CompositionalReasoningPanel, order: 129 });
  registerWidget({ id: 'neuro-symbolic-prover', title: 'NS Prover', slot: 'side', component: NeuroSymbolicProverPanel, order: 130 });
  registerWidget({ id: 'federated-learning', title: 'Federated', slot: 'side', component: FederatedLearningPanel, order: 131 });
  registerWidget({ id: 'pmi-analyzer', title: 'PMI', slot: 'side', component: PmiAnalyzerPanel, order: 132 });
  registerWidget({ id: 'uncertainty-quantification', title: 'Uncertainty', slot: 'side', component: UncertaintyQuantificationPanel, order: 133 });
  registerWidget({ id: 'hyperparameter-optimizer', title: 'HPO', slot: 'side', component: HyperparameterOptimizerPanel, order: 134 });
  
  // 6. System & plugins
  registerWidget({ id: 'plugins', title: 'Plugins', slot: 'side', component: PluginsPanel, order: 200 });
  registerWidget({ id: 'concierge', title: 'Concierge', slot: 'side', component: ConciergePanel, order: 201 });
  registerWidget({ id: 'genui', title: 'Gen UI', slot: 'side', component: GenerativePanel, order: 202 });
  registerWidget({ id: 'knowledge', title: 'Knowledge', slot: 'side', component: KnowledgeModuleManager, order: 203 });
  registerWidget({ id: 'model-export', title: 'Export', slot: 'side', component: ModelExporter, order: 204 });
  registerWidget({ id: 'self-edit', title: 'Self-Edit', slot: 'side', component: SelfEditPanel, order: 205 });

  // Bottom rail (tabs) - outputs & monitoring
  registerWidget({ id: 'metrics', title: 'Metrics', slot: 'bottom', component: TrainingDashboard, order: 10 });
  registerWidget({ id: 'predict', title: 'Predict', slot: 'bottom', component: PredictPanel, order: 20 });
  registerWidget({ id: 'experiments', title: 'History', slot: 'bottom', component: ExperimentsPanel, order: 25 });
  registerWidget({ id: 'cluster', title: 'Cluster', slot: 'bottom', component: ClusterConsole, order: 30 });
  registerWidget({ id: 'inference', title: 'Inference', slot: 'bottom', component: InferenceSandbox, order: 35 });
  registerWidget({ id: 'live-runtime', title: 'Live Runtime', slot: 'bottom', component: LiveRuntimePanel, order: 38 });
  registerWidget({ id: 'console', title: 'Console', slot: 'bottom', component: Console, order: 40 });
}
