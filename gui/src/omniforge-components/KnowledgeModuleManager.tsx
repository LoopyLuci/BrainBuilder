import { useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";

export default function KnowledgeModuleManager() {
  const [result, setResult] = useState("");
  const create = async () => {
    try {
      const id = await invoke<string>("create_km", {
        baseModel: "base-model",
        adapterPath: "output/demo-km/adapter",
        name: "demo-km",
        rank: 8,
        alpha: 16,
        outputPath: "output/demo-km.km",
      });
      setResult(`Created: ${id}`);
    } catch (e: any) {
      setResult(`Error: ${e}`);
    }
  };
  return (
    <div className="p-4">
      <h2 className="text-xl font-semibold mb-2">Knowledge Modules</h2>
      <button onClick={create} className="bg-purple-600 px-3 py-1 rounded text-sm">Package .km</button>
      <p className="mt-2 text-sm text-slate-300">{result}</p>
    </div>
  );
}
