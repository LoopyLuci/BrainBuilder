import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/tauri";

export interface ChatMessage {
  role: "user" | "assistant" | "system" | "tool";
  content: string;
}

export function useConcierge() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const send = useCallback(async (text: string) => {
    if (!text.trim()) return;
    setMessages((prev) => [...prev, { role: "user", content: text }]);
    setLoading(true);
    setError(null);
    try {
      const reply = await invoke<string>("concierge_chat", { message: text });
      setMessages((prev) => [...prev, { role: "assistant", content: reply }]);
    } catch (e: any) {
      const msg = typeof e === "string" ? e : e?.message ?? "Concierge error";
      setError(msg);
      setMessages((prev) => [
        ...prev,
        { role: "assistant", content: `Error: ${msg}` },
      ]);
    } finally {
      setLoading(false);
    }
  }, []);

  const clear = useCallback(() => {
    setMessages([]);
    setError(null);
  }, []);

  return { messages, loading, error, send, clear };
}
