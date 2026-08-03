import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';

type Card = {
  id: string;
  title: string;
  description: string;
  status?: string;
  action?: string;
};

export function HomeDashboard() {
  const [cards, setCards] = useState<Card[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    Promise.all([
      invoke<{ running: boolean; platform: string }>('bot_dashboard_status').catch(() => ({ running: false, platform: 'telegram' })),
      invoke<any>('list_experiments').catch(() => []),
      invoke<any>('get_components').catch(() => []),
      invoke<any>('get_cluster_status').catch(() => ({ status: 'idle' })),
      invoke<any>('luci_status').catch(() => ({ status: 'idle' })),
    ])
      .then(([bot, experiments, components, cluster, luci]) => {
        if (cancelled) return;
        const items: Card[] = [
          {
            id: 'bot',
            title: 'Bot Server',
            description: 'Telegram, Discord, WhatsApp, Signal, Matrix, Email, SMS',
            status: bot.running ? `${bot.platform}: running` : 'stopped',
            action: 'Open Bot Dashboard',
          },
          {
            id: 'training',
            title: 'Training',
            description: `${Array.isArray(experiments) ? experiments.length : 0} experiments`,
            status: 'ready',
            action: 'Open Training',
          },
          {
            id: 'models',
            title: 'Models',
            description: `${Array.isArray(components) ? components.length : 0} components`,
            status: 'ready',
            action: 'Open Models',
          },
          {
            id: 'cluster',
            title: 'Cluster',
            description: 'Distributed compute and jobs',
            status: typeof cluster === 'object' && cluster ? String(cluster.status ?? 'idle') : 'idle',
            action: 'Open Cluster',
          },
          {
            id: 'luci',
            title: 'Luci',
            description: 'Central hub, plans, memory, tools',
            status: typeof luci === 'object' && luci ? String(luci.status ?? 'idle') : 'idle',
            action: 'Open Luci',
          },
          {
            id: 'console',
            title: 'Console',
            description: 'Logs, events, and system output',
            status: 'ready',
            action: 'Open Console',
          },
        ];
        setCards(items);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, []);

  const focusTab = (id: string) => {
    window.dispatchEvent(new CustomEvent('bb:focus-tab', { detail: { tabId: id } }));
  };

  return (
    <Panel title="Dashboard" subtitle="System overview and quick access to all BrainBuilder subsystems.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      {loading && <div className="bb-text-muted">Loading overview…</div>}
      {!loading && (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10 }}>
          {cards.map((c) => (
            <div key={c.id} className="bb-card" style={{ padding: 12, display: 'flex', flexDirection: 'column', gap: 6 }}>
              <div style={{ fontSize: 14, fontWeight: 700 }}>{c.title}</div>
              <div className="bb-text-muted" style={{ fontSize: 12 }}>{c.description}</div>
              {c.status && (
                <div>
                  <span className="bb-chip" style={{ fontSize: 10 }}>{c.status}</span>
                </div>
              )}
              <div style={{ marginTop: 'auto' }}>
                <Button variant="secondary" onClick={() => focusTab(c.id)} disabled={loading}>
                  {c.action}
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}
    </Panel>
  );
}
