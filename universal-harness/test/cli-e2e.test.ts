// The strongest proof this toolkit works: spawn the compiled `harness`
// binary as a real, separate OS process (exactly how an agent would use
// it) and drive it over real stdio with a real MCP client — no in-process
// shortcuts anywhere in this test.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';

const execFileAsync = promisify(execFile);
const cliBin = path.join(process.cwd(), 'dist/src/cli.js');
const fixtureCli = path.join(process.cwd(), 'test/fixtures/fixture-cli.mjs');

test('harness binary: introspect-cli writes a real manifest file', async () => {
  const dir = mkdtempSync(path.join(tmpdir(), 'harness-e2e-'));
  const manifestPath = path.join(dir, 'manifest.json');
  try {
    await execFileAsync('node', [cliBin, 'introspect-cli', '--command', `node ${fixtureCli}`, '--out', manifestPath]);
    const manifest = JSON.parse(await import('node:fs/promises').then((fs) => fs.readFile(manifestPath, 'utf-8')));
    assert.equal(manifest.target.kind, 'cli');
    assert.equal(manifest.operations.length, 2);

    const { stdout } = await execFileAsync('node', [cliBin, 'call', '--manifest', manifestPath, '--op', 'add', '--args', '{"a":10,"b":32}']);
    const result = JSON.parse(stdout);
    assert.equal(result.ok, true);
    assert.equal(result.output, '42');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('harness binary: mcp subcommand serves real tools over real stdio to a real MCP client', async (t) => {
  const dir = mkdtempSync(path.join(tmpdir(), 'harness-e2e-mcp-'));
  const manifestPath = path.join(dir, 'manifest.json');
  t.after(() => rmSync(dir, { recursive: true, force: true }));

  await execFileAsync('node', [cliBin, 'introspect-cli', '--command', `node ${fixtureCli}`, '--out', manifestPath]);

  const transport = new StdioClientTransport({ command: 'node', args: [cliBin, 'mcp', '--manifest', manifestPath] });
  const client = new Client({ name: 'e2e-test-client', version: '0.0.0' });
  await client.connect(transport);
  t.after(async () => {
    await client.close();
  });

  const { tools } = await client.listTools();
  assert.ok(tools.some((tool) => tool.name.endsWith('__greet')));

  const greetTool = tools.find((tool) => tool.name.endsWith('__greet'))!;
  const result = await client.callTool({ name: greetTool.name, arguments: { name: 'real subprocess' } });
  assert.equal(result.isError, false);
  assert.equal((result.content as any)[0].text, 'Hello, real subprocess!');
});
