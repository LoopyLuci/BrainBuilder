import { save, open } from '@tauri-apps/api/dialog';
import { useGraphStore } from '../state/graphStore';
import { convertToBBIR, convertFromBBIR } from './utils';
import { saveGraph, loadGraph } from '../api/tauri';
import { Button } from '../ui/Button';

export function GraphToolbar() {
  const graphId = useGraphStore((s) => s.graphId);
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const training = useGraphStore((s) => s.training);
  const descriptors = useGraphStore((s) => s.descriptors);
  const setGraph = useGraphStore((s) => s.setGraph);
  const setTraining = useGraphStore((s) => s.setTraining);
  const undo = useGraphStore((s) => s.undo);
  const redo = useGraphStore((s) => s.redo);
  const past = useGraphStore((s) => s.past);
  const future = useGraphStore((s) => s.future);

  const onSave = async () => {
    const path = await save({ filters: [{ name: 'BrainBuilder Graph', extensions: ['bbir.edn'] }] });
    if (!path) return;
    await saveGraph(path, convertToBBIR(nodes, edges, graphId, 'untitled', training));
  };

  const onLoad = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'BrainBuilder Graph', extensions: ['edn'] }],
    });
    if (typeof selected !== 'string') return;
    const graph = await loadGraph(selected);
    const { nodes: n, edges: e } = convertFromBBIR(graph, descriptors);
    setGraph(n, e, graph.graph_id);
    if (graph.training) setTraining(graph.training);
  };

  const onNew = () => {
    setGraph([], []);
  };

  return (
    <div className="bb-canvas-toolbar">
      <Button variant="secondary" onClick={onNew}>New</Button>
      <Button variant="secondary" onClick={onSave}>Save…</Button>
      <Button variant="secondary" onClick={onLoad}>Load…</Button>
      <div className="bb-toolbar-divider" />
      <Button variant="ghost" onClick={undo} disabled={past.length === 0} title="Undo">↶ Undo</Button>
      <Button variant="ghost" onClick={redo} disabled={future.length === 0} title="Redo">↷ Redo</Button>
    </div>
  );
}
