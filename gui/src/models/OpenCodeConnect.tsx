import { useEffect, useState } from 'react';
import { hasProviderCredentials, setProviderCredentials } from '../api/models';
import { useProviderStore } from '../state/providerStore';
import { logError, logInfo } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';

// Connect OpenCode Go as a hosted inference provider. The API key is written
// straight to the OS keychain via the backend — it never lives in localStorage
// or app state, and the secret is never read back to the frontend (we only ask
// the backend whether one is present).
export function OpenCodeConnect() {
  const [connected, setConnected] = useState<boolean | null>(null);
  const [key, setKey] = useState('');
  const [busy, setBusy] = useState(false);
  const refreshProviders = useProviderStore((s) => s.refresh);

  useEffect(() => {
    hasProviderCredentials('opencode').then(setConnected).catch(() => setConnected(false));
  }, []);

  const save = async () => {
    setBusy(true);
    try {
      await setProviderCredentials('opencode', key);
      setKey('');
      const present = await hasProviderCredentials('opencode');
      setConnected(present);
      await refreshProviders();
      logInfo(present ? 'OpenCode Go connected — its models are now in the provider selector.' : 'OpenCode Go key cleared.');
    } catch (e) {
      logError(`Saving OpenCode credentials failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const disconnect = async () => {
    setBusy(true);
    try {
      await setProviderCredentials('opencode', '');
      setConnected(false);
      await refreshProviders();
      logInfo('OpenCode Go disconnected.');
    } catch (e) {
      logError(`Clearing OpenCode credentials failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel
      title="OpenCode Go"
      subtitle="Hosted open models (GLM, Kimi, Qwen, DeepSeek…). Key stored in your OS keychain."
    >
      {connected ? (
        <div className="bb-row" style={{ alignItems: 'center' }}>
          <span className="bb-chip bb-chip--accent">Connected</span>
          <Button variant="ghost" onClick={disconnect} disabled={busy}>
            Disconnect
          </Button>
        </div>
      ) : (
        <div className="bb-row">
          <input
            className="bb-input"
            type="password"
            value={key}
            onChange={(e) => setKey(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && key.trim() && save()}
            placeholder="OpenCode Go API key"
          />
          <Button variant="primary" onClick={save} disabled={busy || !key.trim()}>
            Connect
          </Button>
        </div>
      )}
    </Panel>
  );
}
