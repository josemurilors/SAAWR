import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, inTauri } from "./api";
import { Close, Folder, KindIcon, Upload, WEAPON_CLASS_LABEL, kindOf, weaponClass } from "./Icons";
import type { GameDto, QItem, Special, TrafficChoice } from "./types";

interface Props {
  game: GameDto | null;
  queue: QItem[];
  problems: Record<number, string | null>;
  busy: boolean;
  dragging: boolean;
  onAddPaths: (paths: string[]) => void;
  onPatch: (id: number, patch: Partial<QItem>) => void;
  onRename: (id: number, name: string) => void;
  onRemove: (id: number) => void;
  onClear: () => void;
  onInject: () => void;
  onMockDrop?: () => void;
}

const asList = (r: string | string[] | null): string[] => (r === null ? [] : Array.isArray(r) ? r : [r]);

const SPECIALS: { value: Special; label: string; hint: string }[] = [
  { value: "none", label: "Normal", hint: "" },
  { value: "silencer", label: "Pistola com silenciador", hint: "Herda o comportamento da PISTOL_SILENCED." },
  { value: "jetpack", label: "Jetpack", hint: "Clona o JETPACK (referência trocada sozinha)." },
  { value: "binoculars", label: "Binóculos (experimental)", hint: "Usa NIGHTVISION como base e liga a visão em primeira pessoa. Ainda não testado no jogo." },
];

/** How the in-game name will read: the HUD font has no accents and a few reserved symbols. */
function HudPreview({ name }: { name: string }) {
  const [hud, setHud] = useState(name);
  useEffect(() => {
    const h = setTimeout(() => api.hudName(name).then(setHud).catch(() => {}), 150);
    return () => clearTimeout(h);
  }, [name]);
  const changed = hud !== name.trim().replace(/\s+/g, " ");
  return (
    <div className="meta">
      No jogo aparece: <strong style={{ color: "var(--text)" }}>{hud || "(vazio)"}</strong>
      {changed && " · acentos, símbolos ou tamanho foram ajustados para o jogo"}
    </div>
  );
}

function trafficValue(t: TrafficChoice): string {
  return t.kind === "group" ? `group:${t.index}` : t.kind;
}
function parseTraffic(v: string): TrafficChoice {
  if (v.startsWith("group:")) return { kind: "group", index: Number(v.slice(6)) };
  return { kind: v === "none" ? "none" : "auto" };
}

export function ImportView(p: Props) {
  const pickFolders = async () => {
    if (!inTauri) return p.onMockDrop?.();
    p.onAddPaths(asList(await open({ directory: true, multiple: true, title: "Pasta do mod (ou de vários)" })));
  };
  const pickFiles = async () => {
    if (!inTauri) return p.onMockDrop?.();
    p.onAddPaths(
      asList(await open({ multiple: true, title: ".dff + .txd", filters: [{ name: "Modelo e textura", extensions: ["dff", "txd"] }] })),
    );
  };

  const ready = p.queue.filter((i) => i.include && !p.problems[i.id]).length;

  return (
    <div className="page">
      <h2>Importar</h2>
      <div className="sub">Solte a pasta de um mod, uma pasta com vários, ou os arquivos .dff e .txd.</div>

      <div className={`dropzone ${p.dragging ? "over" : ""}`}>
        <Upload className="hero" />
        <div className="big">Arraste aqui</div>
        <div className="hint">O nome vem da pasta e a referência vem do nome do .dff.</div>
        <div className="row" style={{ justifyContent: "center" }}>
          <button className="btn" onClick={pickFolders}>
            <Folder /> Procurar pasta
          </button>
          <button className="btn" onClick={pickFiles}>
            Procurar arquivos
          </button>
        </div>
      </div>

      {p.queue.length > 0 && (
        <>
          <div className="row" style={{ margin: "20px 0 -4px", justifyContent: "space-between" }}>
            <strong>
              Fila ({p.queue.length}) <span className="meta" style={{ fontWeight: 400, color: "var(--muted)" }}>· {ready} pronto(s)</span>
            </strong>
            <button className="btn ghost small" onClick={p.onClear}>
              Limpar fila
            </button>
          </div>
          <div className="queue">
            {p.queue.map((item) => (
              <QueueCard key={item.id} item={item} game={p.game} problem={p.problems[item.id] ?? null} p={p} />
            ))}
          </div>
          <div className="actionbar">
            <button className="btn primary" disabled={ready === 0 || p.busy} onClick={p.onInject}>
              {p.busy ? "Injetando…" : `INJETAR ${ready} NO JOGO`}
            </button>
          </div>
        </>
      )}
    </div>
  );
}

function QueueCard({ item, game, problem, p }: { item: QItem; game: GameDto | null; problem: string | null; p: Props }) {
  const patch = (x: Partial<QItem>) => p.onPatch(item.id, x);
  const kind = kindOf(game?.refs.find((r) => r.model === item.reference)?.kind, item.weapon, item.reference);
  const setSpecial = (special: Special) => {
    const forced = special === "jetpack" ? "JETPACK" : special === "binoculars" ? "NIGHTVISION" : null;
    const ok = forced && game?.weapons.some((w) => w.name === forced);
    patch({ special, ...(ok ? { reference: forced! } : {}) });
  };
  const showProblem = item.include && problem;

  return (
    <div className={`card ${item.include ? "" : "off"} ${showProblem ? "bad" : ""}`}>
      <div className={`glyph ${item.include ? "" : "dim"}`}>
        <KindIcon kind={kind} />
      </div>
      <div className="body">
        <div className="line">
          <input type="checkbox" checked={item.include} onChange={(e) => patch({ include: e.target.checked })} />
          <label className="fl">
            Nome
            <input type="text" value={item.name} onChange={(e) => p.onRename(item.id, e.target.value)} style={{ minWidth: 230 }} />
          </label>
          <label className="fl">
            Nome curto
            <input
              type="text"
              value={item.short}
              onChange={(e) => {
                const short = e.target.value.toLowerCase();
                patch({ short, shortEdited: short !== "" });
              }}
              style={{ minWidth: 120 }}
            />
          </label>
          <div className="spacer" />
          <button className="btn ghost small" title="Tirar da fila" onClick={() => p.onRemove(item.id)}>
            <Close />
          </button>
        </div>

        <HudPreview name={item.name} />

        <div className="line">
          <div className="seg">
            <button className={!item.weapon ? "on" : ""} onClick={() => item.weapon && patch({ weapon: false, reference: "" })}>
              Veículo
            </button>
            <button className={item.weapon ? "on" : ""} onClick={() => item.weapon || patch({ weapon: true, reference: "" })}>
              Arma
            </button>
          </div>
          <label className="fl">
            Referência
            <select value={item.reference} onChange={(e) => patch({ reference: e.target.value })}>
              <option value="">escolha…</option>
              {item.weapon
                ? game?.weapons.map((w) => (
                    <option key={w.name} value={w.name}>
                      {w.name}
                    </option>
                  ))
                : game?.refs.map((r) => (
                    <option key={r.model} value={r.model}>
                      {r.model} ({r.kind})
                    </option>
                  ))}
            </select>
          </label>
          {item.weapon && (
            <label className="fl">
              Tipo especial
              <select value={item.special} onChange={(e) => setSpecial(e.target.value as Special)}>
                {SPECIALS.map((s) => (
                  <option key={s.value} value={s.value}>
                    {s.label}
                  </option>
                ))}
              </select>
            </label>
          )}
          {!item.weapon && (
            <label className="fl">
              Tráfego
              <select value={trafficValue(item.traffic)} onChange={(e) => patch({ traffic: parseTraffic(e.target.value) })}>
                <option value="auto">como a referência</option>
                <option value="none">nenhum</option>
                {game?.groups.map((g) => (
                  <option key={g.index} value={`group:${g.index}`}>
                    {g.label}
                  </option>
                ))}
              </select>
            </label>
          )}
        </div>

        {item.weapon && SPECIALS.find((s) => s.value === item.special)?.hint && (
          <div className="meta">{SPECIALS.find((s) => s.value === item.special)?.hint}</div>
        )}

        <div className="opts">
          {!item.weapon && item.extras.handling && (
            <label>
              <input type="checkbox" checked={item.useHandling} onChange={(e) => patch({ useHandling: e.target.checked })} />
              Usar o handling do pacote
            </label>
          )}
          {item.weapon && item.extras.weaponLines && (
            <label>
              <input type="checkbox" checked={item.useHandling} onChange={(e) => patch({ useHandling: e.target.checked })} />
              Usar as linhas de arma do pacote
            </label>
          )}
          {!item.weapon && item.extras.cleo && (
            <label>
              <input type="checkbox" checked={item.useCleo} onChange={(e) => patch({ useCleo: e.target.checked })} />
              Copiar scripts CLEO
            </label>
          )}
          {!item.weapon && (
            <label>
              <input type="checkbox" checked={item.audio} onChange={(e) => patch({ audio: e.target.checked })} />
              Clonar o áudio
            </label>
          )}
        </div>

        {showProblem ? (
          <div className="problem">{problem}</div>
        ) : (
          <div className="pills">
            {item.weapon ? (
              <>
                <span className="chip gold">{WEAPON_CLASS_LABEL[weaponClass(item.reference)]}</span>
                <span className="chip">o ícone do HUD é renomeado sozinho</span>
              </>
            ) : (
              <>
                {item.extras.parts && <span className="chip gold">tuning</span>}
                {item.extras.paintjobs && <span className="chip gold">paintjobs</span>}
                {item.extras.ivfc && <span className="chip gold">cores IVF</span>}
                {!item.extras.parts && !item.extras.paintjobs && !item.extras.ivfc && <span className="chip">só modelo e textura</span>}
              </>
            )}
            <span className="chip" title={item.folder}>{item.folder.split(/[\\/]/).pop()}</span>
          </div>
        )}
      </div>
    </div>
  );
}
