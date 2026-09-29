/** Declarative UI tree the Concierge (or user) can emit at runtime. */
export type GenNode =
  | { type: "stack"; direction?: "row" | "col"; gap?: number; children: GenNode[]; className?: string }
  | { type: "text"; content: string; variant?: "title" | "body" | "muted" | "code" }
  | { type: "button"; label: string; action?: string; variant?: "primary" | "ghost" | "danger"; payload?: unknown }
  | { type: "input"; name: string; placeholder?: string; value?: string }
  | { type: "metric"; label: string; value: string | number; hint?: string }
  | { type: "card"; title?: string; children: GenNode[] }
  | { type: "list"; items: string[] }
  | { type: "progress"; value: number; label?: string }
  | { type: "divider" }
  | { type: "html"; html: string };

export interface GenSurface {
  id: string;
  title: string;
  createdAt: number;
  root: GenNode;
}
