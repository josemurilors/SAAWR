import { useEffect, useState } from "react";
import { api, errorText } from "./api";
import { t } from "./i18n";
import type { Clothes, StoreInfo, TextureFit } from "./types";

export const BODY_PARTS: [number, string][] = [
  [0, "Camisa / torso"],
  [2, "Calça"],
  [3, "Sapato"],
  [13, "Colar"],
  [14, "Relógio"],
  [15, "Óculos"],
  [16, "Chapéu"],
  [1, "Cabelo"],
];

const HAIR = 1;

export const defaultClothes = (ownModel: boolean): Clothes => ({ bodyPart: 0, model: ownModel ? "" : "tshirt", price: 10, shops: [] });

interface Props {
  value: Clothes;
  onChange: (c: Clothes) => void;
  /** The item brings its own .dff, so there is no original model to pick. */
  ownModel: boolean;
  /** An installed item: its body part and model cannot change. */
  locked?: boolean;
  /** Queue item whose texture is compared with the CJ in use. */
  queueId?: number;
  /** The original texture the suggestion came from. */
  suggestion?: string | null;
}

const dims = (d: [number, number] | null) => (d ? `${d[0]}×${d[1]}` : "?");

/** Body part, original model, price and the shops of the catalog that sell it. */
export function ClothesFields({ value, onChange, ownModel, locked, queueId, suggestion }: Props) {
  const [info, setInfo] = useState<StoreInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [fit, setFit] = useState<TextureFit | null>(null);
  useEffect(() => {
    api.storeInfo().then(setInfo, (e) => setError(errorText(e)));
  }, []);
  useEffect(() => {
    if (queueId === undefined) return;
    api.textureFit(queueId, value.model, value.bodyPart).then(setFit, () => setFit(null));
  }, [queueId, value.model, value.bodyPart]);
  const grows = fit?.have && fit.want ? fit.want[0] > fit.have[0] : false;
  const set = (p: Partial<Clothes>) => onChange({ ...value, ...p });
  const models = (info?.models ?? []).filter(([, part]) => part === value.bodyPart).map(([m]) => m);
  const setPart = (bodyPart: number) => {
    const first = (info?.models ?? []).find(([, p]) => p === bodyPart)?.[0] ?? "";
    // the shops picked sell another kind of item
    set({ bodyPart, model: ownModel ? "" : first, shops: [] });
  };
  const toggleShop = (section: string, on: boolean) =>
    set({ shops: on ? [...value.shops, section] : value.shops.filter((s) => s !== section) });
  const kind = value.bodyPart === HAIR ? "haircuts" : "clothes";

  if (error) return <div className="problem">{error}</div>;
  return (
    <div className="shop">
      {info && !info.cleo && (
        <div className="problem">{t("O CLEO (com o plugin IniFiles) não foi encontrado: o catálogo de roupas precisa dele.")}</div>
      )}
      {suggestion && (
        <div className="meta">
          {t("Sugestão a partir da textura original “{from}”: parte do corpo, preço e lojas iguais aos do jogo. Mude o que quiser.", { from: suggestion })}
        </div>
      )}
      <div className="line">
        <label className="fl">
          {t("Parte do corpo")}
          <select value={value.bodyPart} onChange={(e) => setPart(Number(e.target.value))} disabled={locked}>
            {BODY_PARTS.map(([v, label]) => (
              <option key={v} value={v}>
                {t(label)}
              </option>
            ))}
          </select>
        </label>
        {!ownModel && (
          <label className="fl">
            {t("Vestida no modelo")}
            <select value={value.model} onChange={(e) => set({ model: e.target.value })} disabled={locked}>
              {models.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
            </select>
          </label>
        )}
        <label className="fl">
          {t("Preço")}
          <input type="number" min={0} value={value.price} onChange={(e) => set({ price: Math.max(0, Math.floor(Number(e.target.value) || 0)) })} style={{ width: 90 }} />
        </label>
      </div>
      <div className="opts">
        {(info?.shops ?? [])
          .filter((s) => s.kind === kind)
          .map((s) => (
            <label key={s.section} title={s.section}>
              <input type="checkbox" checked={value.shops.includes(s.section)} onChange={(e) => toggleShop(s.section, e.target.checked)} />
              {s.label}
            </label>
          ))}
      </div>
      {value.shops.length === 0 ? (
        <div className="note">{t("Sem loja: o item fica de graça no guarda-roupa (digite ROUPA no jogo).")}</div>
      ) : (
        <div className="meta">{t("Dentro da loja, aperte N ou digite LOJA para abrir o catálogo. Sem limite de itens por loja.")}</div>
      )}
      {fit?.action === "resize" && (
        <div className="note">
          {t(grows ? "A textura tem {have}; o CJ instalado usa {want}. Ela será ampliada ao injetar (o arquivo original não muda)." : "A textura tem {have}; o CJ instalado usa {want}. Ela será reduzida ao injetar (o arquivo original não muda).", { have: dims(fit.have), want: dims(fit.want) })}
        </div>
      )}
      {fit?.action === "mismatch" && (
        <div className="problem">
          {t("A textura tem {have}, mas o CJ instalado usa {want}, e ela não pode ser convertida (comprimida ou de outra proporção). No jogo, a roupa pode aparecer como pele.", { have: dims(fit.have), want: dims(fit.want) })}
        </div>
      )}
    </div>
  );
}
