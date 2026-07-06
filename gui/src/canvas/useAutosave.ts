import { useEffect, useRef } from 'react';
import { appDataDir, join } from '@tauri-apps/api/path';
import { createDir, exists } from '@tauri-apps/api/fs';
import { useGraphStore } from '../state/graphStore';
import { convertToBBIR, convertFromBBIR } from './utils';
import { saveGraph, loadGraph } from '../api/tauri';

const AUTOSAVE_INTERVAL_MS = 30_000;
const AUTOSAVE_FILENAME = 'autosave.bbir.edn';
// Bundled example (gui/examples/), resolved relative to the Tauri backend's
// working directory — same convention `components_dir` uses in main.rs
// (cwd = gui/, so `../components` reaches the top-level components/ dir and
// `examples/...` reaches gui/examples/ directly).
const FIRST_RUN_EXAMPLE_PATH = 'examples/first_run.bbir.edn';

async function autosavePath(): Promise<string> {
  const dir = await appDataDir();
  if (!(await exists(dir))) await createDir(dir, { recursive: true });
  return join(dir, AUTOSAVE_FILENAME);
}

/** Restores the last autosaved graph on startup (once descriptors are loaded,
 * so nodes can be re-hydrated with their component descriptors); if there's
 * no autosave yet (first launch), loads the bundled example instead so the
 * app trains something real out of the box rather than opening to an empty
 * canvas. Periodically autosaves thereafter — closing the app never loses
 * work. */
export function useAutosave() {
  const graphId = useGraphStore((s) => s.graphId);
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const training = useGraphStore((s) => s.training);
  const descriptors = useGraphStore((s) => s.descriptors);
  const setGraph = useGraphStore((s) => s.setGraph);
  const setTraining = useGraphStore((s) => s.setTraining);
  const restored = useRef(false);
  const descriptorsReady = Object.keys(descriptors).length > 0;

  useEffect(() => {
    if (restored.current || !descriptorsReady) return;
    restored.current = true;
    (async () => {
      try {
        const path = await autosavePath();
        const source = (await exists(path)) ? path : FIRST_RUN_EXAMPLE_PATH;
        const graph = await loadGraph(source);
        const { nodes: n, edges: e } = convertFromBBIR(graph, descriptors);
        if (n.length > 0) {
          setGraph(n, e, graph.graph_id);
          if (graph.training) setTraining(graph.training);
        }
      } catch (err) {
        console.error('Startup graph restore failed:', err);
      }
    })();
  }, [descriptorsReady, descriptors, setGraph, setTraining]);

  useEffect(() => {
    const interval = setInterval(async () => {
      if (nodes.length === 0) return;
      try {
        const path = await autosavePath();
        await saveGraph(path, convertToBBIR(nodes, edges, graphId, 'autosave', training));
      } catch (err) {
        console.error('Autosave failed:', err);
      }
    }, AUTOSAVE_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [graphId, nodes, edges, training]);
}
