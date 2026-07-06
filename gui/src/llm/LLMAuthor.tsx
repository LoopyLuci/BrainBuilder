import { useState } from 'react';
import { generateGraph } from '../api/models';
import { getComponentDescriptors } from '../api/tauri';
import { convertFromBBIR } from '../canvas/utils';
import { useGraphStore } from '../state/graphStore';
import { logError, logInfo } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';

export function LLMAuthor() {
  const [description, setDescription] = useState('');
  const [model, setModel] = useState('llama3.2');
  const [busy, setBusy] = useState(false);
  const setGraph = useGraphStore((s) => s.setGraph);
  const setTraining = useGraphStore((s) => s.setTraining);

  const generate = async () => {
    if (!description.trim()) return;
    setBusy(true);
    try {
      const json = await generateGraph(description, model);
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
        Local-first: talks to a local Ollama server (no cloud, no API key). Requires{' '}
        <code className="bb-code">ollama serve</code> running and <code className="bb-code">{model}</code> pulled.
      </p>
      <textarea
        className="bb-textarea"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        placeholder="e.g. a small transformer block for next-word prediction on short text"
        rows={3}
      />
      <div className="bb-row">
        <input
          className="bb-input"
          value={model}
          onChange={(e) => setModel(e.target.value)}
          placeholder="ollama model name"
        />
        <Button variant="primary" onClick={generate} disabled={busy || !description.trim()}>
          {busy ? 'Generating…' : 'Generate'}
        </Button>
      </div>
    </Panel>
  );
}
