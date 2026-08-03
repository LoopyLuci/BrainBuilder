import { useCallback, useEffect, useState } from "react";
import ReactFlow, {
  Controls,
  Background,
  MiniMap,
  useNodesState,
  useEdgesState,
  addEdge,
  Connection,
  Node,
  Edge,
  BackgroundVariant,
} from "reactflow";
import "reactflow/dist/style.css";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";

interface Props {
  onSelectNode: (id: string | null) => void;
}

export default function Canvas({ onSelectNode }: Props) {
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  const onConnect = useCallback(
    (params: Connection) => {
      setEdges((eds) => addEdge({ ...params, animated: true }, eds));
      if (params.source && params.target) {
        invoke("connect_nodes", {
          sourceNode: params.source,
          sourceSocket: params.sourceHandle ?? "output",
          targetNode: params.target,
          targetSocket: params.targetHandle ?? "input",
        }).catch(console.error);
      }
    },
    [setEdges]
  );

  // Sync canvas from backend (Concierge tool calls emit events)
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<{ nodes: Node[]; edges: Edge[] }>("canvas-update", (event) => {
      if (event.payload.nodes) setNodes(event.payload.nodes);
      if (event.payload.edges) setEdges(event.payload.edges);
    }).then((fn) => {
      unlisten = fn;
    });
    // Initial load
    invoke<{ nodes: Node[]; edges: Edge[] }>("get_canvas")
      .then((snap) => {
        if (snap?.nodes) setNodes(snap.nodes);
        if (snap?.edges) setEdges(snap.edges);
      })
      .catch(() => {});
    return () => {
      unlisten?.();
    };
  }, [setNodes, setEdges]);

  return (
    <div className="h-full w-full">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={(_, n) => onSelectNode(n.id)}
        onPaneClick={() => onSelectNode(null)}
        fitView
        proOptions={{ hideAttribution: true }}
      >
        <Controls className="!bg-slate-800 !border-slate-600 !shadow-none" />
        <MiniMap
          className="!bg-slate-800 !border-slate-600"
          nodeColor="#3b82f6"
        />
        <Background variant={BackgroundVariant.Dots} gap={16} size={1} color="#334155" />
      </ReactFlow>
    </div>
  );
}
