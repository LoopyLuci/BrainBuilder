import { useEffect, useState } from 'react';
import {
  ClusterStatus,
  createCluster,
  generatePairingCode,
  getClusterStatus,
  getDistributedTrainingStatus,
  getObserverUrl,
  hostDistributedJob,
  JobInfo,
  joinClusterWithCode,
  joinDistributedJob,
  listDistributedJobs,
  TrainingStatus,
} from '../api/cluster';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { useGraphStore } from '../state/graphStore';
import { convertToBBIR } from '../canvas/utils';
import { logError, logInfo } from '../console/logStore';

function formatBytes(bytes: number | null): string {
  if (bytes === null) return 'unknown';
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

// Data-parallel training across the Cluster: host the current canvas graph,
// or join a job another paired device is hosting. Real gradient exchange —
// see gui/src-tauri/src/cluster_actor.rs — not a simulated progress bar.
// Live loss surfaces in the existing Metrics tab (same `metrics-update`
// event the local trainer publishes to), so this panel only needs to show
// step/total_steps and let you start/join.
function DistributedTraining() {
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const graphId = useGraphStore((s) => s.graphId);
  const training = useGraphStore((s) => s.training);
  const [expectedClients, setExpectedClients] = useState(1);
  const [jobs, setJobs] = useState<JobInfo[]>([]);
  const [status, setStatus] = useState<TrainingStatus | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    const poll = () => {
      listDistributedJobs().then(setJobs).catch(() => {});
      getDistributedTrainingStatus().then(setStatus).catch(() => {});
    };
    poll();
    const id = setInterval(poll, 2000);
    return () => clearInterval(id);
  }, []);

  const handleHost = async () => {
    if (nodes.length === 0) {
      logError('Add components to the canvas before hosting a training job.');
      return;
    }
    setBusy(true);
    try {
      const bbir = convertToBBIR(nodes, edges, graphId, 'untitled', training);
      const jobId = await hostDistributedJob(JSON.stringify(bbir), expectedClients);
      logInfo(`Hosting distributed training job ${jobId} — waiting for ${expectedClients} device(s) to join.`);
    } catch (e) {
      logError(`Failed to host distributed job: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const handleJoin = async (jobId: string) => {
    setBusy(true);
    try {
      await joinDistributedJob(jobId);
      logInfo(`Joined distributed training job ${jobId}.`);
    } catch (e) {
      logError(`Failed to join job ${jobId}: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div style={{ paddingTop: 8, borderTop: '1px solid var(--border-subtle)' }}>
      <div className="bb-label" style={{ marginBottom: 4 }}>
        Distributed Training
      </div>
      {status ? (
        <p className="bb-text-muted" style={{ margin: '0 0 8px' }}>
          {status.role === 'host' ? 'Hosting' : 'Joined'} job <code className="bb-code">{status.job_id.slice(0, 8)}…</code>{' '}
          — step {status.step}/{status.total_steps}
        </p>
      ) : (
        <p className="bb-text-muted" style={{ margin: '0 0 8px' }}>
          Trains the current canvas graph across every device that joins, averaging real gradients each round (see
          the Metrics tab for live loss).
        </p>
      )}
      <div className="bb-row">
        <input
          className="bb-input"
          type="number"
          min={1}
          value={expectedClients}
          onChange={(e) => setExpectedClients(Math.max(1, parseInt(e.target.value, 10) || 1))}
          style={{ width: 70 }}
          title="Number of other devices to wait for before training starts"
        />
        <Button variant="primary" onClick={handleHost} disabled={busy || !!status}>
          Host Training on Cluster
        </Button>
      </div>
      {jobs.length > 0 && (
        <div style={{ marginTop: 8 }}>
          <div className="bb-label" style={{ marginBottom: 4 }}>
            Joinable Jobs
          </div>
          <ul className="bb-list">
            {jobs.map((j) => (
              <li key={j.job_id} className="bb-list-item">
                {j.job_name} (hosted by {j.host_display_name})
                <Button variant="ghost" onClick={() => handleJoin(j.job_id)} disabled={busy || !!status}>
                  Join
                </Button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

export function ClusterConsole() {
  const [status, setStatus] = useState<ClusterStatus | null>(null);
  const [displayName, setDisplayName] = useState('My Device');
  const [pairingCode, setPairingCode] = useState<string | null>(null);
  const [joinCode, setJoinCode] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [observerUrl, setObserverUrl] = useState<string | null>(null);

  const refresh = async () => {
    try {
      setStatus(await getClusterStatus());
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 3000);
    getObserverUrl().then(setObserverUrl).catch(() => {});
    return () => clearInterval(interval);
  }, []);

  const handleCreate = async () => {
    setBusy(true);
    setError(null);
    try {
      setStatus(await createCluster(displayName));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleGenerateCode = async () => {
    setBusy(true);
    setError(null);
    try {
      setPairingCode(await generatePairingCode());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleJoin = async () => {
    setBusy(true);
    setError(null);
    try {
      setStatus(await joinClusterWithCode(joinCode.trim(), displayName));
      setJoinCode('');
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Cluster Console">
      {error && <div className="bb-text-error">{error}</div>}

      {!status ? (
        <div className="bb-empty">Connecting to cluster subsystem…</div>
      ) : !status.has_cluster ? (
        <div className="bb-panel__body">
          <p className="bb-text-muted" style={{ margin: 0 }}>
            This device isn't part of a Cluster yet. Create one to make it the Manager, or join an existing Cluster
            with a pairing code shown on another device's Console.
          </p>
          <input
            className="bb-input"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder="This device's name"
          />
          <Button variant="primary" onClick={handleCreate} disabled={busy}>
            Create My Cluster
          </Button>
          <div className="bb-row">
            <input
              className="bb-input"
              value={joinCode}
              onChange={(e) => setJoinCode(e.target.value)}
              placeholder="6-digit pairing code"
              maxLength={6}
            />
            <Button variant="secondary" onClick={handleJoin} disabled={busy || joinCode.trim().length === 0}>
              Join
            </Button>
          </div>
        </div>
      ) : (
        <div className="bb-panel__body">
          <div>
            Cluster <code className="bb-code">{status.cluster_id?.slice(0, 12)}…</code>
            {status.is_manager && <span className="bb-chip bb-chip--accent" style={{ marginLeft: 6 }}>Manager</span>}
          </div>

          <div>
            <div className="bb-label" style={{ marginBottom: 4 }}>
              Devices ({status.nodes.length})
            </div>
            <ul className="bb-list">
              {status.nodes.map((n) => (
                <li key={n.peer_id} className="bb-list-item">
                  {n.display_name}
                  {n.is_self && ' (this device)'} — {n.os}, {n.cpu_cores} cores, {formatBytes(n.ram_total_bytes)} RAM
                </li>
              ))}
            </ul>
          </div>

          {status.is_manager && (
            <div>
              <Button variant="secondary" onClick={handleGenerateCode} disabled={busy}>
                Generate Pairing Code
              </Button>
              {pairingCode && (
                <div style={{ marginTop: 6 }}>
                  Code (valid 5 min): <strong style={{ fontSize: 18, letterSpacing: 2 }}>{pairingCode}</strong>
                  <div className="bb-text-muted">Enter this on the device you want to add.</div>
                </div>
              )}
            </div>
          )}

          {observerUrl && (
            <div style={{ paddingTop: 8, borderTop: '1px solid var(--border-subtle)' }}>
              <div className="bb-label" style={{ marginBottom: 4 }}>
                Phone / Tablet Access
              </div>
              <p className="bb-text-muted" style={{ margin: 0 }}>
                On the same Wi-Fi, open this address in any browser to see the Cluster's live status (view-only — no
                app to install):
              </p>
              <code className="bb-code" style={{ display: 'inline-block', marginTop: 4 }}>
                {observerUrl}
              </code>
            </div>
          )}

          <DistributedTraining />
        </div>
      )}
    </Panel>
  );
}
