import { test } from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { introspectApi } from '../src/introspect/api.js';
import { executeApi } from '../src/execute/api.js';
import { startFixtureApiServer } from './fixtures/fixture-api-server.js';

const specPath = path.join(process.cwd(), 'test/fixtures/fixture-api.openapi.json');

test('introspectApi + executeApi round-trip against a real local HTTP server', async (t) => {
  const { port, close } = await startFixtureApiServer();
  t.after(close);

  const manifest = await introspectApi({ spec: specPath, baseUrl: `http://127.0.0.1:${port}` });
  const names = manifest.operations.map((o) => o.name).sort();
  assert.deepEqual(names, ['create_item', 'get_item', 'list_items']);

  const getResult = await executeApi(manifest, 'get_item', { id: '1' });
  assert.equal(getResult.ok, true);
  assert.deepEqual(getResult.output, { id: '1', name: 'widget' });

  const missing = await executeApi(manifest, 'get_item', { id: '999' });
  assert.equal(missing.ok, false);
  assert.match(missing.error ?? '', /404/);

  const listResult = await executeApi(manifest, 'list_items', { query: 'widg' });
  assert.equal(listResult.ok, true);
  assert.deepEqual(listResult.output, [{ id: '1', name: 'widget' }]);

  const createResult = await executeApi(manifest, 'create_item', { body: { name: 'gadget' } });
  assert.equal(createResult.ok, true);
  assert.deepEqual(createResult.output, { id: '2', name: 'gadget' });

  const verifyResult = await executeApi(manifest, 'get_item', { id: '2' });
  assert.equal(verifyResult.ok, true);
  assert.deepEqual(verifyResult.output, { id: '2', name: 'gadget' });
});

test('executeApi rejects missing required params before sending a request', async () => {
  const manifest = await introspectApi({ spec: specPath, baseUrl: 'http://127.0.0.1:1' });
  const result = await executeApi(manifest, 'get_item', {});
  assert.equal(result.ok, false);
  assert.match(result.error ?? '', /missing required param "id"/);
});
