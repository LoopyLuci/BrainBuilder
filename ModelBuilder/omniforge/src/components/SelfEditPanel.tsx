import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";

interface FileEntry {
  path: string;
  size: number;
}

export default function SelfEditPanel() {
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [original, setOriginal] = useState("");
  const [status, setStatus] = useState<string>("");
  const [dirty, setDirty] = useState(false);
  const [flash, setFlash] = useState(false);
  const [saving, setSaving] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const list = await invoke<FileEntry[]>("list_editable_sources");
      setFiles(list);
    } catch (e: any) {
      setStatus(`List error: ${e}`);
    }
  }, []);

  useEffect(() => {
    refresh();
    const u = listen<{ path: string }>("source-reloaded", (e) => {
      setStatus(`Hot reload: ${e.payload.path}`);
      setFlash(true);
      setTimeout(() => setFlash(false), 900);
      if (e.payload.path === selected) {
        invoke<string>("read_source_file", { path: e.payload.path })
          .then((c) => {
            setContent(c);
            setOriginal(c);
            setDirty(false);
          })
          .catch(() => {});
      }
    });
    return () => {
      u.then((f) => f());
    };
  }, [refresh, selected]);

  const open = async (path: string) => {
    setSelected(path);
    setStatus("Loading…");
    try {
      const c = await invoke<string>("read_source_file", { path });
      setContent(c);
      setOriginal(c);
      setDirty(false);
      setStatus("");
    } catch (e: any) {
      setStatus(`Read error: ${e}`);
    }
  };

  const save = async () => {
    if (!selected || !dirty) return;
    setSaving(true);
    setStatus("Atomic write…");
    try {
      await invoke("atomic_write_source", {
        path: selected,
        content,
        triggerReload: true,
      });
      setOriginal(content);
      setDirty(false);
      setStatus("Saved · hot reload signaled");
      setFlash(true);
      setTimeout(() => setFlash(false), 900);
    } catch (e: any) {
      setStatus(`Save failed: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const onChange = (v: string) => {
    setContent(v);
    setDirty(v !== original);
  };

  // Ctrl/Cmd+S
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "s") {
        e.preventDefault();
        save();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });

  return (
    <div className={`h-full flex ${flash ? "hot-reload-flash" : ""}`}>
      {/* File tree */}
      <aside className="w-56 border-r border-white/5 flex flex-col bg-black/20">
        <div className="px-3 py-2 border-b border-white/5 flex items-center justify-between">
          <span className="panel-title">Sources</span>
          <button className="btn-ghost text-[11px] px-2 py-0.5" onClick={refresh}>
            Refresh
          </button>
        </div>
        <div className="flex-1 overflow-y-auto p-2 space-y-0.5">
          {files.map((f) => (
            <button
              key={f.path}
              onClick={() => open(f.path)}
              className={`w-full text-left px-2 py-1.5 rounded-md text-xs truncate transition-colors ${
                selected === f.path
                  ? "bg-blue-600/30 text-blue-100"
                  : "text-slate-400 hover:bg-white/5 hover:text-slate-200"
              }`}
              title={f.path}
            >
              {f.path.replace(/^src\//, "")}
            </button>
          ))}
          {!files.length && (
            <p className="text-[11px] text-slate-600 px-2 py-4">
              No editable sources found. Run from the project root in dev mode.
            </p>
          )}
        </div>
      </aside>

      {/* Editor */}
      <div className="flex-1 flex flex-col min-w-0">
        <div className="px-3 py-2 border-b border-white/5 flex items-center justify-between gap-3">
          <div className="min-w-0">
            <div className="text-sm font-medium truncate">
              {selected ?? "Select a file"}
              {dirty && <span className="ml-2 text-amber-400 text-xs">● unsaved</span>}
            </div>
            <div className="text-[11px] text-slate-500">
              Atomic write → temp + rename · Vite HMR in dev
            </div>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <span className="text-[11px] text-slate-500 max-w-[220px] truncate">{status}</span>
            <button
              className="btn-primary text-xs"
              disabled={!dirty || saving || !selected}
              onClick={save}
            >
              {saving ? "Saving…" : "Save"}
            </button>
          </div>
        </div>
        <textarea
          className="code-editor flex-1 w-full bg-transparent text-slate-200 p-4 outline-none resize-none"
          spellCheck={false}
          value={content}
          onChange={(e) => onChange(e.target.value)}
          placeholder="// Select a source file to edit OmniForge itself"
          disabled={!selected}
        />
      </div>
    </div>
  );
}
