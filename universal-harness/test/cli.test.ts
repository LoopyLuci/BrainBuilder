import { test } from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { introspectCli } from '../src/introspect/cli.js';
import { executeCli } from '../src/execute/cli.js';

const fixture = path.join(process.cwd(), 'test/fixtures/fixture-cli.mjs');
const command = `node ${fixture}`;

test('introspectCli discovers subcommands and their flags', async () => {
  const manifest = await introspectCli({ command });
  assert.equal(manifest.target.kind, 'cli');
  const names = manifest.operations.map((o) => o.name).sort();
  assert.deepEqual(names, ['add', 'greet']);

  const greet = manifest.operations.find((o) => o.name === 'greet');
  assert.ok(greet && greet.kind === 'cli');
  const flagNames = greet.params.map((p) => p.flag).sort();
  assert.deepEqual(flagNames, ['--loud', '--name']);
  const nameParam = greet.params.find((p) => p.flag === '--name');
  assert.equal(nameParam?.isSwitch, false);
  const loudParam = greet.params.find((p) => p.flag === '--loud');
  assert.equal(loudParam?.isSwitch, true);
});

test('executeCli runs the real subcommand with constructed flags', async () => {
  const manifest = await introspectCli({ command });
  const result = await executeCli(manifest, 'greet', { name: 'Luci', loud: true });
  assert.equal(result.ok, true);
  assert.equal(result.output, 'HELLO, LUCI!');
});

test('executeCli reports real failures (non-zero exit) as ok:false', async () => {
  const manifest = await introspectCli({ command });
  const result = await executeCli(manifest, 'add', { a: 'not-a-number' });
  assert.equal(result.ok, false);
  assert.match(result.error ?? '', /exited with code 1/);
});

test('executeCli computes a real result end-to-end', async () => {
  const manifest = await introspectCli({ command });
  const result = await executeCli(manifest, 'add', { a: 2, b: 3 });
  assert.equal(result.ok, true);
  assert.equal(result.output, '5');
});
