import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";

interface Plugin {
  name: string;
  version: string;
  description: string;
  author: string;
}

export default function PluginManager() {
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [out, setOut] = useState("");

  useEffect(() => {
    invoke<Plugin[]>("get_plugins").then(setPlugins).catch(console.error);
  }, []);

  const run = async (name: string) => {
    try {
      const r = await invoke<string>("run_plugin_tool", {
        plugin: name,
        tool: "analyze_sentiment",
        arguments: JSON.stringify({ text: "This is a great product, I love it!" }),
      });
      setOut(r);
    } catch (e: any) {
      setOut(String(e));
    }
  };

  return (
    <div className="p-4">
      <h2 className="text-xl font-semibold mb-2">Plugins</h2>
      <ul className="space-y-2">
        {plugins.map((p) => (
          <li key={p.name} className="bg-slate-800 p-2 rounded flex justify-between items-center">
            <div>
              <div className="font-medium">{p.name}</div>
              <div className="text-xs text-slate-400">{p.description}</div>
            </div>
            <button onClick={() => run(p.name)} className="text-sm bg-slate-600 px-2 py-1 rounded">
              Run
            </button>
          </li>
        ))}
        {!plugins.length && <li className="text-slate-500 text-sm">No plugins found.</li>}
      </ul>
      {out && <pre className="mt-3 text-xs bg-slate-900 p-2 rounded">{out}</pre>}
    </div>
  );
}
