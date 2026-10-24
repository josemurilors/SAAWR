import { useEffect, useState } from "react";
import { api } from "./api";
import { t } from "./i18n";

interface PlateProps {
  /** Vehicles show the model ID; weapons show the weapon type (what trainers ask for). */
  value: string | number;
  weapon?: boolean;
  big?: boolean;
  dim?: boolean;
}

/** An ID shown as a California plate: white and blue for vehicles, yellow and black for weapon types. */
export function Plate({ value, weapon, big, dim }: PlateProps) {
  return (
    <span
      className={`plate ${weapon ? "weapon" : ""} ${big ? "big" : ""} ${dim ? "dim" : ""}`}
      title={weapon ? t("Tipo da arma: é este número que vai em trainers e scripts") : t("ID do modelo")}
    >
      <span className="top">{weapon ? t("tipo") : "Los Santos"}</span>
      <span className="num">{value}</span>
    </span>
  );
}

/** How the in-game name will read: the HUD font has no accents and a few reserved symbols. */
export function HudPreview({ name }: { name: string }) {
  const [hud, setHud] = useState(name);
  useEffect(() => {
    const h = setTimeout(() => api.hudName(name).then(setHud).catch(() => {}), 150);
    return () => clearTimeout(h);
  }, [name]);
  const changed = hud !== name.trim().replace(/\s+/g, " ");
  return (
    <div className="meta">
      {t("No jogo aparece: ")}<strong style={{ color: "var(--chalk)", fontWeight: 600 }}>{hud || t("(vazio)")}</strong>
      {changed && t(" · acentos, símbolos ou tamanho foram ajustados para o jogo")}
    </div>
  );
}
