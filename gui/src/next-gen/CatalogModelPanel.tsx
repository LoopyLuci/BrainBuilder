import { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { logError, logInfo } from '../console/logStore';

interface Props { modelId: string; title: string; description?: string; }

export default function CatalogModelPanel({ modelId, title, description }: Props) {
  const [output, setOutput] = useState('');
  const [loading, setLoading] = useState(false);
  const run = async () => {
    setLoading(true); setOutput('');
    try {
      const res = await invoke<Record<string, unknown>>('catalog_model_run', { modelId, params: {} });
      setOutput(JSON.stringify(res, null, 2));
      logInfo(`${title}: ${JSON.stringify(res)}`);
    } catch (e) { logError(`${title}: ${String(e)}`); setOutput(String(e)); }
    finally { setLoading(false); }
  };
  return (<div className="panel"><h3>{title}</h3>{description && <p className="text-xs text-gray-500 mb-2">{description}</p>}<button onClick={run} disabled={loading}>Run Model</button><pre className="mt-2">{output}</pre></div>);
}
