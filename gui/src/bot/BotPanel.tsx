import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { botListAdapters, botStartAdapter, botAdapterHealth, botSendMessage, botStartCall, botEndCall } from '../api/bot';
import { BotConfigDto, CallSessionDto } from '../api/bot';

export function BotPanel() {
  const [adapters, setAdapters] = useState<BotConfigDto[]>([]);
  const [selected, setSelected] = useState<string>('telegram');
  const [busy, setBusy] = useState(false);
  const [health, setHealth] = useState<string | null>(null);
  const [conversation, setConversation] = useState('default');
  const [user, setUser] = useState('me');
  const [text, setText] = useState('');
  const [activeCall, setActiveCall] = useState<CallSessionDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    try {
      setError(null);
      const list = await botListAdapters();
      setAdapters(list);
      if (list.length && !list.find(a => a.platform === selected)) setSelected(list[0].platform);
    } catch (e) {
      setError(String(e));
    }
  };

  const doStart = async () => {
    setBusy(true);
    try {
      const cfg = adapters.find(a => a.platform === selected) ?? ({ bot_id: selected, platform: selected, enabled: true, credentials: {} } as BotConfigDto);
      await botStartAdapter(cfg);
      setHealth('starting...');
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const checkHealth = async () => {
    try {
      setError(null);
      const h = await botAdapterHealth(selected);
      setHealth(h);
    } catch (e) {
      setError(String(e));
    }
  };

  const send = async () => {
    if (!text.trim()) return;
    setBusy(true);
    try {
      await botSendMessage(selected, conversation, text.trim());
      setText('');
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const startCall = async () => {
    setBusy(true);
    try {
      const call = await botStartCall(selected, conversation, user);
      setActiveCall(call);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const endCall = async () => {
    if (!activeCall) return;
    setBusy(true);
    try {
      await botEndCall(activeCall.id);
      setActiveCall(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => { load(); }, []);

  return (
    <Panel
      title="Bot Server"
      subtitle="Personal messaging hub with Luci routing"
      action={
        <div style={{ display: 'flex', gap: 8 }}>
          <Button variant="ghost" onClick={load} disabled={busy}>Refresh</Button>
          <Button variant="primary" onClick={doStart} disabled={busy || adapters.length === 0}>Start Adapter</Button>
        </div>
      }
    >
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
        <div>
          <div className="bb-label" style={{ marginBottom: 4 }}>Platform</div>
          <select className="bb-input" value={selected} onChange={(e) => setSelected(e.target.value)}>
            {adapters.map(a => (
              <option key={a.platform} value={a.platform}>{a.platform}</option>
            ))}
          </select>
        </div>
        <div>
          <div className="bb-label" style={{ marginBottom: 4 }}>Health</div>
          <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <div className={`bb-chip ${health === 'ok' ? 'bb-chip--success' : health ? 'bb-chip--warn' : ''}`}>{health ?? '—'}</div>
            <Button variant="secondary" onClick={checkHealth} disabled={busy}>Check</Button>
          </div>
        </div>
      </div>

      <div style={{ marginTop: 12, display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 8 }}>
        <div>
          <div className="bb-label" style={{ marginBottom: 4 }}>Conversation</div>
          <input className="bb-input" value={conversation} onChange={(e) => setConversation(e.target.value)} />
        </div>
        <div>
          <div className="bb-label" style={{ marginBottom: 4 }}>User</div>
          <input className="bb-input" value={user} onChange={(e) => setUser(e.target.value)} />
        </div>
        <div style={{ display: 'flex', alignItems: 'flex-end' }}>
          <Button variant="primary" onClick={startCall} disabled={busy || !!activeCall}>Start Call</Button>
        </div>
      </div>

      <div style={{ marginTop: 12, display: 'grid', gridTemplateColumns: '1fr auto', gap: 8 }}>
        <input
          className="bb-input"
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder="Message Luci or a bot..."
          onKeyDown={(e) => e.key === 'Enter' && send()}
        />
        <Button variant="primary" onClick={send} disabled={busy || !text.trim()}>Send</Button>
      </div>

      {activeCall && (
        <div style={{ marginTop: 12 }} className="bb-card">
          <div className="bb-label" style={{ marginBottom: 4 }}>Active Call</div>
          <div style={{ display: 'flex', gap: 12, alignItems: 'center', flexWrap: 'wrap' }}>
            <div><span className="bb-text-muted">id: </span><span style={{ fontFamily: 'var(--font-mono)' }}>{activeCall.id}</span></div>
            <div><span className="bb-text-muted">platform: </span>{activeCall.platform}</div>
            <div><span className="bb-text-muted">user: </span>{activeCall.user}</div>
            <Button variant="danger" onClick={endCall} disabled={busy}>End Call</Button>
          </div>
        </div>
      )}
    </Panel>
  );
}
