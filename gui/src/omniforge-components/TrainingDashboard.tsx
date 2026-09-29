import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

export default function TrainingDashboard() {
  const [jobId, setJobId] = useState<string | null>(null);
  const [status, setStatus] = useState("Idle");
  const [logs, setLogs] = useState<string[]>([]);
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    const u1 = listen<{ jobId: string; line: string }>("training-progress", (e) => {
      const line = e.payload.line;
      setLogs((prev) => [...prev.slice(-200), line]);
      const m = line.match(/PROGRESS:\s*([0-9.]+)/);
      if (m) setProgress(Math.min(1, parseFloat(m[1])));
      if (line.startsWith("LOSS:")) setStatus(line);
    });
    const u2 = listen<{ jobId: string; success: boolean }>("training-complete", (e) => {
      setStatus(e.payload.success ? "Complete" : "Failed");
      setProgress(e.payload.success ? 1 : progress);
    });
    const u3 = listen<{ jobId: string; error: string }>("training-error", (e) => {
      setLogs((prev) => [...prev, `ERROR: ${e.payload.error}`]);
    });
    return () => {
      u1.then((f) => f());
      u2.then((f) => f());
      u3.then((f) => f());
    };
  }, [progress]);

  const start = async () => {
    setStatus("Starting…");
    setLogs([]);
    setProgress(0);
    try {
      const id = await invoke<string>("start_training", {
        config: {
          base_model: "gpt2",
          dataset_path: "data/train.jsonl",
          output_km_path: "output/demo-km",
          recipe: "lora",
          rank: 8,
          alpha: 16,
          learning_rate: 0.0002,
          epochs: 1,
        },
      });
      setJobId(id);
      setStatus(`Running ${id}`);
    } catch (e: any) {
      setStatus(`Error: ${e}`);
    }
  };

  return (
    <div className="h-full p-6 overflow-y-auto">
      <h2 className="text-xl font-semibold mb-2">Training Dashboard</h2>
      <p className="text-slate-400 text-sm mb-4">
        Launches LoRA/QLoRA via the bundled Python trainer (demo mode if transformers are missing).
      </p>
      <button onClick={start} className="bg-blue-600 hover:bg-blue-500 px-4 py-2 rounded text-sm">
        Start LoRA job
      </button>
      <div className="mt-4 p-4 bg-slate-800 rounded space-y-2">
        <p className="text-sm">Status: <span className="text-blue-300">{status}</span></p>
        {jobId && <p className="text-xs text-slate-500">Job: {jobId}</p>}
        <div className="w-full bg-slate-700 h-2 rounded overflow-hidden">
          <div className="bg-blue-500 h-full transition-all" style={{ width: `${progress * 100}%` }} />
        </div>
        <pre className="text-xs max-h-64 overflow-auto text-slate-300 whitespace-pre-wrap">
          {logs.join("\n") || "No logs yet."}
        </pre>
      </div>
    </div>
  );
}
