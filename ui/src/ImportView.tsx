import { open } from "@tauri-apps/plugin-dialog";
import { inTauri } from "./api";
import { t } from "./i18n";
import { Close, Folder, KindIcon, Upload, WEAPON_CLASS_LABEL, kindOf, weaponClass } from "./Icons";
import { HudPreview } from "./Plate";
import { ShopFields } from "./ShopFields";
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
  { value: "jetpack", label: "Jetpack", hint: "Clona o JETPACK (a referência muda sozinha)." },
  { value: "binoculars", label: "Binóculos (experimental)", hint: "Usa NIGHTVISION como base e liga a visão em primeira pessoa. Ainda não foi testado no jogo." },
];

export function trafficValue(t: TrafficChoice): string {
  return t.kind === "group" ? `group:${t.index}` : t.kind;
}
export function parseTraffic(v: string): TrafficChoice {
  if (v.startsWith("group:")) return { kind: "group", index: Number(v.slice(6)) };
  return { kind: v === "none" ? "none" : "auto" };
}

export function ImportView(p: Props) {
  const pickFolders = async () => {
    if (!inTauri) return p.onMockDrop?.();
    p.onAddPaths(asList(await open({ directory: true, multiple: true, title: t("Pasta do mod (ou de vários)") })));
  };
  const pickFiles = async () => {
    if (!inTauri) return p.onMockDrop?.();
    p.onAddPaths(
      asList(await open({ multiple: true, title: ".dff + .txd", filters: [{ name: t("Modelo e textura"), extensions: ["dff", "txd"] }] })),
    );
  };

  const ready = p.queue.filter((i) => i.include && !p.problems[i.id]).length;
  const hasQueue = p.queue.length > 0;

  return (
    <div className="page">
      <h2>{t("Importar")}</h2>
      <div className="sub">{t("Solte a pasta de um mod, uma pasta com vários, ou os arquivos .dff e .txd. O nome vem da pasta e a referência vem do nome do .dff.")}</div>

      <div className={`dropzone ${hasQueue ? "compact" : ""} ${p.dragging ? "over" : ""}`}>
        <Upload className="hero" />
        <div className="big">{hasQueue ? t("Solte mais mods aqui") : t("Arraste a pasta do mod para cá")}</div>
        {hasQueue ? <div className="hint" /> : <div className="hint">{t("Funciona com a pasta de um mod, com uma pasta de vários, ou com .dff + .txd soltos.")}</div>}
        <div className="row" style={{ justifyContent: "center" }}>
          <button className="btn" onClick={pickFolders}>
            <Folder /> {t("Escolher pasta")}
          </button>
          <button className="btn" onClick={pickFiles}>
            {t("Escolher arquivos")}
          </button>
        </div>
      </div>

      {hasQueue && (
        <>
          <div className="listhead">
            <h3>{t("Fila")}</h3>
            <span className="meta">
              {t("{n} item(ns) · {ready} pronto(s) para injetar", { n: p.queue.length, ready })}
            </span>
            <button className="btn quiet small" onClick={p.onClear}>
              {t("Limpar fila")}
            </button>
          </div>
          <div className="queue">
            {p.queue.map((item) => (
              <QueueRow key={item.id} item={item} game={p.game} problem={p.problems[item.id] ?? null} p={p} />
            ))}
          </div>
          <div className="actionbar">
            <button className="btn primary" disabled={ready === 0 || p.busy} onClick={p.onInject}>
              {p.busy ? t("Injetando…") : t("Injetar {n} no jogo", { n: ready })}
            </button>
          </div>
        </>
      )}
    </div>
  );
}

function QueueRow({ item, game, problem, p }: { item: QItem; game: GameDto | null; problem: string | null; p: Props }) {
  const patch = (x: Partial<QItem>) => p.onPatch(item.id, x);
  const kind = kindOf(game?.refs.find((r) => r.model === item.reference)?.kind, item.weapon, item.reference);
  const setSpecial = (special: Special) => {
    const forced = special === "jetpack" ? "JETPACK" : special === "binoculars" ? "NIGHTVISION" : null;
    const ok = forced && game?.weapons.some((w) => w.name === forced);
    patch({ special, ...(ok ? { reference: forced! } : {}) });
  };
  const showProblem = item.include && problem;
  const hint = SPECIALS.find((s) => s.value === item.special)?.hint;

  return (
    <div className={`qrow ${item.include ? "" : "off"} ${showProblem ? "bad" : ""}`}>
      <div className={`glyph ${item.include ? "" : "dim"}`}>
        <KindIcon kind={kind} />
      </div>
      <div className="qbody">
        <div className="line">
          <input type="checkbox" checked={item.include} onChange={(e) => patch({ include: e.target.checked })} aria-label={t("Injetar este item")} style={{ marginBottom: 9 }} />
          <label className="fl">
            {t("Nome no jogo")}
            <input type="text" value={item.name} onChange={(e) => p.onRename(item.id, e.target.value)} style={{ minWidth: 260 }} />
          </label>
          <label className="fl">
            {t("Nome curto")}
            <input
              type="text"
              value={item.short}
              onChange={(e) => {
                const short = e.target.value.toLowerCase();
                patch({ short, shortEdited: short !== "" });
              }}
              style={{ minWidth: 130 }}
            />
          </label>
          <div className="spacer" />
          <button className="btn quiet small" title={t("Tirar da fila")} onClick={() => p.onRemove(item.id)}>
            <Close />
          </button>
        </div>
        <HudPreview name={item.name} />

        <div className="line">
          <div className="seg" role="group" aria-label={t("Tipo")}>
            <button className={!item.weapon ? "on" : ""} onClick={() => item.weapon && patch({ weapon: false, reference: "" })}>
              {t("Veículo")}
            </button>
            <button className={item.weapon ? "on" : ""} onClick={() => item.weapon || patch({ weapon: true, reference: "" })}>
              {t("Arma")}
            </button>
          </div>
          <label className="fl">
            {t("Baseado em")}
            <select value={item.reference} onChange={(e) => patch({ reference: e.target.value })}>
              <option value="">{t("escolha…")}</option>
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
          {item.weapon && item.special === "none" && (
            <label className="fl">
              {t("Som e comportamento iguais a")}
              <select value={item.soundLike} onChange={(e) => patch({ soundLike: e.target.value })}>
                <option value="">{t("a referência")}</option>
                {game?.weapons.map((w) => (
                  <option key={w.name} value={w.name}>
                    {w.name}
                  </option>
                ))}
              </select>
            </label>
          )}
          {item.weapon ? (
            <label className="fl">
              {t("Comportamento especial")}
              <select value={item.special} onChange={(e) => setSpecial(e.target.value as Special)}>
                {SPECIALS.map((s) => (
                  <option key={s.value} value={s.value}>
                    {t(s.label)}
                  </option>
                ))}
              </select>
            </label>
          ) : (
            <label className="fl">
              {t("Aparece no tráfego")}
              <select value={trafficValue(item.traffic)} onChange={(e) => patch({ traffic: parseTraffic(e.target.value) })}>
                <option value="auto">{t("como o veículo de referência")}</option>
                <option value="none">{t("em nenhum grupo")}</option>
                {game?.groups.map((g) => (
                  <option key={g.index} value={`group:${g.index}`}>
                    {g.label}
                  </option>
                ))}
              </select>
            </label>
          )}
        </div>
        {item.weapon && hint && <div className="meta">{t(hint)}</div>}

        <div className="opts">
          {!item.weapon && item.extras.handling && (
            <label>
              <input type="checkbox" checked={item.useHandling} onChange={(e) => patch({ useHandling: e.target.checked })} />
              {t("Usar o handling do pacote")}
            </label>
          )}
          {item.weapon && item.extras.weaponLines && (
            <label>
              <input type="checkbox" checked={item.useHandling} onChange={(e) => patch({ useHandling: e.target.checked })} />
              {t("Usar as linhas de arma do pacote")}
            </label>
          )}
          {item.extras.cleo && (
            <label>
              <input type="checkbox" checked={item.useCleo} onChange={(e) => patch({ useCleo: e.target.checked })} />
              {t("Copiar scripts CLEO")}
            </label>
          )}
          {!item.weapon && (
            <label>
              <input type="checkbox" checked={item.audio} onChange={(e) => patch({ audio: e.target.checked })} />
              {t("Clonar o som do motor")}
            </label>
          )}
        </div>

        {item.weapon && <ShopFields game={game} reference={item.reference} value={item.shop} onChange={(shop) => patch({ shop })} />}

        {showProblem ? (
          <div className="problem">{problem}</div>
        ) : (
          <div className="row" style={{ gap: 6, flexWrap: "wrap" }}>
            {item.weapon ? (
              <>
                <span className="chip accent">{t(WEAPON_CLASS_LABEL[weaponClass(item.reference)])}</span>
                <span className="chip">{t("o ícone do HUD é renomeado sozinho")}</span>
              </>
            ) : (
              <>
                {item.extras.parts && <span className="chip accent">{t("peças de tuning")}</span>}
                {item.extras.paintjobs && <span className="chip accent">{t("paintjobs")}</span>}
                {item.extras.ivfc && <span className="chip accent">{t("cores IVF")}</span>}
                {!item.extras.parts && !item.extras.paintjobs && !item.extras.ivfc && <span className="chip">{t("só modelo e textura")}</span>}
              </>
            )}
            <span className="chip" title={item.folder}>
              {item.folder.split(/[\\/]/).pop()}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
