import { useEffect, useRef, useState } from 'react';
import {
  botDashboardStatus,
  botDashboardTelemetry,
  botDashboardEvents,
  botDashboardSettingsGet,
  botDashboardSettingsSet,
  botDashboardStart,
  botDashboardStop,
  botDashboardRestart,
  botAutostart,
} from '../api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';

type Telemetry = {
  ping_ms: number | null;
  upload_mbps: number;
  download_mbps: number;
  jitter_ms: number | null;
  response_time_ms: number | null;
  cpu_percent: number;
  memory_mb: number;
};
type Status = {
  running: boolean;
  platform: string;
  uptime_secs: number;
  message_count: number;
  command_count: number;
  error_count: number;
  ping_ms: number | null;
};
type Settings = {
  platform: string;
  enabled: boolean;
  credentials: Record<string, string>;
  home_channel: string | null;
  allowed_users: string[];
  proxy: string | null;
};
type Event = { id: number; kind: string; message: string; at: number };

const HISTORY = 40;

function Sparkline({ points, color = 'var(--accent)' }: { points: number[]; color?: string }) {
  if (points.length < 2) return <div className="bb-text-muted" style={{ fontSize: 11 }}>Waiting for data…</div>;
  const w = 260;
  const h = 64;
  const pad = 4;
  const min = Math.min(...points);
  const max = Math.max(...points);
  const range = max - min || 1;
  const xs = points.map((_, i) => pad + (i / (points.length - 1)) * (w - pad * 2));
  const ys = points.map((v) => h - pad - ((v - min) / range) * (h - pad * 2));
  const path = points.map((_, i) => `${i === 0 ? 'M' : 'L'}${xs[i].toFixed(1)},${ys[i].toFixed(1)}`).join(' ');
  return (
    <svg viewBox={`0 0 ${w} ${h}`} style={{ width: '100%', height: h, background: 'var(--bg-subtle)', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
      <path d={path} fill="none" stroke={color} strokeWidth={1.6} strokeLinejoin="round" strokeLinecap="round" />
    </svg>
  );
}

export function BotDashboardPanel() {
  const [status, setStatus] = useState<Status | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [telemetry, setTelemetry] = useState<Telemetry | null>(null);
  const [events, setEvents] = useState<Event[]>([]);
  const [history, setHistory] = useState<{ ping: number[]; jitter: number[]; rt: number[] }>({ ping: [], jitter: [], rt: [] });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [edit, setEdit] = useState<Settings | null>(null);
  const intervalRef = useRef<number | null>(null);

  const refresh = async () => {
    try {
      setError(null);
      const [s, t, e] = await Promise.all([
        botDashboardStatus(),
        botDashboardTelemetry(),
        botDashboardEvents(),
      ]);
      setStatus(s);
      setTelemetry(t);
      setEvents(e);
      setHistory((h) => {
        const next = { ...h };
        next.ping = [...next.ping, t.ping_ms ?? 0].slice(-HISTORY);
        next.jitter = [...next.jitter, t.jitter_ms ?? 0].slice(-HISTORY);
        next.rt = [...next.rt, t.response_time_ms ?? 0].slice(-HISTORY);
        return next;
      });
      if (!edit && s.running) {
        const settingsRaw = await botDashboardSettingsGet();
        setSettings(settingsRaw);
      }
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    refresh();
    intervalRef.current = window.setInterval(refresh, 1000);
    return () => { if (intervalRef.current) clearInterval(intervalRef.current); };
  }, []);

  const start = async () => {
    setBusy(true);
    try {
      const token = edit?.credentials?.token ?? settings?.credentials?.token ?? '';
      const home = edit?.home_channel ?? settings?.home_channel ?? null;
      const users = edit?.allowed_users ?? settings?.allowed_users ?? [];
      const proxy = edit?.proxy ?? settings?.proxy ?? null;
      await botDashboardStart({ platform: 'telegram', credentials: { token }, home_channel: home, allowed_users: users, proxy });
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const stop = async () => {
    setBusy(true);
    try {
      await botDashboardStop();
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const restart = async () => {
    setBusy(true);
    try {
      const token = edit?.credentials?.token ?? settings?.credentials?.token ?? '';
      const home = edit?.home_channel ?? settings?.home_channel ?? null;
      const users = edit?.allowed_users ?? settings?.allowed_users ?? [];
      const proxy = edit?.proxy ?? settings?.proxy ?? null;
      await botDashboardRestart({ platform: 'telegram', credentials: { token }, home_channel: home, allowed_users: users, proxy });
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const saveSettings = async () => {
    if (!edit) return;
    setBusy(true);
    try {
      await botDashboardSettingsSet(edit);
      setSettings(edit);
      setEdit(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const autostart = async () => {
    setBusy(true);
    try {
      await botAutostart();
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const StatusCard = ({ label, value }: { label: string; value: React.ReactNode }) => (
    <div className="bb-card" style={{ padding: 10, minHeight: 64 }}>
      <div className="bb-text-muted" style={{ fontSize: 11, marginBottom: 4 }}>{label}</div>
      <div style={{ fontSize: 18, fontWeight: 700 }}>{value}</div>
    </div>
  );

  return (
    <Panel
      title="Bot Dashboard"
      subtitle="Telemetry, controls, and events for the BrainBuilder bot server."
      action={
        <div style={{ display: 'flex', gap: 6 }}>
          <Button variant="ghost" onClick={refresh} disabled={busy}>Refresh</Button>
          <Button variant="secondary" onClick={autostart} disabled={busy}>Auto-start</Button>
        </div>
      }
    >
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 10, marginBottom: 12 }}>
        <StatusCard label="Status" value={<span className={`bb-chip ${status?.running ? 'bb-chip--success' : 'bb-chip--warn'}`}>{status?.running ? 'running' : 'stopped'}</span>} />
        <StatusCard label="Uptime" value={`${status?.uptime_secs ?? 0}s`} />
        <StatusCard label="Messages" value={String(status?.message_count ?? 0)} />
        <StatusCard label="Commands" value={String(status?.command_count ?? 0)} />
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10, marginBottom: 12 }}>
        <div className="bb-card" style={{ padding: 10 }}>
          <div className="bb-text-muted" style={{ fontSize: 11, marginBottom: 4 }}>Ping</div>
          <Sparkline points={history.ping} />
          <div style={{ marginTop: 6, fontSize: 12 }}>{telemetry?.ping_ms ?? '—'} ms</div>
        </div>
        <div className="bb-card" style={{ padding: 10 }}>
          <div className="bb-text-muted" style={{ fontSize: 11, marginBottom: 4 }}>Jitter</div>
          <Sparkline points={history.jitter} color="var(--warning)" />
          <div style={{ marginTop: 6, fontSize: 12 }}>{telemetry?.jitter_ms?.toFixed(1) ?? '—'} ms</div>
        </div>
        <div className="bb-card" style={{ padding: 10 }}>
          <div className="bb-text-muted" style={{ fontSize: 11, marginBottom: 4 }}>Response Time</div>
          <Sparkline points={history.rt} color="var(--success)" />
          <div style={{ marginTop: 6, fontSize: 12 }}>{telemetry?.response_time_ms ?? '—'} ms</div>
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 10, marginBottom: 12 }}>
        <StatusCard label="CPU" value={`${telemetry?.cpu_percent.toFixed(1) ?? '0.0'}%`} />
        <StatusCard label="Memory" value={`${telemetry?.memory_mb ?? 0} MB`} />
        <StatusCard label="Upload" value={`${(telemetry?.upload_mbps ?? 0).toFixed(1)} Mbps`} />
        <StatusCard label="Download" value={`${(telemetry?.download_mbps ?? 0).toFixed(1)} Mbps`} />
      </div>

      <div style={{ display: 'flex', gap: 8, marginBottom: 12, flexWrap: 'wrap' }}>
        <Button variant="primary" onClick={start} disabled={busy || status?.running}>Start</Button>
        <Button variant="secondary" onClick={stop} disabled={busy || !status?.running}>Stop</Button>
        <Button variant="secondary" onClick={restart} disabled={busy}>Restart</Button>
        <Button variant="ghost" onClick={() => setEdit(settings ?? edit)} disabled={busy || !settings}>Edit Settings</Button>
      </div>

      {edit && (
        <div className="bb-card" style={{ padding: 12, marginBottom: 12 }}>
          <div className="bb-label" style={{ marginBottom: 6 }}>Bot Settings</div>
          <div style={{ display: 'grid', gap: 8 }}>
            <div>
              <div className="bb-label">Bot Token</div>
              <input className="bb-input" value={edit.credentials.token ?? ''} onChange={(e) => setEdit({ ...edit, credentials: { ...edit.credentials, token: e.target.value } })} />
            </div>
            <div>
              <div className="bb-label">Home Channel</div>
              <input className="bb-input" value={edit.home_channel ?? ''} onChange={(e) => setEdit({ ...edit, home_channel: e.target.value || null })} />
            </div>
            <div>
              <div className="bb-label">Allowed Users (comma-separated)</div>
              <input className="bb-input" value={(edit.allowed_users ?? []).join(', ')} onChange={(e) => setEdit({ ...edit, allowed_users: e.target.value.split(',').map((s) => s.trim()).filter(Boolean) })} />
            </div>
            <div>
              <div className="bb-label">Proxy</div>
              <input className="bb-input" value={edit.proxy ?? ''} onChange={(e) => setEdit({ ...edit, proxy: e.target.value || null })} />
            </div>
            <div style={{ display: 'flex', gap: 8 }}>
              <Button variant="primary" onClick={saveSettings} disabled={busy}>Save</Button>
              <Button variant="secondary" onClick={() => setEdit(null)} disabled={busy}>Cancel</Button>
            </div>
          </div>
        </div>
      )}

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
        <div className="bb-card" style={{ padding: 10 }}>
          <div className="bb-label" style={{ marginBottom: 6 }}>Recent Events</div>
          {events.length === 0 && <div className="bb-text-muted">No events yet.</div>}
          <div style={{ maxHeight: 180, overflow: 'auto' }}>
            {events.map((ev) => (
              <div key={ev.id} style={{ display: 'flex', gap: 8, fontSize: 12, borderBottom: '1px solid var(--border-subtle)', padding: '4px 0' }}>
                <span className="bb-text-muted" style={{ fontFamily: 'var(--font-mono)', width: 70 }}>{new Date(ev.at).toLocaleTimeString()}</span>
                <span className="bb-chip" style={{ fontSize: 10 }}>{ev.kind}</span>
                <span style={{ flex: 1 }}>{ev.message}</span>
              </div>
            ))}
          </div>
        </div>
        <div className="bb-card" style={{ padding: 10 }}>
          <div className="bb-label" style={{ marginBottom: 6 }}>Configuration</div>
          {settings ? (
            <div style={{ fontSize: 12 }}>
              <div><span className="bb-text-muted">platform:</span> {settings.platform}</div>
              <div><span className="bb-text-muted">enabled:</span> {String(settings.enabled)}</div>
              <div><span className="bb-text-muted">home_channel:</span> {settings.home_channel ?? '—'}</div>
              <div><span className="bb-text-muted">allowed_users:</span> {(settings.allowed_users ?? []).join(', ') || '—'}</div>
              <div><span className="bb-text-muted">proxy:</span> {settings.proxy ?? '—'}</div>
              <div><span className="bb-text-muted">token:</span> {settings.credentials.token ? '••••' + settings.credentials.token.slice(-4) : '—'}</div>
            </div>
          ) : (
            <div className="bb-text-muted">Not loaded.</div>
          )}
        </div>
      </div>
    </Panel>
  );
}
