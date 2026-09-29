// A tiny real HTTP server used only by universal-harness's own tests, so the
// API layer is proven against a real request/response round-trip instead of
// mocked fetch.
import http from 'node:http';
import type { AddressInfo } from 'node:net';

interface Item {
  id: string;
  name: string;
}

export function startFixtureApiServer(): Promise<{ port: number; close: () => Promise<void> }> {
  const items = new Map<string, Item>([['1', { id: '1', name: 'widget' }]]);

  const server = http.createServer(async (req, res) => {
    const url = new URL(req.url ?? '/', 'http://localhost');

    if (req.method === 'GET' && url.pathname === '/items') {
      const query = url.searchParams.get('query') ?? '';
      const matches = [...items.values()].filter((i) => i.name.includes(query));
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify(matches));
      return;
    }

    const itemMatch = url.pathname.match(/^\/items\/(\w+)$/);
    if (req.method === 'GET' && itemMatch) {
      const item = items.get(itemMatch[1]);
      if (!item) {
        res.writeHead(404, { 'content-type': 'application/json' });
        res.end(JSON.stringify({ error: 'not found' }));
        return;
      }
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(JSON.stringify(item));
      return;
    }

    if (req.method === 'POST' && url.pathname === '/items') {
      const chunks: Buffer[] = [];
      for await (const chunk of req) chunks.push(chunk);
      const parsed = JSON.parse(Buffer.concat(chunks).toString('utf-8') || '{}');
      const id = String(items.size + 1);
      const item: Item = { id, name: parsed.name };
      items.set(id, item);
      res.writeHead(201, { 'content-type': 'application/json' });
      res.end(JSON.stringify(item));
      return;
    }

    res.writeHead(404, { 'content-type': 'application/json' });
    res.end(JSON.stringify({ error: 'not found' }));
  });

  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => {
      const port = (server.address() as AddressInfo).port;
      resolve({
        port,
        close: () => new Promise((r) => server.close(() => r())),
      });
    });
  });
}
