import { useEffect, useMemo, useState } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import {
  liveRuntimeRun,
  liveRuntimeStatus,
  liveRuntimeStop,
  liveRuntimeStreamEvents,
  liveRuntimeSmokeTest,
  LiveRuntimeRunOptions,
} from '../api/models';
import { logError, logInfo } from '../console/logStore';

type RunState = 'idle' | 'running' | 'error';

export default function LiveRuntimePanel() {
  const [graphJson, setGraphJson] = useState('');
  const [runState, setRunState] = useState<RunState>('idle');
  const [status, setStatus] = useState('');
  const [events, setEvents] = useState<Array<{ time: string; type: string; payload: unknown }>>([]);
  const [smoke, setSmoke] = useState<string | null>(null);
  const [opts, setOpts] = useState<LiveRuntimeRunOptions>({ stream: true, temperature: 0.7, topP: 0.95, maxTokens: 256 });

  const smokeTest = async () => {
    setSmoke(null);
    try {
      const result = await liveRuntimeSmokeTest();
      setSmoke(result);
      logInfo(`live runtime smoke test: ${result}`);
    } catch (e) {
      setSmoke(String(e));
      logError(`live runtime smoke test failed: ${String(e)}`);
    }
  };

  // Auto-run the backend smoke test once when the panel becomes visible,
  // so the unified runtime is exercised without requiring manual clicks.
  useEffect(() => {
    let cancelled = false;
    const timer = setTimeout(async () => {
      if (cancelled) return;
      await smokeTest();
    }, 800);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [smokeTest]);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    const subscribe = async () => {
      try {
        await liveRuntimeStreamEvents();
      } catch {
        /* best-effort; frontend may not be inside Tauri */
      }

      const handlers: Array<[string, string]> = [
        ['inference-start', 'start'],
        ['inference-node-start', 'node'],
        ['inference-token', 'token'],
        ['inference-output', 'output'],
        ['inference-error', 'error'],
        ['inference-complete', 'complete'],
      ];

      const unlisteners: UnlistenFn[] = [];
      for (const [event, label] of handlers) {
        const fn = await listen<unknown>(event, (ev) => {
          setEvents((prev) => [...prev.slice(-200), { time: new Date().toLocaleTimeString(), type: label, payload: ev.payload }]);
        });
        unlisteners.push(fn);
      }

      unlisten = () => {
        unlisteners.forEach((u) => u());
      };
    };

    subscribe();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const runGraph = async () => {
    if (!graphJson.trim()) return;
    setRunState('running');
    setSmoke(null);
    setEvents([]);
    try {
      const result = await liveRuntimeRun(graphJson.trim(), opts);
      setStatus(result);
      setRunState('idle');
      logInfo(`live runtime run complete: ${result}`);
    } catch (e) {
      const message = String(e);
      setStatus(message);
      setRunState('error');
      logError(`live runtime run failed: ${message}`);
    }
  };

  const stop = async () => {
    try {
      const result = await liveRuntimeStop();
      setStatus(result);
      setRunState('idle');
      logInfo(`live runtime stop: ${result}`);
    } catch (e) {
      logError(`live runtime stop failed: ${String(e)}`);
    }
  };

  const pollStatus = async () => {
    try {
      const result = await liveRuntimeStatus();
      setStatus(result);
    } catch (e) {
      logError(`live runtime status failed: ${String(e)}`);
    }
  };

  return (
    <div className="panel">
      <h3>Live Runtime</h3>
      <p className="text-xs text-gray-500 mb-2">Unified runtime console with smoke test, execution, and live inference events.</p>

      <div className="space-y-2">
        <div>
          <label className="text-xs font-semibold">Graph JSON</label>
          <textarea
            className="border rounded w-full font-mono text-xs"
            rows={6}
            value={graphJson}
            onChange={(e) => setGraphJson(e.target.value)}
            placeholder="Paste a BBIR graph JSON here..."
          />
        </div>

        <div className="flex flex-wrap gap-2">
          <button onClick={runGraph} disabled={runState === 'running' || !graphJson.trim()}>
            {runState === 'running' ? 'Running...' : 'Run'}
          </button>
          <button onClick={stop} disabled={runState !== 'running'}>
            Stop
          </button>
          <button onClick={pollStatus}>Status</button>
          <button onClick={smokeTest}>Smoke test</button>
        </div>

        <div className="flex flex-wrap gap-2 text-xs">
          <label className="flex items-center gap-1">
            <input type="checkbox" checked={opts.stream ?? true} onChange={(e) => setOpts({ ...opts, stream: e.target.checked })} />
            Stream
          </label>
          <label className="flex items-center gap-1">
            Temperature
            <input type="number" step="0.1" className="border rounded px-1 py-0.5 w-20" value={opts.temperature ?? 0.7} onChange={(e) => setOpts({ ...opts, temperature: Number(e.target.value) })} />
          </label>
          <label className="flex items-center gap-1">
            Top P
            <input type="number" step="0.05" className="border rounded px-1 py-0.5 w-20" value={opts.topP ?? 0.95} onChange={(e) => setOpts({ ...opts, topP: Number(e.target.value) })} />
          </label>
          <label className="flex items-center gap-1">
            Max tokens
            <input type="number" step="1" className="border rounded px-1 py-0.5 w-24" value={opts.maxTokens ?? 256} onChange={(e) => setOpts({ ...opts, maxTokens: Number(e.target.value) })} />
          </label>
        </div>

        <div className="text-xs">
          <div>Status: <span className="font-mono">{status || '(none)'}</span></div>
          <div>Run state: <span className="font-mono">{runState}</span></div>
          <div>Smoke test: <span className="font-mono">{smoke ?? '(not run)'}</span></div>
        </div>

        <div>
          <div className="text-xs font-semibold">Events</div>
          <div className="border rounded max-h-60 overflow-auto mt-1">
            <table className="w-full text-left text-xs">
              <thead>
                <tr className="bg-black/5">
                  <th className="px-2 py-1">Time</th>
                  <th className="px-2 py-1">Type</th>
                  <th className="px-2 py-1">Payload</th>
                </tr>
              </thead>
              <tbody>
                {events.slice().reverse().map((ev, idx) => (
                  <tr key={idx} className="border-t">
                    <td className="px-2 py-1 font-mono whitespace-nowrap">{ev.time}</td>
                    <td className="px-2 py-1 font-mono whitespace-nowrap">{ev.type}</td>
                    <td className="px-2 py-1 font-mono break-all">{typeof ev.payload === 'string' ? ev.payload : JSON.stringify(ev.payload)}</td>
                  </tr>
                ))}
                {events.length === 0 && (
                  <tr><td colSpan={3} className="px-2 py-2 text-gray-500">No events yet. Run the smoke test or a graph.</td></tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}
