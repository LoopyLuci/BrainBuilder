// Schema-driven procedural UI: renders a form from a JSON field schema instead
// of hand-written JSX, so new controls (from a synthesized component's
// hyperparameters, an agent, or a plugin) appear from data alone. Descriptor
// hyperparameters already carry type/default/constraints — feed them straight
// in via `hyperparamsToSchema`.

export type FieldType = 'number' | 'text' | 'boolean' | 'select';

export interface FieldSchema {
  name: string;
  label?: string;
  type: FieldType;
  default?: unknown;
  /** For 'select'. */
  options?: string[];
  /** For 'number'. */
  min?: number;
  max?: number;
  step?: number;
}

export interface FormSchema {
  fields: FieldSchema[];
}

export type FormValues = Record<string, unknown>;

export function SchemaForm({
  schema,
  values,
  onChange,
}: {
  schema: FormSchema;
  values: FormValues;
  onChange: (next: FormValues) => void;
}) {
  const set = (name: string, value: unknown) => onChange({ ...values, [name]: value });

  return (
    <div className="bb-form-grid">
      {schema.fields.map((f) => {
        const label = f.label ?? f.name;
        const value = values[f.name] ?? f.default ?? '';
        return (
          <FieldRow key={f.name} field={f} label={label} value={value} onSet={(v) => set(f.name, v)} />
        );
      })}
    </div>
  );
}

function FieldRow({
  field,
  label,
  value,
  onSet,
}: {
  field: FieldSchema;
  label: string;
  value: unknown;
  onSet: (v: unknown) => void;
}) {
  const control = () => {
    switch (field.type) {
      case 'boolean':
        return (
          <input type="checkbox" checked={Boolean(value)} onChange={(e) => onSet(e.target.checked)} />
        );
      case 'select':
        return (
          <select className="bb-select" value={String(value)} onChange={(e) => onSet(e.target.value)}>
            {(field.options ?? []).map((o) => (
              <option key={o} value={o}>
                {o}
              </option>
            ))}
          </select>
        );
      case 'number':
        return (
          <input
            className="bb-input"
            type="number"
            value={Number(value)}
            min={field.min}
            max={field.max}
            step={field.step}
            onChange={(e) => onSet(Number(e.target.value))}
          />
        );
      default:
        return (
          <input className="bb-input" type="text" value={String(value)} onChange={(e) => onSet(e.target.value)} />
        );
    }
  };

  return (
    <>
      <label className="bb-label">{label}</label>
      {control()}
    </>
  );
}

// Bridges a component descriptor's hyperparameter definitions (type/default)
// into a FormSchema, so any component's controls can be rendered generically.
export function hyperparamsToSchema(
  hyperparameters: { name: string; param_type: string; default: unknown }[],
): FormSchema {
  const typeMap: Record<string, FieldType> = {
    int: 'number',
    integer: 'number',
    float: 'number',
    number: 'number',
    bool: 'boolean',
    boolean: 'boolean',
    string: 'text',
  };
  return {
    fields: hyperparameters.map((h) => ({
      name: h.name,
      type: typeMap[h.param_type] ?? 'text',
      default: h.default,
    })),
  };
}
