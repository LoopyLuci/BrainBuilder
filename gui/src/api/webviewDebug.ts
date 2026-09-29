import { invoke } from '@tauri-apps/api/tauri';

export interface JsEvalResult {
  success: boolean;
  result?: any;
  error?: string;
}

export interface DomQueryResult {
  selector: string;
  count: number;
  html?: string;
  text?: string;
}

export interface UiElement {
  tag: string;
  id?: string | null;
  class?: string | null;
  text?: string | null;
  href?: string | null;
  xpath: string;
}

export interface UiSnapshot {
  url: string;
  title: string;
  elements: UiElement[];
}

export interface DebugState {
  enabled: boolean;
  log_level: string;
}

export async function webviewDebugEval(js: string): Promise<JsEvalResult> {
  return invoke('webview_debug_eval', { js });
}

export async function webviewDebugQuery(selector: string): Promise<DomQueryResult> {
  return invoke('webview_debug_query', { selector });
}

export async function webviewDebugClick(selector: string): Promise<JsEvalResult> {
  return invoke('webview_debug_click', { selector });
}

export async function webviewDebugFill(selector: string, value: string): Promise<JsEvalResult> {
  return invoke('webview_debug_fill', { selector, value });
}

export async function webviewDebugSnapshot(): Promise<UiSnapshot> {
  return invoke('webview_debug_snapshot');
}

export async function webviewDebugGetState(): Promise<UiSnapshot> {
  return invoke('webview_debug_get_state');
}

export async function webviewDebugSetEnabled(enabled: boolean): Promise<void> {
  return invoke('webview_debug_set_enabled', { enabled });
}

export async function webviewDebugIsEnabled(): Promise<boolean> {
  return invoke('webview_debug_is_enabled');
}
