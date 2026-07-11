// Turns one or more Capability Manifests into a standard MCP server over
// stdio — the interoperability layer. Any MCP client (Claude Code, Claude
// Desktop, or any other agent framework that speaks MCP) can connect,
// list tools, and call them, without knowing or caring whether a given tool
// is secretly a CLI invocation, an HTTP request, or a GUI click.
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { CallToolRequestSchema, ListToolsRequestSchema } from '@modelcontextprotocol/sdk/types.js';
import type { CapabilityManifest, Operation } from '../types.js';
import { executeOperation } from '../execute/index.js';

function slugify(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
}

interface ToolEntry {
  manifest: CapabilityManifest;
  operation: Operation;
}

function jsonSchemaFor(op: Operation): Record<string, unknown> {
  const properties: Record<string, unknown> = {};
  const required: string[] = [];
  for (const p of op.params) {
    properties[p.name] = {
      type: p.type === 'number' ? 'number' : p.type === 'boolean' ? 'boolean' : 'string',
      description: p.description,
    };
    if (p.required) required.push(p.name);
  }
  if (op.kind === 'api' && op.hasBody) {
    properties.body = { type: 'object', description: 'JSON request body' };
  }
  return { type: 'object', properties, required };
}

/** Builds the tool registry (name -> manifest+operation) for one or more manifests, namespacing by manifest name to avoid collisions. */
export function buildToolRegistry(manifests: CapabilityManifest[]): Map<string, ToolEntry> {
  const registry = new Map<string, ToolEntry>();
  for (const manifest of manifests) {
    const namespace = slugify(manifest.name);
    for (const operation of manifest.operations) {
      const toolName = `${namespace}__${operation.name}`;
      registry.set(toolName, { manifest, operation });
    }
  }
  return registry;
}

export interface McpHarnessServerOptions {
  name?: string;
  version?: string;
}

export function createMcpServer(manifests: CapabilityManifest[], options: McpHarnessServerOptions = {}): Server {
  const registry = buildToolRegistry(manifests);
  const server = new Server(
    { name: options.name ?? 'universal-harness', version: options.version ?? '0.1.0' },
    { capabilities: { tools: {} } },
  );

  server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: [...registry.entries()].map(([name, { manifest, operation }]) => ({
      name,
      description: `[${manifest.name} / ${operation.kind}] ${operation.description}`,
      inputSchema: jsonSchemaFor(operation),
    })),
  }));

  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const entry = registry.get(request.params.name);
    if (!entry) {
      return { content: [{ type: 'text', text: `unknown tool "${request.params.name}"` }], isError: true };
    }
    const args = (request.params.arguments ?? {}) as Record<string, unknown>;
    const result = await executeOperation(entry.manifest, entry.operation.name, args);
    const payload = result.ok ? (result.output ?? { ok: true }) : { error: result.error };
    const text = typeof payload === 'string' ? payload : JSON.stringify(payload);
    return {
      content: [{ type: 'text', text }],
      isError: !result.ok,
    };
  });

  return server;
}

/** Starts an MCP server over stdio and keeps running until the transport closes (i.e. the client disconnects). Call this from a process whose only job is to be the MCP server. */
export async function serveManifestsOverStdio(manifests: CapabilityManifest[], options?: McpHarnessServerOptions): Promise<void> {
  const server = createMcpServer(manifests, options);
  const transport = new StdioServerTransport();
  await server.connect(transport);
}
