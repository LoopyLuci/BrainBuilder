import { describe, it, expect, beforeEach } from 'vitest';
import { useLayoutStore } from './layoutStore';

describe('layoutStore', () => {
  beforeEach(() => {
    localStorage.clear();
    // Reset to a clean hidden set between tests.
    useLayoutStore.setState({ hidden: new Set() });
  });

  it('toggles a widget hidden and back', () => {
    const { toggle, isHidden } = useLayoutStore.getState();
    expect(isHidden('data')).toBe(false);
    toggle('data');
    expect(useLayoutStore.getState().isHidden('data')).toBe(true);
    toggle('data');
    expect(useLayoutStore.getState().isHidden('data')).toBe(false);
  });

  it('persists the hidden set to localStorage', () => {
    useLayoutStore.getState().toggle('models');
    expect(JSON.parse(localStorage.getItem('brainbuilder.layout.hidden')!)).toEqual(['models']);
  });

  it('show() is a no-op for a visible widget and removes a hidden one', () => {
    const { show, toggle } = useLayoutStore.getState();
    show('agent'); // no-op
    expect(useLayoutStore.getState().isHidden('agent')).toBe(false);
    toggle('agent');
    show('agent');
    expect(useLayoutStore.getState().isHidden('agent')).toBe(false);
  });
});
