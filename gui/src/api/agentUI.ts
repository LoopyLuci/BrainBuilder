import {
  webviewDebugEval,
  webviewDebugQuery,
  webviewDebugClick,
  webviewDebugFill,
  webviewDebugSnapshot,
  webviewDebugGetState,
  webviewDebugSetEnabled,
  webviewDebugIsEnabled,
} from './webviewDebug';

export async function isAgentDebugEnabled(): Promise<boolean> {
  return webviewDebugIsEnabled();
}

export async function setAgentDebugEnabled(enabled: boolean): Promise<void> {
  return webviewDebugSetEnabled(enabled);
}

export async function waitForSelector(
  selector: string,
  timeoutMs = 5000,
  pollMs = 250,
): Promise<boolean> {
  if (!(await webviewDebugIsEnabled())) {
    await webviewDebugSetEnabled(true);
  }

  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const res = await webviewDebugQuery(selector);
    if (res.count > 0) {
      return true;
    }
    await new Promise((r) => setTimeout(r, pollMs));
  }
  return false;
}

export async function clickByText(text: string, exact = false): Promise<{
  success: boolean;
  selector?: string;
  result?: string;
  error?: string;
}> {
  const normalized = text.trim().toLowerCase();
  const js = exact
    ? `(() => {
        const target = Array.from(document.querySelectorAll('button,[role="button"],a,[tabindex]')).find(el => el.textContent.trim().toLowerCase() === ${JSON.stringify(normalized)});
        if (!target) return { success: false, error: 'text not found' };
        target.click();
        return { success: true, result: 'clicked' };
      })()`
    : `(() => {
        const target = Array.from(document.querySelectorAll('button,[role="button"],a,[tabindex]')).find(el => el.textContent.toLowerCase().includes(${JSON.stringify(normalized)}));
        if (!target) return { success: false, error: 'text not found' };
        target.click();
        return { success: true, result: 'clicked' };
      })()`;

  const res = await webviewDebugEval(js);
  return res;
}

export async function fillForm(
  data: Record<string, string>,
): Promise<
  Array<{ selector: string; success: boolean; error?: string }>
> {
  const results: Array<{ selector: string; success: boolean; error?: string }> =
    [];
  for (const [selector, value] of Object.entries(data)) {
    try {
      const r = await webviewDebugFill(selector, value);
      results.push({
        selector,
        success: r.success,
        error: r.error,
      });
    } catch (e) {
      results.push({
        selector,
        success: false,
        error: e instanceof Error ? e.message : String(e),
      });
    }
  }
  return results;
}

export async function assertVisible(
  selector: string,
  timeoutMs = 3000,
): Promise<boolean> {
  return waitForSelector(selector, timeoutMs);
}

export async function getUiSnapshot() {
  return webviewDebugGetState();
}
