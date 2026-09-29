// A minimal Chrome DevTools Protocol client: connect to a debuggable page
// (any Chromium-based surface — Chrome/Edge, Electron, or a Tauri app using
// the WebView2/WebKit CDP bridge) and evaluate JS in it. This is the same
// "fetch /json/list, open a raw WebSocket, send Runtime.evaluate" pattern
// used ad hoc throughout BrainBuilder's own tutorial-verification scripts —
// promoted here into a reusable, tested client so any introspector/executor
// can drive a real page without hand-rolling the protocol each time.
export interface CdpClient {
  evaluate<T = unknown>(expression: string): Promise<T>;
  close(): void;
}

interface CdpTarget {
  type: string;
  url: string;
  title: string;
  webSocketDebuggerUrl: string;
}

export interface ConnectCdpOptions {
  /**
   * Substring to match against a target's URL when more than one 'page'
   * target exists (multi-window apps — Tauri/Electron apps with a secondary
   * window, or a browser with multiple tabs — routinely expose several).
   * Without this, the first page target is used, which is a guess, not a
   * guarantee it's the one you want.
   */
  urlIncludes?: string;
}

export async function listCdpPageTargets(cdpUrl: string): Promise<{ url: string; title: string }[]> {
  const listRes = await fetch(`${cdpUrl.replace(/\/$/, '')}/json/list`);
  const targets = (await listRes.json()) as CdpTarget[];
  return targets.filter((t) => t.type === 'page').map((t) => ({ url: t.url, title: t.title }));
}

export async function connectCdp(cdpUrl: string, options: ConnectCdpOptions = {}): Promise<CdpClient> {
  const listRes = await fetch(`${cdpUrl.replace(/\/$/, '')}/json/list`);
  const targets = (await listRes.json()) as CdpTarget[];
  const pages = targets.filter((t) => t.type === 'page');
  // Exact match wins first — otherwise a caller passing a full URL (e.g. the
  // app's root "http://tauri.localhost/") can accidentally also match a
  // secondary window whose URL happens to start with the same prefix (e.g.
  // "http://tauri.localhost/assistant.html" contains that prefix too).
  const page = options.urlIncludes
    ? (pages.find((t) => t.url === options.urlIncludes) ?? pages.find((t) => t.url.includes(options.urlIncludes!)))
    : pages[0];
  if (!page) {
    const seen = pages.map((t) => `${t.title} (${t.url})`).join(', ') || 'none';
    throw new Error(`no matching page target found at ${cdpUrl}/json/list (urlIncludes: ${options.urlIncludes ?? 'unset'}). Available: ${seen}`);
  }

  const ws = new WebSocket(page.webSocketDebuggerUrl);
  let nextId = 0;
  const pending = new Map<number, { resolve: (v: any) => void; reject: (e: Error) => void }>();

  function send(method: string, params: Record<string, unknown> = {}): Promise<any> {
    return new Promise((resolve, reject) => {
      const id = ++nextId;
      pending.set(id, { resolve, reject });
      ws.send(JSON.stringify({ id, method, params }));
    });
  }

  await new Promise<void>((resolve, reject) => {
    ws.addEventListener('open', () => resolve());
    ws.addEventListener('error', () => reject(new Error(`failed to connect to ${page.webSocketDebuggerUrl}`)));
  });

  ws.addEventListener('message', (ev: any) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id)!;
      pending.delete(msg.id);
      if (msg.error) reject(new Error(JSON.stringify(msg.error)));
      else resolve(msg.result);
    }
  });

  await send('Runtime.enable');

  return {
    async evaluate<T>(expression: string): Promise<T> {
      const result = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
      if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
      return result.result.value as T;
    },
    close() {
      ws.close();
    },
  };
}
