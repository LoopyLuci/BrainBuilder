import { test } from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { connectCdp, listCdpPageTargets } from '../src/cdp-client.js';
import { findHeadlessBrowser, launchHeadlessBrowser } from './fixtures/launch-headless-browser.js';

const page1Url = pathToFileURL(path.join(process.cwd(), 'test/fixtures/fixture-gui-page.html')).toString();
const page2Url = pathToFileURL(path.join(process.cwd(), 'test/fixtures/fixture-gui-page-2.html')).toString();

test('connectCdp picks the right window when a target exposes more than one page', async (t) => {
  if (!findHeadlessBrowser()) {
    t.skip('no Chromium-based browser found on this machine');
    return;
  }

  const browser = await launchHeadlessBrowser(page1Url, 9334);
  t.after(() => browser.close());

  // Open a second tab in the same browser instance, mirroring apps (like
  // BrainBuilder) that expose more than one window over the same CDP port.
  await fetch(`${browser.cdpUrl}/json/new?${encodeURIComponent(page2Url)}`, { method: 'PUT' });

  const pages = await listCdpPageTargets(browser.cdpUrl);
  assert.ok(pages.length >= 2, `expected at least 2 page targets, got ${pages.length}`);

  // Ambiguous: no urlIncludes given, connects to whichever page came first — proven separately, not asserted on here.
  const client1 = await connectCdp(browser.cdpUrl, { urlIncludes: page1Url });
  const marker1 = await client1.evaluate<boolean>(`!!document.querySelector('[data-testid="my-button"]')`);
  assert.equal(marker1, true, 'connecting with urlIncludes=page1Url should land on page 1');
  client1.close();

  const client2 = await connectCdp(browser.cdpUrl, { urlIncludes: page2Url });
  const marker2 = await client2.evaluate<boolean>(`!!document.querySelector('[data-testid="only-on-page-2"]')`);
  assert.equal(marker2, true, 'connecting with urlIncludes=page2Url should land on page 2');
  client2.close();
});
