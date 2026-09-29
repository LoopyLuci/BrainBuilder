import { useGraphStore } from '../state/graphStore';
import PropertiesPanel from '../omniforge-components/PropertiesPanel';

export function RightPropertiesPanel() {
  const selectedNodeId = useGraphStore((s) => s.selectedNode);
  return <PropertiesPanel nodeId={selectedNodeId ?? ''} />;
}
