import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";

interface Props {
  nodeId: string;
}

export default function PropertiesPanel({ nodeId }: Props) {
  const [info, setInfo] = useState<string>("");

  useEffect(() => {
    invoke<string>("inspect_node", { nodeId })
      .then(setInfo)
      .catch((e) => setInfo(String(e)));
  }, [nodeId]);

  return (
    <div className="p-3">
      <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">
        Properties
      </h3>
      <p className="text-xs text-slate-500 mb-2">Node: {nodeId}</p>
      <pre className="text-xs bg-slate-800 p-2 rounded overflow-auto max-h-96 whitespace-pre-wrap">
        {info || "Loading…"}
      </pre>
    </div>
  );
}
