import { useState } from "react";
import Canvas from "./components/Canvas";
import ConciergePanel from "./components/ConciergePanel";
import NodePalette from "./components/NodePalette";
import PropertiesPanel from "./components/PropertiesPanel";
import TrainingDashboard from "./components/TrainingDashboard";
import InferenceSandbox from "./components/InferenceSandbox";
import KnowledgeModuleManager from "./components/KnowledgeModuleManager";
import PluginManager from "./components/PluginManager";
import ModelExporter from "./components/ModelExporter";
import GenerativePanel from "./components/GenerativePanel";
import SelfEditPanel from "./components/SelfEditPanel";

type Tab =
  | "canvas"
  | "training"
  | "inference"
  | "km"
  | "plugins"
  | "export"
  | "genui"
  | "self";

const TABS: { id: Tab; label: string; hint?: string }[] = [
  { id: "canvas", label: "Canvas" },
  { id: "training", label: "Training" },
  { id: "inference", label: "Inference" },
  { id: "km", label: "KMs" },
  { id: "plugins", label: "Plugins" },
  { id: "export", label: "Export" },
  { id: "genui", label: "Gen UI", hint: "Generative surfaces" },
  { id: "self", label: "Self-Edit", hint: "Hot reload sources" },
];

export default function App() {
  const [tab, setTab] = useState<Tab>("canvas");
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [showConcierge, setShowConcierge] = useState(true);

  return (
    <div className="h-screen w-screen flex flex-col text-slate-100 overflow-hidden">
      {/* Top bar */}
      <header className="h-13 flex items-center justify-between px-4 py-2 border-b border-white/5 glass-strong shrink-0">
        <div className="flex items-center gap-4 min-w-0">
          <div className="flex items-center gap-2.5">
            <div className="w-7 h-7 rounded-lg bg-gradient-to-br from-blue-500 to-violet-600 flex items-center justify-center text-xs font-bold shadow-lg shadow-blue-500/20">
              Ω
            </div>
            <span className="text-base font-semibold tracking-tight bg-gradient-to-r from-blue-300 to-violet-300 bg-clip-text text-transparent">
              OmniForge
            </span>
          </div>
          <nav className="flex gap-0.5 ml-2 overflow-x-auto">
            {TABS.map((t) => (
              <button
                key={t.id}
                title={t.hint}
                onClick={() => setTab(t.id)}
                className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-all duration-150 ${
                  tab === t.id
                    ? "bg-white/10 text-white shadow-inner"
                    : "text-slate-400 hover:text-slate-200 hover:bg-white/5"
                }`}
              >
                {t.label}
              </button>
            ))}
          </nav>
        </div>
        <button
          onClick={() => setShowConcierge((v) => !v)}
          className="btn-ghost text-xs shrink-0"
        >
          {showConcierge ? "Hide Concierge" : "Show Concierge"}
        </button>
      </header>

      <div className="flex-1 flex min-h-0">
        {tab === "canvas" && (
          <aside className="w-48 border-r border-white/5 glass shrink-0 overflow-y-auto">
            <NodePalette />
          </aside>
        )}

        <main className="flex-1 min-w-0 relative">
          {tab === "canvas" && <Canvas onSelectNode={setSelectedNodeId} />}
          {tab === "training" && <TrainingDashboard />}
          {tab === "inference" && <InferenceSandbox />}
          {tab === "km" && <KnowledgeModuleManager />}
          {tab === "plugins" && <PluginManager />}
          {tab === "export" && <ModelExporter />}
          {tab === "genui" && <GenerativePanel />}
          {tab === "self" && <SelfEditPanel />}
        </main>

        <div className="flex shrink-0">
          {tab === "canvas" && selectedNodeId && (
            <aside className="w-64 border-l border-white/5 glass overflow-y-auto">
              <PropertiesPanel nodeId={selectedNodeId} />
            </aside>
          )}
          {showConcierge && (
            <aside className="w-[340px] border-l border-white/5 flex flex-col">
              <ConciergePanel />
            </aside>
          )}
        </div>
      </div>
    </div>
  );
}
