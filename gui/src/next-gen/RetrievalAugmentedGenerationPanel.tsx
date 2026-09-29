import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface RagQueryResult {
  query: string;
  results: Array<{ id: string; score: number }>;
}

export function RetrievalAugmentedGenerationPanel() {
  const [query, setQuery] = useState('');
  const [result, setResult] = useState<RagQueryResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const search = async () => {
    if (!query.trim()) return;
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const r = await invoke<RagQueryResult>('rag_query', {
        query: query.trim(),
        top_k: 5,
      });
      setResult(r);
      logInfo(`RAG returned ${r.results.length} results`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`rag_query failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel
      title="Retrieval Augmented Generation"
      subtitle="Semantic retrieval over embedded document store."
    >
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Query</div>
        <div className="bb-row">
          <input
            className="bb-input"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Enter search query…"
            style={{ flex: 1 }}
          />
          <Button variant="primary" onClick={search} disabled={busy || !query.trim()}>
            Search
          </Button>
        </div>
      </div>

      {result && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div className="bb-label" style={{ marginBottom: 4 }}>Results</div>
          {result.results.length === 0 ? (
            <div className="bb-text-muted">No results.</div>
          ) : (
            <div style={{ maxHeight: 160, overflow: 'auto' }}>
              <table style={{ width: '100%', fontSize: 11, borderCollapse: 'collapse' }}>
                <thead>
                  <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.08)' }}>
                    <th style={{ textAlign: 'left', padding: '4px 4px' }}>ID</th>
                    <th style={{ textAlign: 'right', padding: '4px 4px' }}>Score</th>
                  </tr>
                </thead>
                <tbody>
                  {result.results.map((r, i) => (
                    <tr key={i} style={{ borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
                      <td style={{ padding: '3px 4px', fontFamily: 'var(--font-mono)' }}>{r.id}</td>
                      <td style={{ padding: '3px 4px', textAlign: 'right' }}>{r.score.toFixed(4)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </Panel>
  );
}
