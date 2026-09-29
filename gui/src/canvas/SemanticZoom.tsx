export function getRenderLevel(zoom: number): 'macro' | 'meso' | 'micro' {
  if (zoom < 0.4) return 'macro';
  if (zoom > 1.5) return 'micro';
  return 'meso';
}
