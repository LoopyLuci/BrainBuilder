// Walks the live DOM of a CDP-debuggable page and turns interactive elements
// into click/fill operations. Selector strategy prefers attributes that are
// meant to be stable identifiers (data-testid, data-tutorial, id) over a
// generated nth-child path, and says so honestly via selectorConfidence —
// an agent (or a human reviewing a generated manifest) should treat
// 'best-effort' operations as more likely to break across UI changes.
import { connectCdp } from '../cdp-client.js';
import type { CapabilityManifest, GuiOperation } from '../types.js';

export interface GuiIntrospectOptions {
  cdpUrl: string;
  name?: string;
  /** Cap on how many elements to turn into operations, to avoid a pathological page producing thousands. */
  maxOperations?: number;
  /** Substring matched against a page target's URL — required when the app exposes more than one window/tab over CDP. */
  urlIncludes?: string;
}

// Executed inside the target page via Runtime.evaluate — must be a single
// self-contained expression with no references to the outer TS scope.
const SCAN_SCRIPT = (maxOperations: number) => `
(function() {
  function buildSelector(el) {
    if (el.getAttribute('data-testid')) return { selector: '[data-testid="' + el.getAttribute('data-testid') + '"]', confidence: 'stable' };
    if (el.getAttribute('data-tutorial')) return { selector: '[data-tutorial="' + el.getAttribute('data-tutorial') + '"]', confidence: 'stable' };
    if (el.id) return { selector: '#' + CSS.escape(el.id), confidence: 'stable' };
    if (el.getAttribute('aria-label')) return { selector: el.tagName.toLowerCase() + '[aria-label="' + el.getAttribute('aria-label') + '"]', confidence: 'stable' };
    if (el.getAttribute('name')) return { selector: el.tagName.toLowerCase() + '[name="' + el.getAttribute('name') + '"]', confidence: 'stable' };
    // Fallback: nth-child path from a nearby ancestor with an id, or from body.
    var parts = [];
    var node = el;
    for (var depth = 0; depth < 5 && node && node.nodeType === 1; depth++) {
      var tag = node.tagName.toLowerCase();
      if (node.id) { parts.unshift('#' + CSS.escape(node.id)); break; }
      var parent = node.parentElement;
      if (!parent) { parts.unshift(tag); break; }
      var idx = Array.prototype.indexOf.call(parent.children, node) + 1;
      parts.unshift(tag + ':nth-child(' + idx + ')');
      node = parent;
    }
    return { selector: parts.join(' > '), confidence: 'best-effort' };
  }

  function labelFor(el) {
    return el.getAttribute('aria-label') || el.getAttribute('data-testid') || el.getAttribute('data-tutorial') ||
      el.getAttribute('placeholder') || (el.textContent || '').trim().slice(0, 40) || el.id || el.tagName.toLowerCase();
  }

  function slugify(s) {
    return (s || 'element').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'element';
  }

  var results = [];
  var seenSelectors = new Set();
  var seenNames = new Set();
  var candidates = document.querySelectorAll('button, a[href], [role="button"], input, textarea, select, [onclick]');

  for (var i = 0; i < candidates.length && results.length < ${maxOperations}; i++) {
    var el = candidates[i];
    var rect = el.getBoundingClientRect();
    if (rect.width === 0 && rect.height === 0) continue; // hidden/detached
    var built = buildSelector(el);
    if (seenSelectors.has(built.selector)) continue;
    seenSelectors.add(built.selector);

    var tag = el.tagName.toLowerCase();
    var type = (el.getAttribute('type') || '').toLowerCase();
    var isFillable = (tag === 'input' && !['checkbox', 'radio', 'button', 'submit', 'reset'].includes(type)) || tag === 'textarea' || tag === 'select';
    var action = isFillable ? 'fill' : 'click';

    var baseName = slugify(labelFor(el));
    var name = baseName;
    var suffix = 2;
    while (seenNames.has(name)) { name = baseName + '-' + suffix; suffix++; }
    seenNames.add(name);

    results.push({
      selector: built.selector,
      selectorConfidence: built.confidence,
      action: action,
      name: name,
      description: (tag + (type ? '[type=' + type + ']' : '')) + ': ' + labelFor(el),
    });
  }
  return results;
})()
`;

interface ScanResult {
  selector: string;
  selectorConfidence: 'stable' | 'best-effort';
  action: 'click' | 'fill';
  name: string;
  description: string;
}

export async function introspectGui(options: GuiIntrospectOptions): Promise<CapabilityManifest> {
  const client = await connectCdp(options.cdpUrl, { urlIncludes: options.urlIncludes });
  try {
    const scanned = await client.evaluate<ScanResult[]>(SCAN_SCRIPT(options.maxOperations ?? 200));
    const operations: GuiOperation[] = scanned.map((s) => ({
      kind: 'gui',
      name: s.name,
      description: s.description,
      action: s.action,
      selector: s.selector,
      selectorConfidence: s.selectorConfidence,
      params: s.action === 'fill' ? [{ name: 'value', required: true, type: 'string', description: 'Text to type into the element' }] : [],
    }));

    return {
      name: options.name ?? `GUI at ${options.cdpUrl}`,
      description: `Interactive elements discovered live via CDP at ${options.cdpUrl}`,
      target: { kind: 'gui', cdpUrl: options.cdpUrl, urlIncludes: options.urlIncludes },
      generatedAt: new Date().toISOString(),
      operations,
    };
  } finally {
    client.close();
  }
}
