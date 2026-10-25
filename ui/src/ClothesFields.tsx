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
];

/** Shop sections as `shopping.dat` names them. */
const SHOP_LABEL: Record<string, string> = {
  CSchp: "Binco",
  CSsprt: "ProLaps",
  LACS1: "Sub Urban",
  clothgp: "Zip",
  Csdesgn: "Victim",
  Csexl: "Didier Sachs",
};

export const defaultClothes = (ownModel: boolean): Clothes => ({ bodyPart: 0, model: ownModel ? "" : "tshirt", price: 10, shops: [] });

interface Props {
  value: Clothes;
  onChange: (c: Clothes) => void;
  /** The item brings its own .dff, so there is no original model to pick. */
  ownModel: boolean;
  /** Short of the installed item being edited: its own places count as free, and its body
   * part and model stay as installed. */
  skip?: string;
  /** Queue item whose texture is compared with the CJ in use. */
  queueId?: number;
}

const dims = (d: [number, number] | null) => (d ? `${d[0]}×${d[1]}` : "?");

/** Body part, original model, price and the shops with free places. */
export function ClothesFields({ value, onChange, ownModel, skip, queueId }: Props) {
  const [info, setInfo] = useState<StoreInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [fit, setFit] = useState<TextureFit | null>(null);
  useEffect(() => {
    api.storeInfo(skip ?? null).then(setInfo, (e) => setError(errorText(e)));
  }, [skip]);
  useEffect(() => {
    if (queueId === undefined) return;
    api.textureFit(queueId, value.model, value.bodyPart).then(setFit, () => setFit(null));
  }, [queueId, value.model, value.bodyPart]);
  const grows = fit?.have && fit.want ? fit.want[0] > fit.have[0] : false;
  const set = (p: Partial<Clothes>) => onChange({ ...value, ...p });
  const models = (info?.models ?? []).filter(([, part]) => part === value.bodyPart).map(([m]) => m);
  const setPart = (bodyPart: number) => {
    const first = (info?.models ?? []).find(([, p]) => p === bodyPart)?.[0] ?? "";
    // shops picked for another body part may have no room for this one
    set({ bodyPart, model: ownModel ? "" : first, shops: [] });
  };
  const toggleShop = (section: string, on: boolean) =>
    set({ shops: on ? [...value.shops, section] : value.shops.filter((s) => s !== section) });

  if (error) return <div className="problem">{error}</div>;
  if (info?.blocked) return <div className="problem">{t("Outro mod altera os preços de roupas do shopping.dat; desative-o para adicionar roupas.")}</div>;
  return (
    <div className="shop">
      <div className="line">
        <label className="fl">
          {t("Parte do corpo")}
          <select value={value.bodyPart} onChange={(e) => setPart(Number(e.target.value))} disabled={skip !== undefined}>
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
            <select value={value.model} onChange={(e) => set({ model: e.target.value })} disabled={skip !== undefined}>
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
        {(info?.shops ?? []).map((s) => {
          const free = s.free[value.bodyPart] ?? 0;
          const on = value.shops.includes(s.section);
          const why = s.blocked ? t("outro mod altera esta loja") : free === 0 ? t("cheia") : t("{n} vaga(s)", { n: free });
          return (
            <label key={s.section} title={s.section}>
              <input type="checkbox" checked={on} disabled={!on && (s.blocked || free === 0)} onChange={(e) => toggleShop(s.section, e.target.checked)} />
              {SHOP_LABEL[s.section] ?? s.section} <span className="meta">({why})</span>
            </label>
          );
        })}
      </div>
      {info && info.pricesFree < 5 && <div className="meta">{t("O shopping.dat tem só {n} vaga(s) até o limite do jogo (560 itens).", { n: info.pricesFree })}</div>}
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
      <div className="meta">{t("Cada loja mostra no máximo 12 itens de cada parte do corpo.")}</div>
    </div>
  );
}
