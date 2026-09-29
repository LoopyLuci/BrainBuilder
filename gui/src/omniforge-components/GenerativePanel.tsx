import { useGenUI } from "../generative/store";
import { GenRenderer } from "../generative/Renderer";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import type { GenNode } from "../generative/types";

export default function GenerativePanel() {
  const surfaces = useGenUI((s) => s.surfaces);
  const clear = useGenUI((s) => s.clear);
  const remove = useGenUI((s) => s.remove);
  const push = useGenUI((s) => s.push);
  const replace = useGenUI((s) => s.replace);

  useEffect(() => {
    const u = listen<{ id: string; title: string; root: GenNode }>("genui-push", (e) => {
      push(e.payload.title, e.payload.root);
    });
    return () => { u.then((f) => f()); };
  }, [push]);


  const demo = () => {
    const root: GenNode = {
      type: "stack",
      direction: "col",
      gap: 12,
      children: [
        { type: "text", variant: "title", content: "Generated workspace" },
        { type: "text", variant: "muted", content: "Emitted by the Concierge at runtime." },
        {
          type: "stack",
          direction: "row",
          gap: 10,
          children: [
            { type: "metric", label: "Models", value: 3, hint: "in registry" },
            { type: "metric", label: "Nodes", value: 12, hint: "on canvas" },
            { type: "metric", label: "Jobs", value: 1, hint: "training" },
          ],
        },
        { type: "progress", value: 0.64, label: "Pipeline health" },
        {
          type: "card",
          title: "Quick actions",
          children: [
            {
              type: "stack",
              direction: "row",
              gap: 8,
              children: [
                { type: "button", label: "Run graph", action: "run_graph", variant: "primary" },
                { type: "button", label: "Clear", action: "clear_gen", variant: "ghost" },
              ],
            },
          ],
        },
      ],
    };
    push("Demo surface", root);
  };

  const onAction = async (action: string, payload?: unknown) => {
    if (action === "clear_gen") {
      clear();
      return;
    }
    if (action === "run_graph") {
      try {
        const snap = await invoke<{ nodes: unknown[]; edges: unknown[] }>("get_canvas");
        await invoke("execute_canvas_graph", {
          graph: { nodes: snap.nodes ?? [], edges: snap.edges ?? [] },
        });
      } catch (e) {
        console.error(e);
      }
      return;
    }
    // Forward unknown actions to Concierge as natural language
    try {
      await invoke("concierge_chat", {
        message: `UI action: ${action} ${payload ? JSON.stringify(payload) : ""}`,
      });
    } catch {
      /* ignore */
    }
  };

  return (
    <div className="h-full flex flex-col">
      <div className="px-4 py-3 border-b border-white/5 flex items-center justify-between">
        <div>
          <div className="text-sm font-medium">Generative UI</div>
          <div className="text-[11px] text-slate-500">
            Surfaces created by the agent or demos · live-swappable
          </div>
        </div>
        <div className="flex gap-2">
          <button className="btn-ghost text-xs" onClick={demo}>
            Demo
          </button>
          <button className="btn-ghost text-xs" onClick={clear}>
            Clear
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {surfaces.length === 0 && (
          <div className="card text-sm text-slate-400">
            No generative surfaces yet. Ask the Concierge to{" "}
            <span className="text-blue-300">“generate a status dashboard”</span> or click Demo.
          </div>
        )}
        {surfaces.map((s) => (
          <div key={s.id} className="glass rounded-xl p-4 gen-enter space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="chip">live</span>
                <span className="text-sm font-medium">{s.title}</span>
              </div>
              <button className="btn-ghost text-xs" onClick={() => remove(s.id)}>
                Dismiss
              </button>
            </div>
            <GenRenderer node={s.root} onAction={onAction} />
          </div>
        ))}
      </div>
    </div>
  );
}
