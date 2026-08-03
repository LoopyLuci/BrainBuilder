import type { GenNode } from "./types";
import { useGenUI } from "./store";

interface Props {
  node: GenNode;
  onAction?: (action: string, payload?: unknown) => void;
}

export function GenRenderer({ node, onAction }: Props) {
  const setField = useGenUI((s) => s.setField);
  const formValues = useGenUI((s) => s.formValues);

  switch (node.type) {
    case "stack": {
      const dir = node.direction === "row" ? "flex-row" : "flex-col";
      const gap = node.gap ?? 8;
      return (
        <div className={`flex ${dir} gen-enter ${node.className ?? ""}`} style={{ gap }}>
          {node.children.map((c, i) => (
            <GenRenderer key={i} node={c} onAction={onAction} />
          ))}
        </div>
      );
    }
    case "text": {
      const cls =
        node.variant === "title"
          ? "text-lg font-semibold tracking-tight"
          : node.variant === "muted"
          ? "text-sm text-slate-400"
          : node.variant === "code"
          ? "font-mono text-xs bg-black/30 px-1.5 py-0.5 rounded"
          : "text-sm text-slate-200";
      return <div className={cls}>{node.content}</div>;
    }
    case "button": {
      const v = node.variant ?? "primary";
      const cls =
        v === "danger" ? "btn-danger" : v === "ghost" ? "btn-ghost" : "btn-primary";
      return (
        <button
          className={cls}
          onClick={() => onAction?.(node.action ?? node.label, node.payload)}
        >
          {node.label}
        </button>
      );
    }
    case "input":
      return (
        <input
          className="input"
          name={node.name}
          placeholder={node.placeholder}
          value={formValues[node.name] ?? node.value ?? ""}
          onChange={(e) => setField(node.name, e.target.value)}
        />
      );
    case "metric":
      return (
        <div className="card flex flex-col gap-1 min-w-[120px]">
          <span className="panel-title">{node.label}</span>
          <span className="text-2xl font-semibold tabular-nums text-white">{node.value}</span>
          {node.hint && <span className="text-[11px] text-slate-500">{node.hint}</span>}
        </div>
      );
    case "card":
      return (
        <div className="card gen-enter space-y-3">
          {node.title && <div className="text-sm font-medium text-slate-200">{node.title}</div>}
          {node.children.map((c, i) => (
            <GenRenderer key={i} node={c} onAction={onAction} />
          ))}
        </div>
      );
    case "list":
      return (
        <ul className="space-y-1.5 text-sm">
          {node.items.map((item, i) => (
            <li key={i} className="flex gap-2 text-slate-300">
              <span className="text-blue-400">•</span>
              <span>{item}</span>
            </li>
          ))}
        </ul>
      );
    case "progress":
      return (
        <div className="space-y-1">
          {node.label && <div className="text-xs text-slate-400">{node.label}</div>}
          <div className="h-2 rounded-full bg-slate-800 overflow-hidden">
            <div
              className="h-full rounded-full bg-gradient-to-r from-blue-500 to-indigo-400 transition-all duration-300"
              style={{ width: `${Math.round(Math.min(1, Math.max(0, node.value)) * 100)}%` }}
            />
          </div>
        </div>
      );
    case "divider":
      return <div className="h-px bg-white/10 my-1" />;
    case "html":
      return (
        <div
          className="text-sm prose prose-invert max-w-none"
          dangerouslySetInnerHTML={{ __html: node.html }}
        />
      );
    default:
      return null;
  }
}
