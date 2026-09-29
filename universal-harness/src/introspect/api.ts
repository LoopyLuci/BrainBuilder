// Minimal OpenAPI 3.x reader — deliberately not a full spec validator (no
// extra dependency, stays portable). Reads exactly what's needed to build
// callable operations: paths, methods, operationId, parameters, and whether
// a request body exists. Swagger 2.0 and YAML specs are out of scope for v1
// (documented limitation, not a silent wrong answer) — convert to OpenAPI
// 3.x JSON first.
import { readFile } from 'node:fs/promises';
import type { ApiOperation, ApiParamSpec, CapabilityManifest } from '../types.js';

export interface ApiIntrospectOptions {
  /** Path to a local OpenAPI 3.x JSON file, or a URL serving one. */
  spec: string;
  /** Overrides the spec's own `servers[0].url` if provided. */
  baseUrl?: string;
  headers?: Record<string, string>;
  name?: string;
}

interface OpenApiParameter {
  name: string;
  in: 'path' | 'query' | 'header' | 'cookie';
  required?: boolean;
  description?: string;
  schema?: { type?: string };
}

interface OpenApiOperation {
  operationId?: string;
  summary?: string;
  description?: string;
  parameters?: OpenApiParameter[];
  requestBody?: unknown;
}

interface OpenApiDoc {
  info?: { title?: string; description?: string };
  servers?: { url: string }[];
  paths: Record<string, Record<string, OpenApiOperation>>;
}

const HTTP_METHODS = new Set(['get', 'post', 'put', 'patch', 'delete', 'head', 'options']);

async function loadSpec(spec: string): Promise<OpenApiDoc> {
  const isUrl = /^https?:\/\//.test(spec);
  const text = isUrl ? await (await fetch(spec)).text() : await readFile(spec, 'utf-8');
  return JSON.parse(text) as OpenApiDoc;
}

export async function introspectApi(options: ApiIntrospectOptions): Promise<CapabilityManifest> {
  const doc = await loadSpec(options.spec);
  const baseUrl = options.baseUrl ?? doc.servers?.[0]?.url ?? '';
  const operations: ApiOperation[] = [];

  for (const [path, methods] of Object.entries(doc.paths ?? {})) {
    for (const [method, op] of Object.entries(methods)) {
      if (!HTTP_METHODS.has(method.toLowerCase())) continue;
      const params: ApiParamSpec[] = (op.parameters ?? [])
        .filter((p): p is OpenApiParameter & { in: 'path' | 'query' | 'header' } => p.in === 'path' || p.in === 'query' || p.in === 'header')
        .map((p) => ({
          name: p.name,
          in: p.in,
          required: p.required ?? p.in === 'path',
          description: p.description,
          type: (p.schema?.type as ApiParamSpec['type']) ?? 'string',
        }));

      operations.push({
        kind: 'api',
        name: op.operationId ?? `${method.toLowerCase()}_${path.replace(/[^\w]+/g, '_').replace(/^_+|_+$/g, '')}`,
        description: op.summary ?? op.description ?? `${method.toUpperCase()} ${path}`,
        method: method.toUpperCase(),
        path,
        params,
        hasBody: op.requestBody !== undefined,
      });
    }
  }

  return {
    name: options.name ?? doc.info?.title ?? options.spec,
    description: doc.info?.description,
    target: { kind: 'api', baseUrl, headers: options.headers },
    generatedAt: new Date().toISOString(),
    operations,
  };
}
