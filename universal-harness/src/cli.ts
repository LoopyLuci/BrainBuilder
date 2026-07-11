#!/usr/bin/env node
// The `harness` binary — the one entry point an agent or a human needs.
// Subcommands are flat (introspect-cli, not "introspect cli") on purpose:
// node:util.parseArgs doesn't support git-style nested subcommands, and
// hand-rolling that for marginal prettiness isn't worth it here.
import { parseArgs } from 'node:util';
import { readFile, writeFile } from 'node:fs/promises';
import { introspectCli } from './introspect/cli.js';
import { introspectApi } from './introspect/api.js';
import { introspectGui } from './introspect/gui.js';
import { executeOperation } from './execute/index.js';
import { serveManifestsOverStdio } from './mcp/server.js';
import type { CapabilityManifest } from './types.js';

const USAGE = `harness — turn a CLI, HTTP API, or CDP-debuggable GUI app into callable operations.

Usage:
  harness introspect-cli --command "<cmd>" [--cwd <dir>] [--max-depth <n>] --out <file.json>
  harness introspect-api --spec <path-or-url> [--base-url <url>] [--name <name>] --out <file.json>
  harness introspect-gui --cdp-url <url> [--name <name>] [--max-ops <n>] --out <file.json>
  harness list --manifest <file.json>
  harness call --manifest <file.json> --op <name> [--args '<json object>']
  harness mcp --manifest <file.json> [--manifest <file2.json> ...] [--name <server-name>]

Every subcommand prints JSON to stdout unless --out is given, so it composes with jq/agents alike.
`;

async function loadManifest(file: string): Promise<CapabilityManifest> {
  return JSON.parse(await readFile(file, 'utf-8')) as CapabilityManifest;
}

async function emit(manifest: CapabilityManifest, outFile?: string): Promise<void> {
  const json = JSON.stringify(manifest, null, 2);
  if (outFile) {
    await writeFile(outFile, json, 'utf-8');
    console.error(`wrote ${manifest.operations.length} operation(s) to ${outFile}`);
  } else {
    console.log(json);
  }
}

async function main(): Promise<void> {
  const [sub, ...rest] = process.argv.slice(2);

  if (!sub || sub === '--help' || sub === '-h') {
    process.stdout.write(USAGE);
    process.exitCode = sub ? 0 : 1;
    return;
  }

  if (sub === 'introspect-cli') {
    const { values } = parseArgs({
      args: rest,
      options: {
        command: { type: 'string' },
        cwd: { type: 'string' },
        'max-depth': { type: 'string' },
        out: { type: 'string' },
      },
    });
    if (!values.command) throw new Error('--command is required');
    const manifest = await introspectCli({
      command: values.command,
      cwd: values.cwd,
      maxDepth: values['max-depth'] ? Number(values['max-depth']) : undefined,
    });
    await emit(manifest, values.out);
    return;
  }

  if (sub === 'introspect-api') {
    const { values } = parseArgs({
      args: rest,
      options: {
        spec: { type: 'string' },
        'base-url': { type: 'string' },
        name: { type: 'string' },
        out: { type: 'string' },
      },
    });
    if (!values.spec) throw new Error('--spec is required');
    const manifest = await introspectApi({ spec: values.spec, baseUrl: values['base-url'], name: values.name });
    await emit(manifest, values.out);
    return;
  }

  if (sub === 'introspect-gui') {
    const { values } = parseArgs({
      args: rest,
      options: {
        'cdp-url': { type: 'string' },
        name: { type: 'string' },
        'max-ops': { type: 'string' },
        'url-includes': { type: 'string' },
        out: { type: 'string' },
      },
    });
    if (!values['cdp-url']) throw new Error('--cdp-url is required');
    const manifest = await introspectGui({
      cdpUrl: values['cdp-url'],
      name: values.name,
      maxOperations: values['max-ops'] ? Number(values['max-ops']) : undefined,
      urlIncludes: values['url-includes'],
    });
    await emit(manifest, values.out);
    return;
  }

  if (sub === 'list') {
    const { values } = parseArgs({ args: rest, options: { manifest: { type: 'string' } } });
    if (!values.manifest) throw new Error('--manifest is required');
    const manifest = await loadManifest(values.manifest);
    console.log(`${manifest.name} (${manifest.target.kind}) — ${manifest.operations.length} operation(s)\n`);
    for (const op of manifest.operations) {
      const paramList = op.params.map((p) => `${p.name}${p.required ? '' : '?'}`).join(', ');
      console.log(`  ${op.name}(${paramList})  — ${op.description}`);
    }
    return;
  }

  if (sub === 'call') {
    const { values } = parseArgs({
      args: rest,
      options: {
        manifest: { type: 'string' },
        op: { type: 'string' },
        args: { type: 'string' },
      },
    });
    if (!values.manifest || !values.op) throw new Error('--manifest and --op are required');
    const manifest = await loadManifest(values.manifest);
    const args = values.args ? JSON.parse(values.args) : {};
    const result = await executeOperation(manifest, values.op, args);
    console.log(JSON.stringify(result, null, 2));
    process.exitCode = result.ok ? 0 : 1;
    return;
  }

  if (sub === 'mcp') {
    const { values } = parseArgs({
      args: rest,
      options: {
        manifest: { type: 'string', multiple: true },
        name: { type: 'string' },
      },
    });
    if (!values.manifest || values.manifest.length === 0) throw new Error('at least one --manifest is required');
    const manifests = await Promise.all(values.manifest.map(loadManifest));
    console.error(`serving ${manifests.reduce((n, m) => n + m.operations.length, 0)} operation(s) from ${manifests.length} manifest(s) over stdio...`);
    await serveManifestsOverStdio(manifests, { name: values.name });
    return;
  }

  process.stderr.write(`unknown subcommand "${sub}"\n\n${USAGE}`);
  process.exitCode = 1;
}

main().catch((err) => {
  console.error(err instanceof Error ? err.message : String(err));
  process.exitCode = 1;
});
