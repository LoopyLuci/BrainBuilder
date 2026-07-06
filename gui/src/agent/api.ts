import { invoke } from '@tauri-apps/api/tauri';

export type AutonomyMode = 'propose-approve' | 'auto-apply' | 'full';

export interface AgentSession {
  id: string;
  mode: AutonomyMode;
  branch: string;
  worktree: string;
  repo_root: string;
}

export interface AgentRunReport {
  agent_output: string;
  diff: string;
  gate_passed: boolean;
  gate_output: string;
  merged: boolean;
}

// Start an isolated agent session (git worktree + branch) in the chosen mode.
export async function agentStart(mode: AutonomyMode): Promise<AgentSession> {
  return JSON.parse(await invoke<string>('agent_start', { mode }));
}

export async function agentStatus(): Promise<AgentSession | null> {
  const raw = await invoke<string | null>('agent_status');
  return raw ? JSON.parse(raw) : null;
}

// Run one agent step: OpenCode edits in the sandboxed worktree, tests run, and
// the mode's merge policy is applied. `selector` is the same provider:model
// string the rest of the app uses.
export async function agentRun(task: string, selector: string): Promise<AgentRunReport> {
  return JSON.parse(await invoke<string>('agent_run', { task, selector }));
}

export async function agentApprove(): Promise<void> {
  return invoke('agent_approve');
}

export async function agentRevert(): Promise<void> {
  return invoke('agent_revert');
}
