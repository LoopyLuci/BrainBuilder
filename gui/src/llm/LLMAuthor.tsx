import { useState } from 'react';
import { generateGraph } from '../api/models';
import { getComponentDescriptors } from '../api/tauri';
import { convertFromBBIR } from '../canvas/utils';
import { useGraphStore } from '../state/graphStore';
import { useProviderStore } from '../state/providerStore';
import { logError, logInfo } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { ProviderSelector } from './ProviderSelector';

export function LLMAuthor() {
  const [description, setDescription] = useState('');
  const [busy, setBusy] = useState(false);
  const setGraph = useGraphStore((s) => s.setGraph);
  const setTraining = useGraphStore((s) => s.setTraining);
  const selector = useProviderStore((s) => s.selector);

  const generate = async () => {
    if (!description.trim()) return;
    setBusy(true);
    try {
      const json = await generateGraph(description, selector());
      const graph = JSON.parse(json);
      const descriptors = Object.fromEntries((await getComponentDescriptors()).map((d) => [d.name, d]));
      const { nodes, edges } = convertFromBBIR(graph, descriptors);
      setGraph(nodes, edges, graph.graph_id);
      if (graph.training) setTraining(graph.training);
      logInfo(`Generated graph "${graph.name}" (${nodes.length} nodes) from your description.`);
    } catch (e) {
      logError(`LLM graph generation failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Describe a Model">
      <p className="bb-text-muted" style={{ margin: 0 }}>
        Pick a provider below — <strong>Ollama</strong> runs fully local (no key), or connect{' '}
        <strong>OpenCode Go</strong> in the Models panel to use hosted models. The generated graph is validated
        against your real component library before it hits the canvas.
      </p>
      <textarea
        className="bb-textarea"
        data-tutorial="author-description"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        placeholder="e.g. a small transformer block for next-word prediction on short text"
        rows={3}
      />
      <div data-tutorial="author-provider">
        <ProviderSelector />
      </div>
      <Button variant="primary" data-tutorial="author-generate-btn" onClick={generate} disabled={busy || !description.trim()}>
        {busy ? 'Generating…' : 'Generate'}
      </Button>
    </Panel>
  );
}
