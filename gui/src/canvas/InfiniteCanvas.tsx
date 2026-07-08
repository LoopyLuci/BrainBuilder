import { useCallback } from 'react';
import ReactFlow, { Background, Controls, Connection, Node, Edge } from 'reactflow';
import 'reactflow/dist/style.css';
import { useGraphStore } from '../state/graphStore';
import CustomNode from './NodeComponent';
import { convertToBBIR } from './utils';
import { executeGraph, validateGraph } from '../api/tauri';
import { GraphToolbar } from './GraphToolbar';
import { useAutosave } from './useAutosave';
import { logError, logInfo } from '../console/logStore';
import { Button } from '../ui/Button';
import { CanvasEmptyState } from './CanvasEmptyState';

// Why a connection was rejected, shown as a small toast near the cursor so a
// blocked drag never just silently snaps back with no explanation.
function explainRejection(
  nodes: Node[],
  edges: Edge[],
  conn: Connection,
): string | null {
  if (!conn.source || !conn.target || !conn.sourceHandle || !conn.targetHandle) return 'Drag from a dot on one box to a dot on another.';
  if (conn.source === conn.target) return "A box can't connect to itself.";
  const sourceNode = nodes.find((n) => n.id === conn.source);
  const targetNode = nodes.find((n) => n.id === conn.target);
  const sourcePort = sourceNode?.data?.descriptor?.outputs?.find((p: any) => p.name === conn.sourceHandle);
  const targetPort = targetNode?.data?.descriptor?.inputs?.find((p: any) => p.name === conn.targetHandle);
  if (!sourcePort || !targetPort) return "Those two dots don't belong to a connectable input/output.";
  if (targetPort.role !== 'data') return 'That input is a parameter, not a connectable data port.';
  const alreadyWired = edges.some((e) => e.target === conn.target && e.targetHandle === conn.targetHandle);
  if (alreadyWired) return 'That input already has a connection — remove it first if you want to swap it.';
  if (sourcePort.dtype !== targetPort.dtype) {
    return `These don't fit together: one box outputs "${sourcePort.dtype}", the other expects "${targetPort.dtype}".`;
  }
  return null;
}

const nodeTypes = { default: CustomNode };

export function InfiniteCanvas() {
  const graphId = useGraphStore((s) => s.graphId);
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const onNodesChange = useGraphStore((s) => s.onNodesChange);
  const onEdgesChange = useGraphStore((s) => s.onEdgesChange);
  const addEdgeToStore = useGraphStore((s) => s.addEdgeToStore);
  const setSelectedNode = useGraphStore((s) => s.setSelectedNode);

  // Constraint-based wiring: a drag that would create an inconsistent graph
  // (mismatched data types, an already-filled input, a self-loop) is refused
  // at the drop itself — react-flow shows it as an invalid drop target — so
  // there's no invalid-graph state to recover from after the fact.
  const isValidConnection = useCallback(
    (conn: Connection) => explainRejection(nodes, edges, conn) === null,
    [nodes, edges],
  );

  const onConnect = useCallback(
    (params: Connection) => {
      const reason = explainRejection(nodes, edges, params);
      if (reason) {
        logError(`Couldn't connect those: ${reason}`);
        return;
      }
      const newEdge: Edge = { ...params, id: `e-${params.source}-${params.target}-${Date.now()}` } as Edge;
      addEdgeToStore(newEdge);
    },
    [addEdgeToStore, nodes, edges],
  );

  const onNodeClick = (_: React.MouseEvent, node: Node) => setSelectedNode(node.id);
  const training = useGraphStore((s) => s.training);
  useAutosave();

  const runValidate = async () => {
    const bbir = convertToBBIR(nodes, edges, graphId, 'untitled', training);
    try {
      await validateGraph(bbir);
      logInfo('Graph is valid: ports, dtypes, and shapes are consistent.');
    } catch (err) {
      logError(`Validation failed: ${err}`);
    }
  };

  const exportAndTrain = async () => {
    if (!training.data_source.path_or_uri) {
      logError('Select a dataset in the Data & Training panel first.');
      return;
    }
    const bbir = convertToBBIR(nodes, edges, graphId, 'untitled', training);
    try {
      await validateGraph(bbir);
    } catch (err) {
      logError(`Validation failed: ${err}`);
      return;
    }
    try {
      logInfo('Training started…');
      await executeGraph(bbir);
      logInfo('Training finished.');
    } catch (err) {
      logError(`Training failed: ${err}`);
    }
  };

  return (
    <div style={{ width: '100%', height: '100%' }}>
      <GraphToolbar />
      <div className="bb-canvas-toolbar" style={{ top: 48 }}>
        <Button variant="secondary" data-tutorial="validate-btn" onClick={runValidate}>Validate</Button>
        <Button variant="primary" data-tutorial="export-train-btn" onClick={exportAndTrain}>Export &amp; Train</Button>
      </div>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        isValidConnection={isValidConnection}
        onNodeClick={onNodeClick}
        nodeTypes={nodeTypes}
        fitView
      >
        <Background />
        <Controls />
      </ReactFlow>
      {nodes.length === 0 && <CanvasEmptyState />}
    </div>
  );
}
