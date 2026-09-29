import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface ExplainResultView {
  feature: string;
  importance: number;
}

export function InteractiveExplainabilityPanel() {
  const [requestJson, setRequestJson] = useState('{"instance": {"x": 1, "y": 2}}');
  const [result, setResult] = useState<ExplainResultView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const explain = async () => {
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const r = await invoke<ExplainResultView>('interactive_explain', { request_json: requestJson });
      setResult(r);
      logInfo(`interactive_explain: ${r.feature}=${r.importance.toFixed(2)}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`interactive_explain failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Interactive Explainability" subtitle="Instance-level explanation requests.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Request JSON</div>
        <textarea
          className="bb-input"
          value={requestJson}
          onChange={(e) => setRequestJson(e.target.value)}
          style={{ width: '100%', minHeight: 60, fontFamily: 'var(--font-mono)', fontSize: 12, marginBottom: 6 }}
        />
        <Button variant="primary" onClick={explain} disabled={busy}>
          Explain
        </Button>
      </div>

      {result && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            <div>
              <span className="bb-text-muted">Feature: </span>
              <span style={{ fontWeight: 600 }}>{result.feature}</span>
            </div>
            <div>
              <span className="bb-text-muted">Importance: </span>
              <span style={{ fontWeight: 600 }}>{result.importance.toFixed(2)}</span>
            </div>
          </div>
        </div>
      )}
    </Panel>
  );
}
