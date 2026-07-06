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

const nodeTypes = { default: CustomNode };

export function InfiniteCanvas() {
  const graphId = useGraphStore((s) => s.graphId);
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const onNodesChange = useGraphStore((s) => s.onNodesChange);
  const onEdgesChange = useGraphStore((s) => s.onEdgesChange);
  const addEdgeToStore = useGraphStore((s) => s.addEdgeToStore);
  const setSelectedNode = useGraphStore((s) => s.setSelectedNode);

  const onConnect = useCallback(
    (params: Connection) => {
      const newEdge: Edge = { ...params, id: `e-${params.source}-${params.target}-${Date.now()}` } as Edge;
      addEdgeToStore(newEdge);
    },
    [addEdgeToStore],
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
        <Button variant="secondary" onClick={runValidate}>Validate</Button>
        <Button variant="primary" onClick={exportAndTrain}>Export &amp; Train</Button>
      </div>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onNodeClick={onNodeClick}
        nodeTypes={nodeTypes}
        fitView
      >
        <Background />
        <Controls />
      </ReactFlow>
    </div>
  );
}
