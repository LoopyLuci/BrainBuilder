import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface CaHealth {
  service: string;
  status: string;
  version: string;
}

export interface CaCompressionResult {
  content: string;
  metadata: {
    original_length: number;
    compressed_length: number;
    ratio: number;
    tokens_removed: number;
    strategy: string;
    model_version: string;
    checksum: string;
  };
}

export interface CaCompressionStats {
  input_length: number;
  output_length: number;
  input_tokens: number;
  output_tokens: number;
  compression_ratio: number;
  strategy: string;
  timestamp: string;
}

export function CompressionAgentPanel() {
  const [health, setHealth] = useState<CaHealth | null>(null);
  const [stats, setStats] = useState<CaCompressionStats[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Compress form
  const [content, setContent] = useState('');
  const [strategy, setStrategy] = useState('adaptive');
  const [result, setResult] = useState<CaCompressionResult | null>(null);

  const loadHealth = async () => {
    try {
      setError(null);
      const h = await invoke<CaHealth>('ca_health');
      setHealth(h);
    } catch (e) {
      setError(String(e));
    }
  };

  const loadStats = async () => {
    try {
      setError(null);
      const s = await invoke<CaCompressionStats[]>('ca_recent_stats', { limit: 20 });
      setStats(s);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    loadHealth();
    loadStats();
  }, []);

  const compress = async () => {
    if (!content.trim()) return;
    setBusy(true);
    setResult(null);
    try {
      const res = await invoke<CaCompressionResult>('ca_compress', {
        content: content.trim(),
        strategy,
      });
      setResult(res);
      logInfo(`Compressed via ${res.metadata.strategy}: ${res.metadata.original_length} → ${res.metadata.compressed_length} chars (${(res.metadata.ratio * 100).toFixed(1)}%)`);
      await loadStats();
    } catch (e) {
      logError(`compress failed: ${e}`);
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const refresh = async () => {
    await Promise.all([loadHealth(), loadStats()]);
  };

  const strategies = [
    { value: 'atomic', label: 'Atomic (whitespace normalization)' },
    { value: 'semantic', label: 'Semantic (filler removal)' },
    { value: 'adaptive', label: 'Adaptive (length-aware)' },
  ];

  return (
    <Panel
      title="Compression Agent"
      subtitle="Lossless and semantic text compression strategies."
      action={
        <Button variant="ghost" onClick={refresh} disabled={busy}>
          Refresh
        </Button>
      }
    >
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}

      {/* Health */}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Service Health</div>
        {health ? (
          <div className="bb-card" style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            <div>
              <span className="bb-text-muted">Service: </span>
              <span style={{ fontWeight: 600 }}>{health.service}</span>
            </div>
            <div>
              <span className="bb-text-muted">Status: </span>
              <span className={`bb-chip ${health.status === 'ok' ? 'bb-chip--success' : 'bb-chip--warn'}`}>{health.status}</span>
            </div>
            <div>
              <span className="bb-text-muted">Version: </span>
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: 12 }}>{health.version}</span>
            </div>
          </div>
        ) : (
          <div className="bb-text-muted">Loading…</div>
        )}
      </div>

      {/* Compress form */}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Compress Text</div>
        <textarea
          className="bb-input"
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder="Paste text to compress…"
          style={{ width: '100%', minHeight: 90, fontFamily: 'var(--font-mono)', fontSize: 12, marginBottom: 6 }}
        />
        <div className="bb-row">
          <select
            className="bb-input"
            value={strategy}
            onChange={(e) => setStrategy(e.target.value)}
            style={{ flex: 1 }}
          >
            {strategies.map((s) => (
              <option key={s.value} value={s.value}>{s.label}</option>
            ))}
          </select>
          <Button variant="primary" onClick={compress} disabled={busy || !content.trim()}>
            Compress
          </Button>
        </div>

        {/* Result */}
        {result && (
          <div className="bb-card" style={{ marginTop: 8 }}>
            <div style={{ fontSize: 12, marginBottom: 4 }}>
              <span className="bb-text-muted">Strategy: </span>
              <span style={{ fontWeight: 600 }}>{result.metadata.strategy}</span>
              <span className="bb-text-muted" style={{ marginLeft: 10 }}>
                {result.metadata.original_length} → {result.metadata.compressed_length} chars
              </span>
              <span className="bb-chip bb-chip--accent" style={{ marginLeft: 8 }}>
                {(result.metadata.ratio * 100).toFixed(1)}%
              </span>
            </div>
            <div style={{ fontSize: 11, color: 'rgb(148,163,184)' }}>
              tokens_removed={result.metadata.tokens_removed} · checksum={result.metadata.checksum.slice(0, 12)}…
            </div>
            <pre style={{ marginTop: 6, padding: 8, background: 'rgba(0,0,0,0.25)', borderRadius: 6, fontSize: 11, overflow: 'auto', maxHeight: 120 }}>
              {result.content}
            </pre>
          </div>
        )}
      </div>

      {/* Recent stats */}
      <div>
        <div className="bb-label" style={{ marginBottom: 4 }}>Recent Compressions</div>
        {stats.length === 0 ? (
          <div className="bb-text-muted">No compressions yet.</div>
        ) : (
          <div style={{ maxHeight: 160, overflow: 'auto' }}>
            <table style={{ width: '100%', fontSize: 11, borderCollapse: 'collapse' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.08)' }}>
                  <th style={{ textAlign: 'left', padding: '4px 4px' }}>Time</th>
                  <th style={{ textAlign: 'left', padding: '4px 4px' }}>Strategy</th>
                  <th style={{ textAlign: 'right', padding: '4px 4px' }}>In</th>
                  <th style={{ textAlign: 'right', padding: '4px 4px' }}>Out</th>
                  <th style={{ textAlign: 'right', padding: '4px 4px' }}>Ratio</th>
                </tr>
              </thead>
              <tbody>
                {stats.map((s, i) => (
                  <tr key={i} style={{ borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
                    <td style={{ padding: '3px 4px', fontFamily: 'var(--font-mono)' }}>{new Date(s.timestamp).toLocaleTimeString()}</td>
                    <td style={{ padding: '3px 4px' }}>
                      <span className="bb-chip" style={{ fontSize: 10 }}>{s.strategy}</span>
                    </td>
                    <td style={{ padding: '3px 4px', textAlign: 'right' }}>{s.input_length}</td>
                    <td style={{ padding: '3px 4px', textAlign: 'right' }}>{s.output_length}</td>
                    <td style={{ padding: '3px 4px', textAlign: 'right' }}>{(s.compression_ratio * 100).toFixed(1)}%</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </Panel>
  );
}
