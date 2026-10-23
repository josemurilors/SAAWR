import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { inTauri } from "./api";
import { Folder, Mark, Wrench } from "./Icons";
import type { Snapshot } from "./types";

export type Tab = "import" | "installed";

interface Props {
  tab: Tab;
  onTab: (t: Tab) => void;
  snap: Snapshot | null;
  onOpenGame: (path: string) => void;
  onRepair: () => void;
}

export function TopBar({ tab, onTab, snap, onOpenGame, onRepair }: Props) {
  const [menu, setMenu] = useState(false);
  const [path, setPath] = useState(snap?.gtaPath ?? "");
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => setPath(snap?.gtaPath ?? ""), [snap?.gtaPath]);
  useEffect(() => {
    if (!menu) return;
    const close = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setMenu(false);
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, [menu]);

  const game = snap?.game ?? null;
  const hasError = snap?.findings.some((f) => f.severity === "error") ?? false;
  const level = !game || hasError ? "bad" : snap!.findings.length ? "warn" : "";

  const browse = async () => {
    if (!inTauri) return;
    const dir = await open({ directory: true, title: "Pasta do GTA San Andreas" });
    if (typeof dir === "string") {
      setPath(dir);
      onOpenGame(dir);
    }
  };

  const chip = (label: string, ok: boolean) => (
    <span className={`chip ${ok ? "ok" : "bad"}`}>{label}: {ok ? "ok" : "falta"}</span>
  );

  return (
    <header className="topbar" ref={ref}>
      <div className="brand">
        <Mark />
        SA-AWR
      </div>
      <nav className="tabs">
        <button className={`tab ${tab === "import" ? "active" : ""}`} onClick={() => onTab("import")}>
          Importar
        </button>
        <button className={`tab ${tab === "installed" ? "active" : ""}`} onClick={() => onTab("installed")}>
          Instalados ({snap?.mods.length ?? 0})
        </button>
      </nav>
      <div className="spacer" />
      <button className="status-btn" onClick={() => setMenu((m) => !m)}>
        <span className={`dot ${level}`} />
        {game ? "GTA SA" : "Abrir o GTA SA"}
      </button>

      {menu && (
        <div className="popover">
          <h4>Pasta do jogo</h4>
          <div className="row">
            <input
              className="field"
              value={path}
              placeholder="D:\Games\GTA San Andreas"
              onChange={(e) => setPath(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && onOpenGame(path)}
            />
            <button className="btn" onClick={browse} title="Procurar">
              <Folder />
            </button>
            <button className="btn primary" onClick={() => onOpenGame(path)}>
              Abrir
            </button>
          </div>
          {game && (
            <>
              <div className="check-list">
                {chip("ModLoader", game.modloader)}
                {chip("fastman92", game.fla)}
                {chip("OLA", game.ola)}
              </div>
              {snap!.findings.map((f, i) => (
                <div key={i} className={`finding ${f.severity}`}>
                  {f.severity === "error" ? "ERRO" : "aviso"}: {f.msg}
                </div>
              ))}
              <div className="row" style={{ marginTop: 12 }}>
                <button className="btn" onClick={onRepair}>
                  <Wrench /> Reparar instalação
                </button>
              </div>
            </>
          )}
        </div>
      )}
    </header>
  );
}
