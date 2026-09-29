import { spawn } from 'node:child_process';
import type { CapabilityManifest, CliOperation, ExecutionResult } from '../types.js';

function splitCommand(command: string): string[] {
  return command.trim().split(/\s+/).filter(Boolean);
}

function buildArgs(op: CliOperation, args: Record<string, unknown>): { argv: string[]; error?: string } {
  const argv = [...op.subcommandPath];
  for (const param of op.params) {
    const provided = args[param.name];
    if (provided === undefined) {
      if (param.required) return { argv, error: `missing required param "${param.name}"` };
      continue;
    }
    if (!param.flag) {
      // Positional.
      argv.push(String(provided));
      continue;
    }
    if (param.isSwitch) {
      if (provided) argv.push(param.flag);
      continue;
    }
    argv.push(param.flag, String(provided));
  }
  return { argv };
}

export async function executeCli(manifest: CapabilityManifest, operationName: string, args: Record<string, unknown>): Promise<ExecutionResult> {
  if (manifest.target.kind !== 'cli') return { ok: false, error: `manifest target is not a cli target (got "${manifest.target.kind}")` };
  const op = manifest.operations.find((o) => o.kind === 'cli' && o.name === operationName) as CliOperation | undefined;
  if (!op) return { ok: false, error: `no cli operation named "${operationName}" in manifest "${manifest.name}"` };

  const { argv, error } = buildArgs(op, args);
  if (error) return { ok: false, error };

  const commandParts = splitCommand(manifest.target.command);
  const [bin, ...baseArgs] = commandParts;

  return new Promise((resolve) => {
    const child = spawn(bin, [...baseArgs, ...argv], { cwd: manifest.target.kind === 'cli' ? manifest.target.cwd : undefined, shell: false });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (d) => (stdout += d.toString()));
    child.stderr.on('data', (d) => (stderr += d.toString()));
    child.on('error', (err) => {
      resolve({ ok: false, error: String(err), raw: { stdout, stderr } });
    });
    child.on('close', (code) => {
      resolve({
        ok: code === 0,
        output: stdout.trim(),
        error: code === 0 ? undefined : `exited with code ${code}: ${stderr.trim()}`,
        raw: { stdout, stderr, exitCode: code },
      });
    });
  });
}
