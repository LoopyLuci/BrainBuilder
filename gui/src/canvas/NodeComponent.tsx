import { memo } from 'react';
import { Handle, Position, NodeProps } from 'reactflow';
import { ComponentSummary } from '../api/tauri';

// Renders one connectable target handle per *data* input port and one source
// handle per output port, with handle ids = real port names. Parameter ports
// (e.g. linear's `weight`) are shown as non-connectable labels since the
// runtime manages them. This is what lets convertToBBIR emit real port names
// that the backend's validate_graph accepts.
export const CustomNode = memo(({ data, selected }: NodeProps) => {
  const descriptor: ComponentSummary | undefined = data.descriptor;
  const dataInputs = descriptor?.inputs.filter((p) => p.role === 'data') ?? [];
  const paramInputs = descriptor?.inputs.filter((p) => p.role === 'parameter') ?? [];
  const outputs = descriptor?.outputs ?? [];

  const spread = (count: number, i: number) =>
    count <= 1 ? '50%' : `${((i + 1) / (count + 1)) * 100}%`;

  return (
    <div className={`bb-node ${selected ? 'bb-node--selected' : ''}`}>
      {dataInputs.map((p, i) => (
        <Handle
          key={`in-${p.name}`}
          id={p.name}
          type="target"
          position={Position.Left}
          className="bb-node__handle"
          style={{ top: spread(dataInputs.length, i) }}
        />
      ))}
      <div className="bb-node__meta">{descriptor?.meta_type ?? 'component'}</div>
      <div className="bb-node__label">{data.label}</div>
      {paramInputs.length > 0 && (
        <div className="bb-node__params">params: {paramInputs.map((p) => p.name).join(', ')}</div>
      )}
      {outputs.map((p, i) => (
        <Handle
          key={`out-${p.name}`}
          id={p.name}
          type="source"
          position={Position.Right}
          className="bb-node__handle"
          style={{ top: spread(outputs.length, i) }}
        />
      ))}
    </div>
  );
});

export default CustomNode;
