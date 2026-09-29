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

  // `busy` only guards the credential write itself (fast — local keychain +
  // one IPC round trip). The provider-list refresh that follows touches
  // whatever provider happens to be selected (e.g. Ollama), which can be
  // slow or unreachable — that must never hold the Connect/Disconnect
  // button hostage, so it runs in the background, outside the try/finally
  // that clears `busy`.
  const save = async () => {
    setBusy(true);
    try {
      await setProviderCredentials('opencode', key);
      setKey('');
      const present = await hasProviderCredentials('opencode');
      setConnected(present);
    } catch (e) {
      logError(`Saving OpenCode credentials failed: ${e}`);
      setBusy(false);
      return;
    }
    setBusy(false);
    refreshProviders()
      .then(() => logInfo('OpenCode Go connected — its models are now in the provider selector.'))
      .catch((e) => logError(`Refreshing providers after connecting failed: ${e}`));
  };

  const disconnect = async () => {
    setBusy(true);
    try {
      await setProviderCredentials('opencode', '');
      setConnected(false);
    } catch (e) {
      logError(`Clearing OpenCode credentials failed: ${e}`);
      setBusy(false);
      return;
    }
    setBusy(false);
    refreshProviders()
      .then(() => logInfo('OpenCode Go disconnected.'))
      .catch((e) => logError(`Refreshing providers after disconnecting failed: ${e}`));
  };

  return (
    <Panel
      title="OpenCode Go"
      subtitle="Hosted open models (GLM, Kimi, Qwen, DeepSeek…). Key stored in your OS keychain."
    >
      <div data-tutorial="opencode-status">
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
              data-tutorial="opencode-key-input"
              value={key}
              onChange={(e) => setKey(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && key.trim() && save()}
              placeholder="OpenCode Go API key"
            />
            <Button variant="primary" data-tutorial="opencode-connect-btn" onClick={save} disabled={busy || !key.trim()}>
              Connect
            </Button>
          </div>
        )}
      </div>
    </Panel>
  );
}
