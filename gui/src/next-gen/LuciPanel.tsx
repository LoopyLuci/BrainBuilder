import { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { logError, logInfo } from '../console/logStore';
import { toolExecutorRun, toolExecutorList, ToolExecutionResult } from '../api/tauri';

interface Props { modelId?: string; title?: string; description?: string; }

export default function LuciPanel({ modelId, title, description }: Props) {
  const [user, setUser] = useState('you');
  const [message, setMessage] = useState('');
  const [output, setOutput] = useState('');
  const [loading, setLoading] = useState(false);
  const [planTitle, setPlanTitle] = useState('');
  const [plans, setPlans] = useState<Array<{ id: string; title: string; steps: string[]; status: string; owner: string }>>([]);

  // Tool executor state
  const [tools, setTools] = useState<string[]>([]);
  const [selectedTool, setSelectedTool] = useState('');
  const [toolArgs, setToolArgs] = useState('{}');
  const [toolResult, setToolResult] = useState<ToolExecutionResult | null>(null);

  const loadTools = async () => {
    try {
      const list = await toolExecutorList();
      setTools(list);
      if (list.length && !selectedTool) setSelectedTool(list[0]);
    } catch (e) { logError(`load tools: ${String(e)}`); }
  };

  const runTool = async () => {
    if (!selectedTool) return;
    setLoading(true);
    setToolResult(null);
    try {
      let args: Record<string, any> = {};
      try { args = JSON.parse(toolArgs || '{}'); } catch { /* noop */ }
      const res = await toolExecutorRun(selectedTool, args);
      setToolResult(res);
      logInfo(`tool ${selectedTool}: ${JSON.stringify(res)}`);
    } catch (e) { logError(`tool ${selectedTool}: ${String(e)}`); }
    finally { setLoading(false); }
  };

  const runChat = async () => {
    if (!message.trim()) return;
    setLoading(true); setOutput('');
    try {
      const res = await invoke<Record<string, unknown>>('luci_chat', { user, message: message.trim() });
      setOutput(typeof res === 'string' ? res : JSON.stringify(res, null, 2));
      logInfo(`Luci: ${JSON.stringify(res)}`);
    } catch (e) { logError(`Luci: ${String(e)}`); setOutput(String(e)); }
    finally { setLoading(false); setMessage(''); }
  };

  const proposePlan = async () => {
    if (!planTitle.trim()) return;
    setLoading(true); setOutput('');
    try {
      const res = await invoke<Record<string, unknown>>('luci_propose_plan', { user, title: planTitle.trim() });
      setPlans(prev => [...prev, res as any]);
      setOutput(JSON.stringify(res, null, 2));
      setPlanTitle('');
    } catch (e) { logError(`Luci: ${String(e)}`); setOutput(String(e)); }
    finally { setLoading(false); }
  };

  const listPlans = async () => {
    setLoading(true); setOutput('');
    try {
      const res = await invoke<Array<{ id: string; title: string; steps: string[]; status: string; owner: string }>>('luci_list_plans', { user });
      setPlans(res);
      setOutput(JSON.stringify(res, null, 2));
    } catch (e) { logError(`Luci: ${String(e)}`); setOutput(String(e)); }
    finally { setLoading(false); }
  };

  return (
    <div className="panel">
      <h3>{title}</h3>
      {description && <p className="text-xs text-gray-500 mb-2">{description}</p>}
      <div className="space-y-2">
        <input value={user} onChange={e => setUser(e.target.value)} placeholder="Your name" className="border rounded px-2 py-1 w-full" />

        <div className="flex gap-2">
          <input value={message} onChange={e => setMessage(e.target.value)} onKeyDown={e => e.key === 'Enter' && runChat()} placeholder="Say something to Luci..." className="border rounded px-2 py-1 flex-1" />
          <button onClick={runChat} disabled={loading}>Send</button>
        </div>

        <div className="flex gap-2">
          <input value={planTitle} onChange={e => setPlanTitle(e.target.value)} placeholder="New plan title" className="border rounded px-2 py-1 flex-1" />
          <button onClick={proposePlan} disabled={loading}>Plan</button>
          <button onClick={listPlans} disabled={loading}>My Plans</button>
        </div>

        <div className="mt-2">
          <div className="font-semibold">Tool Executor</div>
          <div className="flex gap-2 mt-1">
            <select className="border rounded px-2 py-1" value={selectedTool} onChange={e => setSelectedTool(e.target.value)} onFocus={loadTools}>
              <option value="">-- load tools --</option>
              {tools.map(t => <option key={t} value={t}>{t}</option>)}
            </select>
            <input className="border rounded px-2 py-1 flex-1" value={toolArgs} onChange={e => setToolArgs(e.target.value)} placeholder='{"path":"..."}' />
            <button onClick={runTool} disabled={loading || !selectedTool}>Run</button>
          </div>
          {toolResult && (
            <pre className="mt-1 whitespace-pre-wrap text-xs">{JSON.stringify(toolResult, null, 2)}</pre>
          )}
        </div>

        <pre className="mt-2 whitespace-pre-wrap">{output}</pre>
        {plans.length > 0 && (
          <div className="mt-2">
            <h4 className="font-semibold">Plans</h4>
            <ul className="list-disc pl-5">
              {plans.map(p => (
                <li key={p.id}>{p.title} — <span className="text-xs text-gray-500">{p.status}</span></li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  );
}
