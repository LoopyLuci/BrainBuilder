import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

export default function InferenceSandbox() {
  const [outputs, setOutputs] = useState<Record<string, unknown>>({});
  const [streams, setStreams] = useState<Record<string, string>>({});
  const [status, setStatus] = useState("idle");
  const [activeNode, setActiveNode] = useState<string | null>(null);

  useEffect(() => {
    const unsubs = [
      listen<{ graphNodeCount: number }>("inference-start", () => {
        setStatus("running");
        setOutputs({});
        setStreams({});
      }),
      listen<{ nodeId: string; nodeType: string }>("inference-node-start", (e) => {
        setActiveNode(e.payload.nodeId);
      }),
      listen<{ nodeId: string; token: string; index: number }>("inference-token", (e) => {
        const { nodeId, token } = e.payload;
        setStreams((prev) => ({
          ...prev,
          [nodeId]: (prev[nodeId] ?? "") + token,
        }));
      }),
      listen<{ nodeId: string; output: unknown }>("inference-output", (e) => {
        setOutputs((prev) => ({ ...prev, [e.payload.nodeId]: e.payload.output }));
      }),
      listen<{ nodeId: string; error: unknown }>("inference-error", (e) => {
        setOutputs((prev) => ({
          ...prev,
          [e.payload.nodeId]: { error: e.payload.error },
        }));
      }),
      listen("inference-complete", () => {
        setStatus("done");
        setActiveNode(null);
      }),
    ];
    return () => {
      unsubs.forEach((p) => p.then((f) => f()));
    };
  }, []);

  const run = async () => {
    setStatus("running");
    setOutputs({});
    setStreams({});
    try {
      const snap = await invoke<{ nodes: any[]; edges: any[] }>("get_canvas");
      await invoke("execute_canvas_graph", {
        graph: { nodes: snap.nodes ?? [], edges: snap.edges ?? [] },
      });
    } catch (e: any) {
      setStatus(`error: ${e}`);
    }
  };

  return (
    <div className="p-4 h-full overflow-y-auto">
      <div className="flex items-center gap-3 mb-3">
        <h2 className="text-xl font-semibold">Inference Sandbox</h2>
        <button
          onClick={run}
          className="bg-emerald-600 hover:bg-emerald-500 px-3 py-1 rounded text-sm"
        >
          Run Graph
        </button>
        <span className="text-xs text-slate-400">Status: {status}</span>
        {activeNode && (
          <span className="text-xs text-amber-400">Running: {activeNode}</span>
        )}
      </div>

      {/* Live token streams */}
      {Object.keys(streams).length > 0 && (
        <div className="mb-4 space-y-2">
          <h3 className="text-sm font-medium text-slate-400">Live stream</h3>
          {Object.entries(streams).map(([id, text]) => (
            <div key={id} className="p-2 bg-slate-900 border border-slate-700 rounded text-sm">
              <div className="text-xs text-blue-400 mb-1">{id}</div>
              <div className="whitespace-pre-wrap font-mono text-slate-200">
                {text}
                {activeNode === id && <span className="animate-pulse">▍</span>}
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Final outputs */}
      <div className="space-y-2">
        {Object.entries(outputs).map(([id, out]) => (
          <div key={id} className="p-2 bg-slate-800 rounded text-xs">
            <span className="font-semibold text-blue-300">{id}</span>
            <pre className="mt-1 whitespace-pre-wrap">{JSON.stringify(out, null, 2)}</pre>
          </div>
        ))}
      </div>
    </div>
  );
}
