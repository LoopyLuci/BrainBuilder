import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface AdaptiveReasoningResult {
  hypothesis: string;
  verdict: string;
  confidence: number;
}

export function AdaptiveReasoningPanel() {
  const [hypothesis, setHypothesis] = useState('');
  const [evidence, setEvidence] = useState('');
  const [result, setResult] = useState<AdaptiveReasoningResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const reason = async () => {
    if (!hypothesis.trim() || !evidence.trim()) return;
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const r = await invoke<AdaptiveReasoningResult>('adaptive_reason', {
        hypothesis: hypothesis.trim(),
        evidence: evidence.trim(),
        confidence: 0.5,
      });
      setResult(r);
      logInfo(`adaptive_reason: ${r.verdict} @ ${r.confidence.toFixed(2)}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`adaptive_reason failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Adaptive Reasoning" subtitle="Hypothesis-and-evidence adaptive reasoning.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Hypothesis</div>
        <input
          className="bb-input"
          value={hypothesis}
          onChange={(e) => setHypothesis(e.target.value)}
          placeholder="Enter hypothesis…"
          style={{ width: '100%', marginBottom: 6 }}
        />
        <div className="bb-label" style={{ marginBottom: 4 }}>Evidence</div>
        <textarea
          className="bb-input"
          value={evidence}
          onChange={(e) => setEvidence(e.target.value)}
          placeholder="Enter evidence…"
          style={{ width: '100%', minHeight: 70, marginBottom: 6 }}
        />
        <Button variant="primary" onClick={reason} disabled={busy || !hypothesis.trim() || !evidence.trim()}>
          Reason
        </Button>
      </div>

      {result && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            <div>
              <span className="bb-text-muted">Verdict: </span>
              <span style={{ fontWeight: 600 }}>{result.verdict}</span>
            </div>
            <div>
              <span className="bb-text-muted">Confidence: </span>
              <span style={{ fontWeight: 600 }}>{result.confidence.toFixed(2)}</span>
            </div>
          </div>
        </div>
      )}
    </Panel>
  );
}
