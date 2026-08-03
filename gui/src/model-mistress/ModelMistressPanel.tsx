import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';

type MmModel = {
  id: string;
  object: string;
  created: number;
  owned_by: string;
  size_bytes?: number;
  format?: string;
  quantization?: string;
  context_length?: number;
};

type MmChatMessage = {
  role: 'user' | 'assistant' | 'system';
  content?: string;
};

type MmHealth = {
  status: string;
  version: string;
  timestamp: string;
  ollama: string;
  local_models_loaded: number;
};

const DEFAULT_BASE_URL = 'http://localhost:8000';

export function ModelMistressPanel() {
  const [baseUrl, setBaseUrl] = useState(DEFAULT_BASE_URL);
  const [health, setHealth] = useState<MmHealth | null>(null);
  const [models, setModels] = useState<MmModel[]>([]);
  const [loading, setLoading] = useState(false);
  const [model, setModel] = useState('');
  const [messages, setMessages] = useState<MmChatMessage[]>([
    { role: 'system', content: 'You are a helpful assistant.' },
  ]);
  const [input, setInput] = useState('');
  const [streaming, setStreaming] = useState(false);

  useEffect(() => {
    const unlisten = listen<string>('model-mistress-token', (event) => {
      setStreaming((prev) => {
        if (!prev) return true;
        return prev;
      });
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  async function refreshHealth() {
    try {
      await invoke<MmHealth>('model_mistress_set_base_url', { url: baseUrl });
      const h = await invoke<MmHealth>('model_mistress_health');
      setHealth(h);
    } catch (e) {
      console.error(e);
    }
  }

  async function refreshModels() {
    try {
      const list = await invoke<MmModel[]>('model_mistress_list_models');
      setModels(list);
      if (!model && list.length > 0) setModel(list[0].id);
    } catch (e) {
      console.error(e);
    }
  }

  async function sendChat() {
    if (!input.trim() || !model) return;
    const newMessages: MmChatMessage[] = [...messages, { role: 'user', content: input }];
    setMessages(newMessages);
    setInput('');
    setLoading(true);
    try {
      const resp = await invoke<any>('model_mistress_chat', {
        model,
        messages: newMessages,
        stream: false,
        temperature: 0.7,
        top_p: 0.9,
        max_tokens: 512,
      });
      const assistant = resp?.choices?.[0]?.message;
      setMessages((prev) => [
        ...prev,
        { role: 'assistant', content: assistant?.content || String(JSON.stringify(assistant)) },
      ]);
    } catch (e) {
      setMessages((prev) => [...prev, { role: 'assistant', content: `Error: ${e}` }]);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div style={{ padding: 12, display: 'flex', flexDirection: 'column', gap: 10, height: '100%' }}>
      <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
        <input
          value={baseUrl}
          onChange={(e) => setBaseUrl(e.target.value)}
          style={{ flex: 1, padding: '6px 8px', borderRadius: 4, border: '1px solid #333', background: '#111', color: '#eee' }}
        />
        <button onClick={refreshHealth} style={{ padding: '6px 10px', cursor: 'pointer' }}>Connect</button>
      </div>

      <div style={{ display: 'flex', gap: 8 }}>
        <div style={{ flex: 1, background: '#111', padding: 10, borderRadius: 6, border: '1px solid #333' }}>
          <div style={{ fontSize: 12, opacity: 0.7, marginBottom: 6 }}>Status</div>
          <div style={{ fontSize: 13 }}>
            {health ? (
              <>
                <div>Status: {health.status}</div>
                <div>Version: {health.version}</div>
                <div>Ollama: {health.ollama}</div>
                <div>Local models: {health.local_models_loaded}</div>
              </>
            ) : (
              <div style={{ opacity: 0.6 }}>Not connected</div>
            )}
          </div>
        </div>
        <div style={{ width: 220, background: '#111', padding: 10, borderRadius: 6, border: '1px solid #333' }}>
          <div style={{ fontSize: 12, opacity: 0.7, marginBottom: 6 }}>Models</div>
          <select
            value={model}
            onChange={(e) => setModel(e.target.value)}
            style={{ width: '100%', background: '#1a1a1a', color: '#eee', padding: 6 }}
          >
            <option value="">Select model</option>
            {models.map((m) => (
              <option key={m.id} value={m.id}>{m.id}</option>
            ))}
          </select>
          <button onClick={refreshModels} style={{ marginTop: 8, padding: '6px 10px', cursor: 'pointer', width: '100%' }}>
            Refresh
          </button>
        </div>
      </div>

      <div style={{ flex: 1, background: '#111', padding: 10, borderRadius: 6, border: '1px solid #333', overflow: 'auto' }}>
        <div style={{ fontSize: 12, opacity: 0.7, marginBottom: 6 }}>Chat</div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {messages.map((m, idx) => (
            <div key={idx} style={{ textAlign: m.role === 'user' ? 'right' : 'left' }}>
              <div style={{ fontSize: 11, opacity: 0.7 }}>{m.role}</div>
              <div style={{ background: m.role === 'user' ? '#223' : '#1a1a1a', padding: 8, borderRadius: 6, whiteSpace: 'pre-wrap' }}>
                {m.content}
              </div>
            </div>
          ))}
          {loading && <div style={{ opacity: 0.7 }}>…</div>}
        </div>
      </div>

      <div style={{ display: 'flex', gap: 8 }}>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && sendChat()}
          style={{ flex: 1, padding: '8px 10px', borderRadius: 4, border: '1px solid #333', background: '#111', color: '#eee' }}
        />
        <button onClick={sendChat} disabled={loading || !model} style={{ padding: '8px 12px', cursor: 'pointer' }}>
          Send
        </button>
      </div>
    </div>
  );
}
