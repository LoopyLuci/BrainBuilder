import { useState, useRef, useEffect } from "react";
import { useConcierge } from "../hooks/useConcierge";
import { useGenUI } from "../generative/store";
import type { GenNode } from "../generative/types";

function tryParseGenUI(text: string): { title: string; root: GenNode } | null {
  // Look for fenced block ```genui ... ```
  const fence = text.match(/```genui\s*([\s\S]*?)```/i);
  if (fence) {
    try {
      const obj = JSON.parse(fence[1]);
      if (obj && obj.root) {
        return { title: obj.title ?? "Agent UI", root: obj.root as GenNode };
      }
      if (obj && obj.type) {
        return { title: "Agent UI", root: obj as GenNode };
      }
    } catch {
      /* ignore */
    }
  }
  // Or raw JSON object with type/root
  const trimmed = text.trim();
  if (trimmed.startsWith("{") && (trimmed.includes('"type"') || trimmed.includes('"root"'))) {
    try {
      const obj = JSON.parse(trimmed);
      if (obj.root) return { title: obj.title ?? "Agent UI", root: obj.root };
      if (obj.type) return { title: "Agent UI", root: obj };
    } catch {
      /* ignore */
    }
  }
  return null;
}

export default function ConciergePanel() {
  const { messages, loading, error, send, clear } = useConcierge();
  const [input, setInput] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);
  const push = useGenUI((s) => s.push);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // When assistant replies, try to materialize generative UI
  useEffect(() => {
    const last = messages[messages.length - 1];
    if (!last || last.role !== "assistant") return;
    const parsed = tryParseGenUI(last.content);
    if (parsed) {
      push(parsed.title, parsed.root);
    }
  }, [messages, push]);

  const onSend = () => {
    if (!input.trim() || loading) return;
    send(input);
    setInput("");
  };

  return (
    <div className="flex flex-col h-full glass-strong">
      <div className="px-3 py-2.5 border-b border-white/5 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="w-2 h-2 rounded-full bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.7)]" />
          <span className="text-sm font-medium">Concierge</span>
        </div>
        <button onClick={clear} className="btn-ghost text-xs px-2 py-1">
          Clear
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        {messages.length === 0 && (
          <div className="text-slate-500 text-sm leading-relaxed space-y-2">
            <p>Ask me to search models, train adapters, or generate UI.</p>
            <p className="text-xs text-slate-600">
              Tip: I can emit <code className="text-blue-300">```genui</code> blocks to build live panels.
            </p>
          </div>
        )}
        {messages.map((m, i) => (
          <div
            key={i}
            className={`flex ${m.role === "user" ? "justify-end" : "justify-start"} gen-enter`}
          >
            <div
              className={`max-w-[92%] px-3 py-2 rounded-2xl text-sm whitespace-pre-wrap leading-relaxed ${
                m.role === "user"
                  ? "bg-gradient-to-br from-blue-600 to-indigo-600 text-white rounded-br-md"
                  : "bg-white/5 border border-white/10 text-slate-100 rounded-bl-md"
              }`}
            >
              {m.content}
            </div>
          </div>
        ))}
        {loading && (
          <div className="text-slate-400 text-sm flex items-center gap-2">
            <span className="inline-block w-1.5 h-1.5 rounded-full bg-blue-400 animate-pulse" />
            Thinking…
          </div>
        )}
        {error && <div className="text-red-400 text-xs">{error}</div>}
        <div ref={bottomRef} />
      </div>

      <div className="p-3 border-t border-white/5 flex gap-2">
        <input
          className="input flex-1"
          placeholder="e.g. Generate a status dashboard…"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !e.shiftKey && onSend()}
          disabled={loading}
        />
        <button
          onClick={onSend}
          disabled={loading || !input.trim()}
          className="btn-primary"
        >
          Send
        </button>
      </div>
    </div>
  );
}
