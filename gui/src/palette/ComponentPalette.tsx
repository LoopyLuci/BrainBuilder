import { useEffect, useMemo, useState } from 'react';
import { useDrag } from 'react-dnd';
import { getComponentDescriptors, ComponentSummary } from '../api/tauri';
import { useGraphStore } from '../state/graphStore';

function DraggableComponent({ summary }: { summary: ComponentSummary }) {
  const addNode = useGraphStore((s) => s.addNode);
  const [, drag] = useDrag(() => ({
    type: 'component',
    item: { name: summary.name },
  }));
  const dataInputs = summary.inputs.filter((p) => p.role === 'data').length;
  const params = summary.inputs.filter((p) => p.role === 'parameter').length;
  return (
    <div
      ref={drag as unknown as React.Ref<HTMLDivElement>}
      onDoubleClick={() => addNode(summary.name, { x: 120, y: 120 })}
      title={`${dataInputs} data in, ${summary.outputs.length} out${params ? `, ${params} param(s)` : ''}`}
      className="bb-palette-item"
    >
      <div style={{ fontWeight: 600 }}>{summary.name}</div>
      <div className="bb-text-muted">{summary.meta_type}</div>
    </div>
  );
}

export function ComponentPalette() {
  const [components, setComponents] = useState<ComponentSummary[]>([]);
  const [filter, setFilter] = useState('');
  const setDescriptors = useGraphStore((s) => s.setDescriptors);

  useEffect(() => {
    getComponentDescriptors()
      .then((list) => {
        setComponents(list);
        setDescriptors(list);
      })
      .catch(console.error);
  }, [setDescriptors]);

  const filtered = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return components;
    return components.filter((c) => c.name.toLowerCase().includes(q) || c.meta_type.toLowerCase().includes(q));
  }, [components, filter]);

  return (
    <div className="bb-panel">
      <h3 className="bb-panel__title">Components</h3>
      <p className="bb-text-muted" style={{ margin: '2px 0 8px' }}>
        Drag or double-click to add
      </p>
      <input
        className="bb-input"
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        placeholder="Filter components…"
        style={{ marginBottom: 8 }}
      />
      {filtered.length === 0 && (
        <div className="bb-empty">{filter.trim() ? `No components match "${filter}"` : 'No components available'}</div>
      )}
      {filtered.map((c) => (
        <DraggableComponent key={c.name} summary={c} />
      ))}
    </div>
  );
}
