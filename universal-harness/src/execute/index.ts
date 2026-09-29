import type { CapabilityManifest, ExecutionResult } from '../types.js';
import { executeCli } from './cli.js';
import { executeApi } from './api.js';
import { executeGui } from './gui.js';

/** Dispatches to the right executor based on the manifest's target kind — the one call site every caller (MCP server, `harness call`) should use. */
export async function executeOperation(manifest: CapabilityManifest, operationName: string, args: Record<string, unknown>): Promise<ExecutionResult> {
  switch (manifest.target.kind) {
    case 'cli':
      return executeCli(manifest, operationName, args);
    case 'api':
      return executeApi(manifest, operationName, args);
    case 'gui':
      return executeGui(manifest, operationName, args);
  }
}

export { executeCli, executeApi, executeGui };
