// Launches a real, OS-installed Chromium-based browser headless with a CDP
// debug port, so the GUI layer is tested against an actual page instead of
// a mock. No npm dependency (no puppeteer/playwright) — just the browser
// already on the machine, the same one BrainBuilder's own WebView2 runtime
// is built on. If no such browser is found, tests using this skip
// themselves rather than failing a machine that doesn't have one.
import { spawn, type ChildProcess } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { existsSync } from 'node:fs';

const CANDIDATE_PATHS = [
  'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
  'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
  'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
  '/usr/bin/google-chrome',
  '/usr/bin/chromium-browser',
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
];

export function findHeadlessBrowser(): string | null {
  return CANDIDATE_PATHS.find((p) => existsSync(p)) ?? null;
}

export interface HeadlessBrowser {
  cdpUrl: string;
  close: () => void;
}

async function waitForCdp(cdpUrl: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`${cdpUrl}/json/list`);
      if (res.ok) return;
    } catch {
      /* not up yet */
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`CDP endpoint at ${cdpUrl} did not become ready within ${timeoutMs}ms`);
}

export async function launchHeadlessBrowser(url: string, port: number): Promise<HeadlessBrowser> {
  const binary = findHeadlessBrowser();
  if (!binary) throw new Error('no Chromium-based browser found on this machine');

  const userDataDir = mkdtempSync(path.join(tmpdir(), 'universal-harness-headless-'));
  const child: ChildProcess = spawn(
    binary,
    ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${userDataDir}`, '--no-first-run', '--disable-gpu', url],
    { stdio: 'ignore' },
  );

  const cdpUrl = `http://127.0.0.1:${port}`;
  try {
    await waitForCdp(cdpUrl, 15_000);
  } catch (err) {
    child.kill();
    throw err;
  }

  return {
    cdpUrl,
    close: () => {
      child.kill();
      try {
        rmSync(userDataDir, { recursive: true, force: true });
      } catch {
        /* best-effort cleanup */
      }
    },
  };
}
