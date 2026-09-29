import { invoke } from '@tauri-apps/api/tauri';

export interface NodeInfo {
  peer_id: string;
  display_name: string;
  os: string;
  cpu_cores: number;
  ram_total_bytes: number | null;
  is_self: boolean;
}

export interface ClusterStatus {
  self_peer_id: string;
  has_cluster: boolean;
  cluster_id: string | null;
  is_manager: boolean;
  nodes: NodeInfo[];
}

export async function getClusterStatus(): Promise<ClusterStatus> {
  return invoke('get_cluster_status');
}

export async function createCluster(displayName: string): Promise<ClusterStatus> {
  return invoke('create_cluster', { displayName });
}

export async function generatePairingCode(): Promise<string> {
  return invoke('generate_pairing_code');
}

export async function joinClusterWithCode(code: string, displayName: string): Promise<ClusterStatus> {
  return invoke('join_cluster_with_code', { code, displayName });
}

export async function getObserverUrl(): Promise<string> {
  return invoke('get_observer_url');
}

export interface JobInfo {
  job_id: string;
  job_name: string;
  host_display_name: string;
}

export interface TrainingStatus {
  job_id: string;
  role: 'host' | 'client';
  step: number;
  total_steps: number;
}

/// Compiles `graphJson` and starts hosting it as a data-parallel training
/// job: `expectedClients` other paired devices must join before the first
/// gradient-averaging round runs. Live loss appears in the Metrics tab.
export async function hostDistributedJob(graphJson: string, expectedClients: number): Promise<string> {
  return invoke('host_distributed_job', { graphJson, expectedClients });
}

export async function listDistributedJobs(): Promise<JobInfo[]> {
  return invoke('list_distributed_jobs');
}

export async function joinDistributedJob(jobId: string): Promise<void> {
  return invoke('join_distributed_job', { jobId });
}

export async function getDistributedTrainingStatus(): Promise<TrainingStatus | null> {
  return invoke('get_distributed_training_status');
}
