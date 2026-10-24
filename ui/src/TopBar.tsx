import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { inTauri } from "./api";
import { t, type Lang } from "./i18n";
import { Folder, Wrench } from "./Icons";
import type { Snapshot } from "./types";

export type Tab = "import" | "installed";

interface Props {
  tab: Tab;
  onTab: (t: Tab) => void;
  snap: Snapshot | null;
  onOpenGame: (path: string) => void;
  onRepair: () => void;
  onRestore: () => void;
  lang: Lang;
  onLang: (l: Lang) => void;
}

export function TopBar({ tab, onTab, snap, onOpenGame, onRepair, onRestore, lang, onLang }: Props) {
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
    const dir = await open({ directory: true, title: t("Pasta do GTA San Andreas") });
    if (typeof dir === "string") {
      setPath(dir);
      onOpenGame(dir);
    }
  };

  const chip = (label: string, ok: boolean) => (
    <span className={`chip ${ok ? "ok" : "bad"}`}>{label}: {ok ? t("ok") : t("falta")}</span>
  );

  return (
    <header className="topbar" ref={ref}>
      <span className="wordmark" title={t("SA-AWR: adicionar sem substituir")}>
        <b>SA-AWR</b>
      </span>

      <nav className="tabs">
        <button className={`tab ${tab === "import" ? "active" : ""}`} onClick={() => onTab("import")}>
          {t("Importar")}
        </button>
        <button className={`tab ${tab === "installed" ? "active" : ""}`} onClick={() => onTab("installed")}>
          {t("Instalados")}<span className="count">{snap?.mods.length ?? 0}</span>
        </button>
      </nav>
      <div className="spacer" />
      <div className="seg" role="group" aria-label={t("Idioma")}>
        <button className={lang === "pt" ? "on" : ""} onClick={() => onLang("pt")}>
          PT
        </button>
        <button className={lang === "en" ? "on" : ""} onClick={() => onLang("en")}>
          EN
        </button>
      </div>
      <button className="status-btn" onClick={() => setMenu((m) => !m)}>
        <span className={`dot ${level}`} />
        {game ? "GTA SA" : t("Escolher o GTA SA")}
      </button>

      {menu && (
        <div className="popover">
          <h4>{t("Onde está o GTA San Andreas?")}</h4>
          <div className="row">
            <input
              className="field-input"
              value={path}
              placeholder="D:\Games\GTA San Andreas"
              onChange={(e) => setPath(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && onOpenGame(path)}
            />
            <button className="btn" onClick={browse} title={t("Escolher a pasta")}>
              <Folder />
            </button>
            <button className="btn primary" onClick={() => onOpenGame(path)}>
              {t("Abrir")}
            </button>
          </div>
          {game && (
            <>
              <div className="chips">
                {chip("ModLoader", game.modloader)}
                {chip("fastman92", game.fla)}
                {chip("OLA", game.ola)}
              </div>
              {snap!.findings.map((f, i) => (
                <div key={i} className={`finding ${f.severity}`}>
                  {f.severity === "error" ? t("ERRO") : t("aviso")}: {f.msg}
                </div>
              ))}
              <div className="row" style={{ marginTop: 12 }}>
                <button className="btn" onClick={onRepair}>
                  <Wrench /> {t("Reparar instalação")}
                </button>
                <button className="btn" onClick={onRestore}>
                  {t("Restaurar originais")}
                </button>
              </div>
            </>
          )}
        </div>
      )}
    </header>
  );
}
