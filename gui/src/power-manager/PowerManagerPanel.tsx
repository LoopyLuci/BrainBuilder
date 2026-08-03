import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface PmHealth {
  service: string;
  status: string;
  domains: number;
}

export interface PmDomain {
  id: string;
  name: string;
  parent_id?: string;
  children_ids?: string[];
  descriptor: Record<string, unknown>;
}

export interface PmIntentResult {
  domain_id: string;
  accepted: boolean;
  applied_w?: number;
}

export interface PmTelemetrySample {
  domain_id: string;
  timestamp: string;
  power_w: number;
  temp_c: number;
  cpu_util?: number;
  gpu_util?: number;
}

export function PowerManagerPanel() {
  const [health, setHealth] = useState<PmHealth | null>(null);
  const [domains, setDomains] = useState<PmDomain[]>([]);
  const [telemetry, setTelemetry] = useState<PmTelemetrySample[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Power limit form
  const [domainId, setDomainId] = useState('');
  const [watts, setWatts] = useState('25');

  const loadHealth = async () => {
    try {
      setError(null);
      const h = await invoke<PmHealth>('pm_health');
      setHealth(h);
    } catch (e) {
      setError(String(e));
    }
  };

  const loadDomains = async () => {
    try {
      setError(null);
      const d = await invoke<PmDomain[]>('pm_list_domains');
      setDomains(d);
    } catch (e) {
      setError(String(e));
    }
  };

  const loadTelemetry = async () => {
    try {
      setError(null);
      const t = await invoke<PmTelemetrySample[]>('pm_recent_telemetry', { limit: 20 });
      setTelemetry(t);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    loadHealth();
    loadDomains();
    loadTelemetry();
  }, []);

  const applyPowerLimit = async () => {
    if (!domainId.trim() || !watts.trim()) return;
    setBusy(true);
    try {
      const result = await invoke<PmIntentResult>('pm_apply_power_limit', {
        domain_id: domainId.trim(),
        watts: parseFloat(watts),
      });
      logInfo(`Power limit applied to ${result.domain_id}: accepted=${result.accepted}`);
      await loadTelemetry();
    } catch (e) {
      logError(`apply_power_limit failed: ${e}`);
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const refresh = async () => {
    await Promise.all([loadHealth(), loadDomains(), loadTelemetry()]);
  };

  return (
    <Panel
      title="Power Manager"
      subtitle="Per-domain power limits, telemetry, and MPC blueprints from Joulara."
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
              <span className="bb-text-muted">Domains: </span>
              <span>{health.domains}</span>
            </div>
          </div>
        ) : (
          <div className="bb-text-muted">Loading…</div>
        )}
      </div>

      {/* Apply power limit */}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Apply Power Limit</div>
        <div className="bb-row">
          <input
            className="bb-input"
            value={domainId}
            onChange={(e) => setDomainId(e.target.value)}
            placeholder="domain id"
            style={{ flex: 1 }}
          />
          <input
            className="bb-input"
            value={watts}
            onChange={(e) => setWatts(e.target.value)}
            placeholder="watts"
            style={{ width: 90 }}
            type="number"
          />
          <Button variant="primary" onClick={applyPowerLimit} disabled={busy || !domainId.trim() || !watts.trim()}>
            Apply
          </Button>
        </div>
      </div>

      {/* Domains */}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Domains ({domains.length})</div>
        {domains.length === 0 ? (
          <div className="bb-text-muted">No domains found.</div>
        ) : (
          <ul className="bb-list">
            {domains.map((d) => (
              <li key={d.id} className="bb-list-item">
                <div style={{ fontWeight: 600 }}>{d.name || d.id}</div>
                <div className="bb-text-muted" style={{ fontSize: 11 }}>
                  id: {d.id}
                  {d.parent_id ? ` · parent: ${d.parent_id}` : ''}
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>

      {/* Telemetry */}
      <div>
        <div className="bb-label" style={{ marginBottom: 4 }}>Recent Telemetry</div>
        {telemetry.length === 0 ? (
          <div className="bb-text-muted">No telemetry yet.</div>
        ) : (
          <div style={{ maxHeight: 160, overflow: 'auto' }}>
            <table style={{ width: '100%', fontSize: 11, borderCollapse: 'collapse' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.08)' }}>
                  <th style={{ textAlign: 'left', padding: '4px 4px' }}>Time</th>
                  <th style={{ textAlign: 'left', padding: '4px 4px' }}>Domain</th>
                  <th style={{ textAlign: 'right', padding: '4px 4px' }}>Power</th>
                  <th style={{ textAlign: 'right', padding: '4px 4px' }}>Temp</th>
                  <th style={{ textAlign: 'right', padding: '4px 4px' }}>CPU</th>
                  <th style={{ textAlign: 'right', padding: '4px 4px' }}>GPU</th>
                </tr>
              </thead>
              <tbody>
                {telemetry.map((s, i) => (
                  <tr key={i} style={{ borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
                    <td style={{ padding: '3px 4px', fontFamily: 'var(--font-mono)' }}>{new Date(s.timestamp).toLocaleTimeString()}</td>
                    <td style={{ padding: '3px 4px' }}>{s.domain_id}</td>
                    <td style={{ padding: '3px 4px', textAlign: 'right' }}>{s.power_w.toFixed(1)} W</td>
                    <td style={{ padding: '3px 4px', textAlign: 'right' }}>{s.temp_c.toFixed(1)} °C</td>
                    <td style={{ padding: '3px 4px', textAlign: 'right' }}>{s.cpu_util?.toFixed(0) ?? '—'}%</td>
                    <td style={{ padding: '3px 4px', textAlign: 'right' }}>{s.gpu_util?.toFixed(0) ?? '—'}%</td>
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
