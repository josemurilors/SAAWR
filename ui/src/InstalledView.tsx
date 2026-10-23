import { useMemo, useState } from "react";
import { Copy, KIND_LABEL, KindIcon, Trash, kindOf, type Kind } from "./Icons";
import type { ModDto } from "./types";

interface Props {
  mods: ModDto[];
  onToggle: (m: ModDto) => void;
  onRemove: (m: ModDto) => void;
  onCopy: (text: string) => void;
}

const KINDS: Kind[] = ["car", "bike", "boat", "plane", "weapon"];

export function InstalledView({ mods, onToggle, onRemove, onCopy }: Props) {
  const [text, setText] = useState("");
  const [kind, setKind] = useState<Kind | null>(null);
  const [confirm, setConfirm] = useState<string | null>(null);

  const shown = useMemo(() => {
    const t = text.trim().toLowerCase();
    return mods.filter((m) => {
      const k = kindOf(m.vehicleType, m.kind === "weapon");
      const kindOk = !kind || (!m.foreign && k === kind);
      return kindOk && (!t || m.name.toLowerCase().includes(t));
    });
  }, [mods, text, kind]);

  return (
    <div className="page">
      <h2>Instalados</h2>
      <div className="sub">Ligue, desligue ou remova o que o SA-AWR instalou. Mods de terceiros aparecem esmaecidos e não são alterados.</div>
      <div className="toolbar">
        <input className="field" placeholder="filtrar por nome" value={text} onChange={(e) => setText(e.target.value)} />
        <div className="seg">
          <button className={kind === null ? "on" : ""} onClick={() => setKind(null)}>
            Todos
          </button>
          {KINDS.map((k) => (
            <button key={k} className={kind === k ? "on" : ""} onClick={() => setKind(k)}>
              {KIND_LABEL[k]}
            </button>
          ))}
        </div>
      </div>

      {shown.length === 0 ? (
        <div className="empty">Nenhum mod para mostrar.</div>
      ) : (
        <div className="grid">
          {shown.map((m) => (
            <ModCard key={m.key} m={m} confirming={confirm === m.key} setConfirm={setConfirm} {...{ onToggle, onRemove, onCopy }} />
          ))}
        </div>
      )}
    </div>
  );
}

interface CardProps extends Pick<Props, "onToggle" | "onRemove" | "onCopy"> {
  m: ModDto;
  confirming: boolean;
  setConfirm: (key: string | null) => void;
}

function ModCard({ m, confirming, setConfirm, onToggle, onRemove, onCopy }: CardProps) {
  if (m.foreign) {
    return (
      <div className="card off">
        <div className="glyph dim">
          <KindIcon kind="car" />
        </div>
        <div className="body">
          <div className="title">{m.name}</div>
          <div className="meta">mod de terceiro · não é alterado pela ferramenta</div>
        </div>
      </div>
    );
  }
  const weapon = m.kind === "weapon";
  return (
    <div className={`card ${m.enabled ? "" : "off"}`}>
      <div className={`glyph ${m.enabled ? "" : "dim"}`}>
        <KindIcon kind={kindOf(m.vehicleType, weapon)} />
      </div>
      <div className="body">
        <div className="line">
          <span className="title">{m.name}</span>
          <span className="idtag">{m.id}</span>
          <div className="spacer" />
          <label className="switch" title={m.enabled ? "Desligar" : "Ligar"}>
            <input type="checkbox" checked={m.enabled} onChange={() => onToggle(m)} />
            <span />
          </label>
        </div>
        <div className="meta">referência: {weapon ? m.reference : m.reference.toLowerCase()}</div>
        {weapon ? (
          <div className="line">
            <span className="weapon-type" title="É este número que vai em trainers e scripts, não o ID do modelo">
              tipo da arma {m.weaponTypeId}
              <button className="btn ghost small" style={{ padding: "0 4px", border: 0 }} onClick={() => onCopy(String(m.weaponTypeId))} title="Copiar">
                <Copy />
              </button>
            </span>
            <span className="meta">use o tipo, não o ID do modelo ({m.id}), em trainers</span>
          </div>
        ) : (
          <div className="pills">
            {m.parts > 0 && <span className="chip gold">tuning ({m.parts})</span>}
            {m.paintjobs > 0 && <span className="chip gold">paintjobs ({m.paintjobs})</span>}
            {m.ivfc && <span className="chip gold">cores IVF</span>}
            {m.packHandling && <span className="chip gold">handling do pacote</span>}
            <span className="chip">tráfego: {m.traffic}</span>
          </div>
        )}
        <div className="line">
          {confirming ? (
            <>
              <button className="btn danger small" onClick={() => { setConfirm(null); onRemove(m); }}>
                Confirmar exclusão
              </button>
              <button className="btn ghost small" onClick={() => setConfirm(null)}>
                Cancelar
              </button>
            </>
          ) : (
            <button className="btn ghost small" onClick={() => setConfirm(m.key)}>
              <Trash /> Excluir
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
