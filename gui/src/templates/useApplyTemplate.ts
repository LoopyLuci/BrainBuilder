import { useGraphStore } from '../state/graphStore';
import { instantiateTemplate, missingComponents, Template } from './registry';
import { logError, logInfo } from '../console/logStore';

// Shared "load this preset onto the canvas" action, used by both the Templates
// panel and the empty-canvas prompt so the behavior stays identical.
export function useApplyTemplate() {
  const descriptors = useGraphStore((s) => s.descriptors);
  const setGraph = useGraphStore((s) => s.setGraph);
  const training = useGraphStore((s) => s.training);
  const setTraining = useGraphStore((s) => s.setTraining);

  return (t: Template): boolean => {
    const missing = missingComponents(t, descriptors);
    if (missing.length > 0) {
      logError(`Template "${t.name}" needs component(s) not installed: ${missing.join(', ')}.`);
      return false;
    }
    const { nodes, edges } = instantiateTemplate(t, descriptors);
    setGraph(nodes, edges);
    if (t.training) setTraining({ ...training, ...t.training });
    logInfo(`Loaded template "${t.name}" — ${nodes.length} node(s) on the canvas.`);
    return true;
  };
}
