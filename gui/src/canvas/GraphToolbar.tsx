import { useEffect } from 'react';
import { save, open } from '@tauri-apps/api/dialog';
import { useGraphStore } from '../state/graphStore';
import { convertToBBIR, convertFromBBIR } from './utils';
import { saveGraph, loadGraph } from '../api/tauri';
import { logError, logInfo } from '../console/logStore';
import { Button } from '../ui/Button';
import { confirmAction } from '../ui/confirmStore';

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
    let path: string | null;
    try {
      path = await save({ filters: [{ name: 'BrainBuilder Graph', extensions: ['bbir.edn'] }] });
    } catch (e) {
      logError(`Couldn't open the save dialog: ${e}`);
      return;
    }
    if (!path) return;
    try {
      await saveGraph(path, convertToBBIR(nodes, edges, graphId, 'untitled', training));
      logInfo(`Graph saved to ${path}.`);
    } catch (e) {
      logError(`Saving the graph failed: ${e}`);
    }
  };

  const onLoad = async () => {
    if (nodes.length > 0) {
      const ok = await confirmAction({
        title: 'Replace what you have on screen?',
        body:
          "Loading a different file will remove what's currently on the canvas. This can't be undone with " +
          "Ctrl+Z once you load. Save your current work first if you're not sure.",
        confirmLabel: 'Yes, load the other file',
      });
      if (!ok) return;
    }
    let selected: string | string[] | null;
    try {
      selected = await open({
        multiple: false,
        filters: [{ name: 'BrainBuilder Graph', extensions: ['edn'] }],
      });
    } catch (e) {
      logError(`Couldn't open the load dialog: ${e}`);
      return;
    }
    if (typeof selected !== 'string') return;
    try {
      const graph = await loadGraph(selected);
      const { nodes: n, edges: e } = convertFromBBIR(graph, descriptors);
      setGraph(n, e, graph.graph_id);
      if (graph.training) setTraining(graph.training);
      logInfo(`Loaded graph "${graph.name}" (${n.length} nodes).`);
    } catch (e) {
      logError(`Loading the graph failed: ${e}`);
    }
  };

  const onNew = async () => {
    if (nodes.length > 0) {
      const ok = await confirmAction({
        title: 'Start a blank canvas?',
        body: "This clears everything you've built here so far, and Ctrl+Z won't bring it back. If you want to keep this model, click Cancel and use Save first.",
        confirmLabel: 'Yes, start blank',
      });
      if (!ok) return;
    }
    setGraph([], []);
  };

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)) return;
      if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== 'z') return;
      e.preventDefault();
      if (e.shiftKey) redo();
      else undo();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [undo, redo]);

  return (
    <div className="bb-canvas-toolbar">
      <Button variant="secondary" data-tutorial="graph-new-btn" onClick={onNew}>New</Button>
      <Button variant="secondary" data-tutorial="graph-save-btn" onClick={onSave}>Save…</Button>
      <Button variant="secondary" data-tutorial="graph-load-btn" onClick={onLoad}>Load…</Button>
      <div className="bb-toolbar-divider" />
      <Button variant="ghost" onClick={undo} disabled={past.length === 0} title="Undo">↶ Undo</Button>
      <Button variant="ghost" onClick={redo} disabled={future.length === 0} title="Redo">↷ Redo</Button>
    </div>
  );
}
