import { test } from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { introspectGui } from '../src/introspect/gui.js';
import { executeGui } from '../src/execute/gui.js';
import { connectCdp } from '../src/cdp-client.js';
import { findHeadlessBrowser, launchHeadlessBrowser } from './fixtures/launch-headless-browser.js';

const fixtureUrl = pathToFileURL(path.join(process.cwd(), 'test/fixtures/fixture-gui-page.html')).toString();

test('GUI layer: introspect + execute against a real browser page via CDP', async (t) => {
  if (!findHeadlessBrowser()) {
    t.skip('no Chromium-based browser found on this machine');
    return;
  }

  const browser = await launchHeadlessBrowser(fixtureUrl, 9333);
  t.after(() => browser.close());

  const manifest = await introspectGui({ cdpUrl: browser.cdpUrl });
  const guiOps = manifest.operations.filter((o) => o.kind === 'gui');
  const button = guiOps.find((o) => o.selector === '[data-testid="my-button"]');
  const input = guiOps.find((o) => o.selector === '[data-testid="my-input"]');
  assert.ok(button, 'button should be discovered');
  assert.equal(button?.action, 'click');
  assert.equal(button?.selectorConfidence, 'stable');
  assert.ok(input, 'input should be discovered');
  assert.equal(input?.action, 'fill');

  const clickResult = await executeGui(manifest, button!.name, {});
  assert.equal(clickResult.ok, true);

  const client = await connectCdp(browser.cdpUrl);
  const outputText = await client.evaluate<string>(`document.querySelector('[data-testid="output"]').textContent`);
  assert.equal(outputText, 'clicked');

  const fillResult = await executeGui(manifest, input!.name, { value: 'hello from harness' });
  assert.equal(fillResult.ok, true);
  const echoText = await client.evaluate<string>(`document.querySelector('[data-testid="echo"]').textContent`);
  assert.equal(echoText, 'hello from harness');
  client.close();
});
