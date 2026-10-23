import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api, errorText, inTauri } from "./api";
import { ImportView } from "./ImportView";
import { InstalledView } from "./InstalledView";
import { TopBar, type Tab } from "./TopBar";
import type { ItemDto, ModDto, QItem, Snapshot, TrafficChoice } from "./types";

interface Toast { id: number; text: string; bad: boolean }

const toQueueItem = (i: ItemDto): QItem => ({
  ...i,
  shortEdited: false,
  special: "none",
  include: true,
  traffic: { kind: "auto" },
  audio: true,
  useHandling: true,
  useCleo: false,
});

export function App() {
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [tab, setTab] = useState<Tab>("import");
  const [queue, setQueue] = useState<QItem[]>([]);
  const [problems, setProblems] = useState<Record<number, string | null>>({});
  const [busy, setBusy] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [log, setLog] = useState<string[]>([]);
  const [showLog, setShowLog] = useState(false);
  const toastId = useRef(0);

  const say = useCallback((text: string, bad = false) => {
    setLog((l) => [...l.slice(-199), `${new Date().toLocaleTimeString()}  ${text}`]);
    const id = ++toastId.current;
    setToasts((t) => [...t, { id, text, bad }]);
    // errors stay a little longer
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), bad ? 9000 : 4500);
  }, []);

  const refresh = useCallback(async () => setSnap(await api.snapshot()), []);

  // the saved install opens on startup
  useEffect(() => {
    api.startup().then(setSnap).catch((e) => say(errorText(e), true));
  }, [say]);

  const addPaths = useCallback(
    async (paths: string[]) => {
      if (!paths.length) return;
      try {
        const items = await api.scanPaths(paths);
        if (!items.length) return say("Nada reconhecido: precisa de uma pasta com .dff e .txd.", true);
        setQueue((q) => [...q, ...items.map(toQueueItem)]);
        setTab("import");
        say(`${items.length} item(ns) na fila`);
      } catch (e) {
        say(errorText(e), true);
      }
    },
    [say],
  );

  // drag and drop of files/folders onto the window (Tauri hands over real paths)
  const addPathsRef = useRef(addPaths);
  addPathsRef.current = addPaths;
  useEffect(() => {
    if (!inTauri) return;
    let off: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((e) => {
        const t = e.payload.type;
        if (t === "enter" || t === "over") setDragging(true);
        else if (t === "leave") setDragging(false);
        else if (t === "drop") {
          setDragging(false);
          addPathsRef.current(e.payload.paths);
        }
      })
      .then((fn) => (off = fn));
    return () => off?.();
  }, []);

  // what blocks each queued item, checked on the Rust side
  useEffect(() => {
    if (!queue.length) return setProblems({});
    const h = setTimeout(() => {
      api
        .validate(queue.filter((i) => i.include).map((i) => ({ id: i.id, short: i.short, weapon: i.weapon, reference: i.reference })))
        .then((r) => setProblems(Object.fromEntries(r.map((p) => [p.id, p.problem]))))
        .catch(() => {});
    }, 200);
    return () => clearTimeout(h);
  }, [queue, snap]);

  const patch = (id: number, p: Partial<QItem>) => setQueue((q) => q.map((i) => (i.id === id ? { ...i, ...p } : i)));
  // changing the in-game name suggests a new short, until the user types one himself
  const rename = (id: number, name: string) => {
    patch(id, { name });
    const item = queue.find((i) => i.id === id);
    if (!item || item.shortEdited) return;
    api
      .suggestShort(id, name, item.weapon)
      .then((short) => short && setQueue((q) => q.map((i) => (i.id === id && !i.shortEdited ? { ...i, short } : i))))
      .catch(() => {});
  };
  const removeItem = (id: number) => {
    api.forgetItems([id]);
    setQueue((q) => q.filter((i) => i.id !== id));
  };
  const clearQueue = () => {
    api.forgetItems(queue.map((i) => i.id));
    setQueue([]);
  };

  const openGame = async (path: string) => {
    try {
      setSnap(await api.openGame(path));
      say("GTA SA aberto");
    } catch (e) {
      say(errorText(e), true);
    }
  };

  const inject = async () => {
    const todo = queue.filter((i) => i.include && !problems[i.id]);
    setBusy(true);
    try {
      const results = await api.inject(
        todo.map((i) => ({
          id: i.id, name: i.name, short: i.short, reference: i.reference, weapon: i.weapon,
          traffic: i.traffic, audio: i.audio, useHandling: i.useHandling, useCleo: i.useCleo,
          special: i.weapon && i.special !== "none" ? i.special : null,
        })),
      );
      const ok = results.filter((r) => r.ok).map((r) => r.id);
      setQueue((q) => q.filter((i) => !ok.includes(i.id)));
      for (const r of results.filter((r) => !r.ok)) {
        say(`${todo.find((i) => i.id === r.id)?.name ?? r.id}: ${r.message}`, true);
      }
      if (ok.length) say(`${ok.length} injetado(s) no jogo`);
      await refresh();
    } catch (e) {
      say(errorText(e), true);
    } finally {
      setBusy(false);
    }
  };

  const run = async (what: () => Promise<string>, done: string) => {
    try {
      await what();
      say(done);
    } catch (e) {
      say(errorText(e), true);
    }
    await refresh();
  };

  const onEdit = async (m: ModDto, name: string, traffic: TrafficChoice | null, audio: boolean | null) => {
    try {
      await api.editMod(m.key, name, traffic, audio);
      say(`${name}: alterações salvas`);
      await refresh();
      return true;
    } catch (e) {
      say(errorText(e), true);
      return false;
    }
  };
  const onToggle = (m: ModDto) => run(() => api.setEnabled(m.key, !m.enabled), `${m.name}: ${m.enabled ? "desligado" : "ligado"}`);
  const onRemove = (m: ModDto) => run(() => api.remove(m.key), `${m.name} removido`);
  const onRepair = () => run(() => api.repair(), "Instalação reparada");
  const onCopy = (text: string) => {
    navigator.clipboard?.writeText(text);
    say(`Copiado: ${text}`);
  };

  const last = log[log.length - 1] ?? "pronto";
  return (
    <div className="app">
      {!inTauri && <div className="mockbar">pré-visualização no navegador · dados de exemplo</div>}
      <TopBar tab={tab} onTab={setTab} snap={snap} onOpenGame={openGame} onRepair={onRepair} />
      <main className="content" key={tab}>
        {tab === "import" ? (
          <ImportView
            game={snap?.game ?? null}
            queue={queue}
            problems={problems}
            busy={busy}
            dragging={dragging}
            onAddPaths={addPaths}
            onPatch={patch}
            onRename={rename}
            onRemove={removeItem}
            onClear={clearQueue}
            onInject={inject}
            onMockDrop={() => addPaths(["mock"])}
          />
        ) : (
          <InstalledView mods={snap?.mods ?? []} game={snap?.game ?? null} onToggle={onToggle} onRemove={onRemove} onCopy={onCopy} onEdit={onEdit} />
        )}
      </main>
      <footer className="footer">
        <span className={`last ${last.includes("rro") ? "bad" : ""}`}>{last}</span>
        <button className="btn ghost small" onClick={() => setShowLog((s) => !s)}>
          Log
        </button>
      </footer>

      {showLog && (
        <div className="logbox">
          {[...log].reverse().map((l, i) => (
            <div key={i}>{l}</div>
          ))}
          {!log.length && "sem mensagens"}
        </div>
      )}
      {dragging && (
        <div className="drop-overlay">
          <div>Solte para adicionar à fila</div>
        </div>
      )}
      <div className="toasts">
        {toasts.map((t) => (
          <div key={t.id} className={`toast ${t.bad ? "bad" : ""}`}>
            {t.text}
          </div>
        ))}
      </div>
    </div>
  );
}
