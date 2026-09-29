import { test } from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { introspectCli } from '../src/introspect/cli.js';
import { introspectApi } from '../src/introspect/api.js';
import { createMcpServer, buildToolRegistry } from '../src/mcp/server.js';
import { startFixtureApiServer } from './fixtures/fixture-api-server.js';

const fixtureCli = path.join(process.cwd(), 'test/fixtures/fixture-cli.mjs');
const fixtureApiSpec = path.join(process.cwd(), 'test/fixtures/fixture-api.openapi.json');

test('MCP server exposes a CLI manifest as real, callable tools to a real MCP client', async (t) => {
  const cliManifest = await introspectCli({ command: `node ${fixtureCli}` });
  const server = createMcpServer([cliManifest], { name: 'fixture-cli-harness' });
  const expectedNames = [...buildToolRegistry([cliManifest]).keys()].sort();

  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  const client = new Client({ name: 'test-client', version: '0.0.0' });
  await Promise.all([client.connect(clientTransport), server.connect(serverTransport)]);
  t.after(async () => {
    await client.close();
  });

  const { tools } = await client.listTools();
  assert.deepEqual(tools.map((tool) => tool.name).sort(), expectedNames);

  const addTool = tools.find((tool) => tool.name.endsWith('__add'))!;
  const result = await client.callTool({ name: addTool.name, arguments: { a: 4, b: 5 } });
  assert.equal(result.isError, false);
  assert.equal((result.content as any)[0].text, '9');
});

test('MCP server exposes an API manifest as real, callable tools against a real HTTP server', async (t) => {
  const { port, close } = await startFixtureApiServer();
  t.after(close);

  const apiManifest = await introspectApi({ spec: fixtureApiSpec, baseUrl: `http://127.0.0.1:${port}`, name: 'Fixture API' });
  const server = createMcpServer([apiManifest]);

  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  const client = new Client({ name: 'test-client-2', version: '0.0.0' });
  await Promise.all([client.connect(clientTransport), server.connect(serverTransport)]);
  t.after(async () => {
    await client.close();
  });

  const { tools } = await client.listTools();
  const getItemTool = tools.find((tool) => tool.name.endsWith('__get_item'));
  assert.ok(getItemTool);
  assert.ok((getItemTool!.inputSchema.required as string[] | undefined)?.includes('id'));

  const result = await client.callTool({ name: getItemTool!.name, arguments: { id: '1' } });
  assert.equal(result.isError, false);
  assert.deepEqual(JSON.parse((result.content as any)[0].text), { id: '1', name: 'widget' });
});
