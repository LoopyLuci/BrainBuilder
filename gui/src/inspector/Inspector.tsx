import { useGraphStore } from '../state/graphStore';
import { Panel } from '../ui/Panel';
import { SchemaForm, hyperparamsToSchema, FormValues } from '../widgets/SchemaForm';

// Renders a node's hyperparameter controls purely from its component
// descriptor, via the shared schema-driven form. New hyperparameters (including
// ones on a synthesized or plugin-provided component) appear automatically with
// no Inspector edit — and the form understands number/boolean/text/select
// types, not just the old number/text pair.
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
  const hyperparams: FormValues = selectedNode.data.hyperparams || {};

  const onChange = (next: FormValues) => {
    updateNodeHyperparams(selectedNode.id, next);
  };

  const schema = descriptor ? hyperparamsToSchema(descriptor.hyperparameters) : { fields: [] };

  return (
    <Panel title={selectedNode.data.label} subtitle={`Component: ${selectedNode.data.component}`}>
      {!descriptor && (
        <p className="bb-text-error">No descriptor found for this component — was it removed from the registry?</p>
      )}

      {descriptor && descriptor.hyperparameters.length === 0 && (
        <p className="bb-text-muted">This component has no hyperparameters.</p>
      )}

      {descriptor && descriptor.hyperparameters.length > 0 && (
        <SchemaForm schema={schema} values={hyperparams} onChange={onChange} />
      )}
    </Panel>
  );
}
