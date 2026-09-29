import { describe, it, expect } from 'vitest';
import { hyperparamsToSchema } from './SchemaForm';

describe('hyperparamsToSchema', () => {
  it('maps descriptor param types to field types', () => {
    const schema = hyperparamsToSchema([
      { name: 'features', param_type: 'int', default: 128 },
      { name: 'rate', param_type: 'float', default: 0.1 },
      { name: 'bias', param_type: 'bool', default: true },
      { name: 'activation', param_type: 'string', default: 'relu' },
    ]);
    const byName = Object.fromEntries(schema.fields.map((f) => [f.name, f.type]));
    expect(byName.features).toBe('number');
    expect(byName.rate).toBe('number');
    expect(byName.bias).toBe('boolean');
    expect(byName.activation).toBe('text');
  });

  it('falls back to text for an unknown param type and preserves the default', () => {
    const schema = hyperparamsToSchema([{ name: 'weird', param_type: 'tensor', default: 42 }]);
    expect(schema.fields[0].type).toBe('text');
    expect(schema.fields[0].default).toBe(42);
  });
});
