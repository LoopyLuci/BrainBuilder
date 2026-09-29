import { useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";

export default function ModelExporter() {
  const [msg, setMsg] = useState("");
  const exportBundle = async () => {
    try {
      const snap = await invoke<{ nodes: any[]; edges: any[] }>("get_canvas");
      await invoke("export_model_bundle", {
        baseModel: "models/base.gguf",
        kmPaths: ["output/demo-km.km"],
        graph: JSON.stringify(snap),
        output: "output/omniforge-bundle.zip",
      });
      setMsg("Exported to output/omniforge-bundle.zip");
    } catch (e: any) {
      setMsg(`Error: ${e}`);
    }
  };
  return (
    <div className="p-4">
      <h2 className="text-xl font-semibold mb-2">Export Bundle</h2>
      <button onClick={exportBundle} className="bg-amber-600 px-3 py-1 rounded text-sm">
        Export
      </button>
      <p className="mt-2 text-sm">{msg}</p>
    </div>
  );
}
