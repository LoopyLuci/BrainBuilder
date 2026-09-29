import type { ApiOperation, CapabilityManifest, ExecutionResult } from '../types.js';

function buildUrl(baseUrl: string, op: ApiOperation, args: Record<string, unknown>): { url: string; error?: string } {
  let path = op.path;
  const query = new URLSearchParams();

  for (const param of op.params) {
    const provided = args[param.name];
    if (provided === undefined) {
      if (param.required) return { url: '', error: `missing required param "${param.name}"` };
      continue;
    }
    if (param.in === 'path') {
      path = path.replace(`{${param.name}}`, encodeURIComponent(String(provided)));
    } else if (param.in === 'query') {
      query.set(param.name, String(provided));
    }
  }

  const qs = query.toString();
  return { url: `${baseUrl.replace(/\/$/, '')}${path}${qs ? `?${qs}` : ''}` };
}

export async function executeApi(manifest: CapabilityManifest, operationName: string, args: Record<string, unknown>): Promise<ExecutionResult> {
  if (manifest.target.kind !== 'api') return { ok: false, error: `manifest target is not an api target (got "${manifest.target.kind}")` };
  const op = manifest.operations.find((o) => o.kind === 'api' && o.name === operationName) as ApiOperation | undefined;
  if (!op) return { ok: false, error: `no api operation named "${operationName}" in manifest "${manifest.name}"` };

  const { url, error } = buildUrl(manifest.target.baseUrl, op, args);
  if (error) return { ok: false, error };

  const headers: Record<string, string> = { ...manifest.target.headers };
  for (const param of op.params) {
    if (param.in === 'header' && args[param.name] !== undefined) headers[param.name] = String(args[param.name]);
  }

  let body: string | undefined;
  if (op.hasBody && args.body !== undefined) {
    body = JSON.stringify(args.body);
    headers['content-type'] = 'application/json';
  }

  try {
    const res = await fetch(url, { method: op.method, headers, body });
    const text = await res.text();
    let parsed: unknown = text;
    try {
      parsed = text ? JSON.parse(text) : undefined;
    } catch {
      /* not JSON — keep raw text */
    }
    return {
      ok: res.ok,
      output: parsed,
      error: res.ok ? undefined : `HTTP ${res.status} ${res.statusText}`,
      raw: { status: res.status, url },
    };
  } catch (err) {
    return { ok: false, error: String(err) };
  }
}
