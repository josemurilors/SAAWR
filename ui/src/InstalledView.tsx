import { useEffect, useMemo, useState } from "react";
import { t } from "./i18n";
import { parseTraffic, trafficValue } from "./ImportView";
import { Copy, FAMILY_LABEL, KindIcon, Trash, WEAPON_CLASS_LABEL, familyOf, kindOf, weaponClass, type Family } from "./Icons";
import { HudPreview, Plate } from "./Plate";
import { ShopFields } from "./ShopFields";
import type { GameDto, ModDto, Shop, TrafficChoice } from "./types";

interface Props {
  mods: ModDto[];
  game: GameDto | null;
  onToggle: (m: ModDto) => void;
  onRemove: (m: ModDto) => void;
  onCopy: (text: string) => void;
  onEdit: (m: ModDto, name: string, traffic: TrafficChoice | null, audio: boolean | null, shop: Shop | null) => Promise<boolean>;
}

/** Stable across switching a mod off (that renames its folder, so `key` changes). */
const identOf = (m: ModDto) => `${m.id}-${m.short}`;

const FAMILIES: Family[] = ["car", "bike", "boat", "plane", "weapon"];

export function InstalledView({ mods, game, onToggle, onRemove, onCopy, onEdit }: Props) {
  const [text, setText] = useState("");
  const [family, setFamily] = useState<Family | null>(null);
  const [selected, setSelected] = useState<string | null>(null);

  const shown = useMemo(() => {
    const t = text.trim().toLowerCase();
    return mods.filter((m) => {
      const f = familyOf(kindOf(m.vehicleType, m.kind === "weapon", m.reference));
      const familyOk = !family || (!m.foreign && f === family);
      return familyOk && (!t || m.name.toLowerCase().includes(t));
    });
  }, [mods, text, family]);

  const current = mods.find((m) => !m.foreign && identOf(m) === selected) ?? null;
  useEffect(() => {
    if (selected && !current) setSelected(null);
  }, [selected, current]);

  return (
    <div className="split">
      <div className="main">
        <div className="page">
          <h2>{t("Instalados")}</h2>
          <div className="sub">{t("Clique em um item para editar. O interruptor liga e desliga sem apagar nada; mods de terceiros ficam esmaecidos e não são alterados.")}</div>
          <div className="toolbar">
            <input className="field-input" placeholder={t("Buscar pelo nome")} value={text} onChange={(e) => setText(e.target.value)} aria-label={t("Buscar pelo nome")} />
            <div className="seg" role="group" aria-label={t("Tipo")}>
              <button className={family === null ? "on" : ""} onClick={() => setFamily(null)}>
                {t("Todos")}
              </button>
              {FAMILIES.map((k) => (
                <button key={k} className={family === k ? "on" : ""} onClick={() => setFamily(k)}>
                  {t(FAMILY_LABEL[k])}
                </button>
              ))}
            </div>
          </div>

          {shown.length === 0 ? (
            <div className="empty">{mods.length ? t("Nada com esse filtro.") : t("Nada instalado ainda. Vá em Importar e solte a pasta de um mod.")}</div>
          ) : (
            <div className="list">
              {shown.map((m) => (
                <Row key={m.key} m={m} selected={identOf(m) === selected} onSelect={() => setSelected(identOf(m))} onToggle={onToggle} />
              ))}
            </div>
          )}
        </div>
      </div>

      {current && !current.foreign && (
        <EditSheet
          key={identOf(current)}
          m={current}
          game={game}
          onClose={() => setSelected(null)}
          onRemove={(m) => {
            setSelected(null);
            onRemove(m);
          }}
          onCopy={onCopy}
          onEdit={onEdit}
        />
      )}
    </div>
  );
}

function Row({ m, selected, onSelect, onToggle }: { m: ModDto; selected: boolean; onSelect: () => void; onToggle: (m: ModDto) => void }) {
  if (m.foreign) {
    return (
      <div className="irow foreign">
        <div className="glyph">
          <KindIcon kind="car" />
        </div>
        <div className="ibody">
          <div className="name">{m.name}</div>
          <div className="meta">{t("mod de terceiro · a ferramenta não mexe")}</div>
        </div>
      </div>
    );
  }
  const weapon = m.kind === "weapon";
  return (
    <div
      className={`irow ${m.enabled ? "" : "off"} ${selected ? "sel" : ""}`}
      onClick={onSelect}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && onSelect()}
    >
      <div className="glyph">
        <KindIcon kind={kindOf(m.vehicleType, weapon, m.reference)} />
      </div>
      <div className="ibody">
        <div className="name">{m.name}</div>
        <div className="pills">
          {weapon ? (
            <span className="chip accent">{t(WEAPON_CLASS_LABEL[weaponClass(m.reference)])}</span>
          ) : (
            <>
              {m.parts > 0 && <span className="chip accent">{t("tuning {n}", { n: m.parts })}</span>}
              {m.paintjobs > 0 && <span className="chip accent">{t("paintjobs {n}", { n: m.paintjobs })}</span>}
              {m.ivfc && <span className="chip accent">{t("cores IVF")}</span>}
              {m.packHandling && <span className="chip accent">{t("handling do pacote")}</span>}
            </>
          )}
          <span className="chip">{t("baseado em {ref}", { ref: weapon ? m.reference : m.reference.toLowerCase() })}</span>
        </div>
      </div>
      <Plate value={weapon ? (m.weaponTypeId ?? "") : (m.id ?? "")} weapon={weapon} dim={!m.enabled} />
      <label className="switch" title={m.enabled ? t("Desligar") : t("Ligar")} onClick={(e) => e.stopPropagation()}>
        <input type="checkbox" checked={m.enabled} onChange={() => onToggle(m)} aria-label={m.enabled ? t("Desligar") : t("Ligar")} />
        <span />
      </label>
    </div>
  );
}

interface SheetProps {
  m: ModDto;
  game: GameDto | null;
  onClose: () => void;
  onRemove: (m: ModDto) => void;
  onCopy: (t: string) => void;
  onEdit: Props["onEdit"];
}

function EditSheet({ m, game, onClose, onRemove, onCopy, onEdit }: SheetProps) {
  const weapon = m.kind === "weapon";
  const [name, setName] = useState(m.name);
  const [traffic, setTraffic] = useState<TrafficChoice>(parseTraffic(m.trafficChoice));
  const [audio, setAudio] = useState(m.audio);
  const [shop, setShop] = useState<Shop | null>(m.shop);
  const [confirm, setConfirm] = useState(false);
  const [saving, setSaving] = useState(false);

  const dirty =
    name !== m.name ||
    (!weapon && (trafficValue(traffic) !== m.trafficChoice || audio !== m.audio)) ||
    (weapon && JSON.stringify(shop) !== JSON.stringify(m.shop));

  const save = async () => {
    setSaving(true);
    await onEdit(m, name, weapon ? null : traffic, weapon ? null : audio, weapon ? shop : null);
    setSaving(false);
  };

  return (
    <aside className="sheet" aria-label={t("Editar {name}", { name: m.name })}>
      <div className="head">
        <Plate value={weapon ? (m.weaponTypeId ?? "") : (m.id ?? "")} weapon={weapon} big dim={!m.enabled} />
        <div className="spacer" />
        <button className="btn quiet small" onClick={onClose} aria-label={t("Fechar")}>
          {t("Fechar")}
        </button>
      </div>
      <div>
        <h3>{m.name}</h3>
        <div className="meta">
          {t("{kind} · baseado em {ref} · arquivo {short}", {
            kind: weapon ? t("arma · {cls}", { cls: t(WEAPON_CLASS_LABEL[weaponClass(m.reference)]) }) : (m.vehicleType ?? t("carro")),
            ref: m.reference,
            short: m.short,
          })}
          {!m.enabled && t(" · desligado")}
        </div>
      </div>

      <label className="fl">
        {weapon ? t("Nome da arma") : t("Nome no jogo")}
        <input type="text" value={name} onChange={(e) => setName(e.target.value)} />
      </label>
      {!weapon && <HudPreview name={name} />}

      {!weapon && (
        <>
          <label className="fl">
            {t("Aparece no tráfego")}
            <select value={trafficValue(traffic)} onChange={(e) => setTraffic(parseTraffic(e.target.value))}>
              <option value="auto">{t("como o veículo de referência")}</option>
              <option value="none">{t("em nenhum grupo")}</option>
              {game?.groups.map((g) => (
                <option key={g.index} value={`group:${g.index}`}>
                  {g.label}
                </option>
              ))}
            </select>
          </label>
          <label className="check">
            <input type="checkbox" checked={audio} onChange={(e) => setAudio(e.target.checked)} />
            {t("Clonar o som do motor")}
          </label>
        </>
      )}

      {weapon && <ShopFields game={game} reference={m.reference} value={shop} onChange={setShop} />}

      {weapon && (
        <div className="note">
          {t("Em trainers e scripts use o tipo")} <strong style={{ color: "var(--chalk)" }}>{m.weaponTypeId}</strong>
          {t(", não o ID do modelo ({id}).", { id: m.id ?? "" })}{" "}
          <button className="btn quiet small" onClick={() => onCopy(String(m.weaponTypeId))}>
            <Copy /> {t("Copiar o tipo")}
          </button>
        </div>
      )}

      <div className="actions">
        <button className="btn primary" disabled={!dirty || saving || !name.trim()} onClick={save}>
          {saving ? t("Salvando…") : t("Salvar alterações")}
        </button>
        <button
          className="btn quiet"
          disabled={!dirty || saving}
          onClick={() => {
            setName(m.name);
            setTraffic(parseTraffic(m.trafficChoice));
            setAudio(m.audio);
            setShop(m.shop);
          }}
        >
          {t("Desfazer")}
        </button>
      </div>
      {!weapon && <div className="note">{t("Peças, paintjobs e o modelo não mudam aqui. Para trocar o modelo, exclua e importe de novo.")}</div>}

      <hr />
      <div className="actions">
        {confirm ? (
          <>
            <button className="btn danger fill" onClick={() => onRemove(m)}>
              {t("Excluir de vez")}
            </button>
            <button className="btn quiet" onClick={() => setConfirm(false)}>
              {t("Manter")}
            </button>
          </>
        ) : (
          <button className="btn danger" onClick={() => setConfirm(true)}>
            <Trash /> {weapon ? t("Excluir a arma") : t("Excluir o veículo")}
          </button>
        )}
      </div>
    </aside>
  );
}
