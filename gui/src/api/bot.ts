import { invoke } from '@tauri-apps/api/tauri';

export interface BotConfigDto {
  bot_id: string;
  platform: string;
  enabled: boolean;
  credentials: Record<string, string>;
}

export interface CallSessionDto {
  id: string;
  conversation: string;
  user: string;
  platform: string;
  started_at: number;
  ended_at: number | null;
}

export async function botListAdapters(): Promise<BotConfigDto[]> {
  return invoke('bot_list_adapters');
}

export async function botStartAdapter(config: BotConfigDto): Promise<void> {
  return invoke('bot_start_adapter', { config });
}

export async function botAdapterHealth(platform: string): Promise<string> {
  return invoke('bot_adapter_health', { platform });
}

export async function botSendMessage(platform: string, conversation: string, text: string): Promise<void> {
  return invoke('bot_send_message', { platform, conversation, text });
}

export async function botStartCall(platform: string, conversation: string, user: string): Promise<CallSessionDto> {
  return invoke('bot_start_call', { platform, conversation, user });
}

export async function botEndCall(callId: string): Promise<void> {
  return invoke('bot_end_call', { callId });
}
