import { invoke } from "@tauri-apps/api/tauri";

const NODE_TYPES = [
  { type: "model", label: "Model", color: "bg-blue-500", desc: "GGUF / ONNX" },
  { type: "adapter", label: "Adapter", color: "bg-purple-500", desc: "LoRA / KM" },
  { type: "data", label: "Dataset", color: "bg-emerald-500", desc: "JSONL / corpus" },
  { type: "prompt", label: "Prompt", color: "bg-amber-500", desc: "Text input" },
  { type: "rag", label: "RAG", color: "bg-cyan-500", desc: "Retrieve context" },
  { type: "router", label: "Router", color: "bg-rose-500", desc: "Branch / MoE" },
  { type: "output", label: "Output", color: "bg-slate-500", desc: "Sink" },
  { type: "custom", label: "Custom", color: "bg-indigo-500", desc: "Plugin node" },
];

export default function NodePalette() {
  const add = async (nodeType: string) => {
    try {
      await invoke("add_node", {
        nodeType,
        modelId: null,
        label: nodeType,
        x: 120 + Math.random() * 280,
        y: 100 + Math.random() * 220,
        config: null,
      });
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="p-3 space-y-1">
      <h3 className="panel-title mb-3 px-1">Nodes</h3>
      {NODE_TYPES.map((n) => (
        <button
          key={n.type}
          onClick={() => add(n.type)}
          className="w-full flex items-center gap-2.5 px-2.5 py-2 rounded-lg hover:bg-white/5 text-left transition-colors group"
        >
          <span className={`w-2.5 h-2.5 rounded-full ${n.color} shadow-sm`} />
          <span className="flex-1 min-w-0">
            <span className="block text-sm text-slate-200 group-hover:text-white">
              {n.label}
            </span>
            <span className="block text-[10px] text-slate-500">{n.desc}</span>
          </span>
        </button>
      ))}
      <div className="mt-6 pt-4 border-t border-white/5 px-1">
        <p className="text-[11px] text-slate-500 leading-relaxed">
          Drag files onto the canvas or ask the Concierge to import models.
        </p>
      </div>
    </div>
  );
}
