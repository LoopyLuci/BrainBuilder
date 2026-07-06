import { useGraphStore } from '../state/graphStore';
import { HyperParamSummary } from '../api/tauri';
import { Panel } from '../ui/Panel';

export function Inspector() {
  const selectedNode = useGraphStore((s) => s.nodes.find((n) => n.id === s.selectedNode));
  const updateNodeHyperparams = useGraphStore((s) => s.updateNodeHyperparams);

  if (!selectedNode) {
    return (
      <Panel>
        <div className="bb-empty">Select a node to edit its hyperparameters</div>
      </Panel>
    );
  }

  const descriptor = selectedNode.data.descriptor;
  const hyperparams = selectedNode.data.hyperparams || {};

  const setParam = (name: string, value: unknown) => {
    updateNodeHyperparams(selectedNode.id, { ...hyperparams, [name]: value });
  };

  return (
    <Panel title={selectedNode.data.label} subtitle={`Component: ${selectedNode.data.component}`}>
      {!descriptor && (
        <p className="bb-text-error">No descriptor found for this component — was it removed from the registry?</p>
      )}

      {descriptor?.hyperparameters.length === 0 && (
        <p className="bb-text-muted">This component has no hyperparameters.</p>
      )}

      {descriptor?.hyperparameters.map((hp: HyperParamSummary) => {
        const value = hyperparams[hp.name] ?? hp.default;
        const isNumeric = hp.param_type === 'int' || hp.param_type === 'float';
        return (
          <div key={hp.name} className="bb-field">
            <label className="bb-label">
              {hp.name} <small>({hp.param_type})</small>
            </label>
            <input
              className="bb-input"
              type={isNumeric ? 'number' : 'text'}
              value={value ?? ''}
              onChange={(e) => setParam(hp.name, isNumeric ? Number(e.target.value) : e.target.value)}
            />
          </div>
        );
      })}
    </Panel>
  );
}
