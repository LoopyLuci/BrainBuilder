import { connectCdp } from '../cdp-client.js';
import type { CapabilityManifest, ExecutionResult, GuiOperation } from '../types.js';

// React (and most modern frameworks) track input state via a JS property
// setter, not the DOM attribute — a plain `el.value = x` gets silently
// reverted on the next render. Going through the native prototype setter
// before dispatching `input` is the same trick browser automation tools use
// to make a fill actually stick on a controlled component.
function actionScript(selector: string, action: GuiOperation['action'], value: string | undefined): string {
  const sel = JSON.stringify(selector);
  const val = JSON.stringify(value ?? '');
  return `
(function() {
  var el = document.querySelector(${sel});
  if (!el) return { ok: false, error: 'no element matches selector ' + ${sel} };
  ${
    action === 'click'
      ? `el.click(); return { ok: true };`
      : action === 'fill'
        ? `
  var tag = el.tagName;
  var proto = tag === 'TEXTAREA' ? window.HTMLTextAreaElement.prototype
    : tag === 'SELECT' ? window.HTMLSelectElement.prototype
    : window.HTMLInputElement.prototype;
  var setter = Object.getOwnPropertyDescriptor(proto, 'value').set;
  setter.call(el, ${val});
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
  return { ok: true };`
        : action === 'read'
          ? `return { ok: true, value: 'value' in el ? el.value : el.textContent };`
          : `return { ok: true, value: true };`
  }
})()`;
}

export async function executeGui(manifest: CapabilityManifest, operationName: string, args: Record<string, unknown>): Promise<ExecutionResult> {
  if (manifest.target.kind !== 'gui') return { ok: false, error: `manifest target is not a gui target (got "${manifest.target.kind}")` };
  const op = manifest.operations.find((o) => o.kind === 'gui' && o.name === operationName) as GuiOperation | undefined;
  if (!op) return { ok: false, error: `no gui operation named "${operationName}" in manifest "${manifest.name}"` };

  if (op.action === 'fill' && args.value === undefined) return { ok: false, error: 'missing required param "value"' };

  const client = await connectCdp(manifest.target.cdpUrl, { urlIncludes: manifest.target.urlIncludes });
  try {
    const script = actionScript(op.selector, op.action, args.value !== undefined ? String(args.value) : undefined);
    const result = await client.evaluate<{ ok: boolean; error?: string; value?: unknown }>(script);
    return { ok: result.ok, output: result.value, error: result.error, raw: result };
  } catch (err) {
    return { ok: false, error: String(err) };
  } finally {
    client.close();
  }
}

/** Convenience: check whether an element for a given selector currently exists, without needing a manifest operation for it. */
export async function guiElementExists(cdpUrl: string, selector: string): Promise<boolean> {
  const client = await connectCdp(cdpUrl);
  try {
    return await client.evaluate<boolean>(`!!document.querySelector(${JSON.stringify(selector)})`);
  } finally {
    client.close();
  }
}
