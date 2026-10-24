import { api } from "./api";
import { t } from "./i18n";
import type { GameDto, Shop } from "./types";

const CATEGORIES = [
  "Pistolas", "Micro SMGs", "Escopetas", "Arremesso", "Colete", "SMGs", "Rifles", "Assalto", "Pesadas", "Especiais", "Brancas",
];
const UNLOCKS = [
  "Desde o início",
  "Após Robbing Uncle Sam",
  "Após Just Business",
  "Após House Party",
  "Após Made In Heaven",
  "Após Body Harvest",
  "Após Pier 69",
  "Após Mountain Cloud Boys",
  "Após Yay Ka-Boom-Boom",
  "Após Black Project",
  "Após You've had your Chips",
];

interface PickupProps {
  game: GameDto | null;
  value: string[];
  onChange: (list: string[]) => void;
}

/** Weapon pickups in the world (Create Weapon Pickups mod): one `x y z [ammo]` per line. */
export function PickupFields({ game, value, onChange }: PickupProps) {
  const missing = !game?.pickups;
  if (missing && value.length === 0) return null;
  return (
    <label className="fl">
      {t("Pickups no mapa (x y z [munição], um por linha)")}
      <textarea
        rows={Math.min(6, Math.max(2, value.length + 1))}
        value={value.join("\n")}
        placeholder="2492.03 -1678.56 13.34 20"
        onChange={(e) => onChange(e.target.value.split("\n"))}
        disabled={missing}
      />
      {missing && <span className="meta">{t("Precisa do Create Weapon Pickups instalado.")}</span>}
    </label>
  );
}

interface Props {
  game: GameDto | null;
  reference: string;
  value: Shop | null;
  onChange: (s: Shop | null) => void;
}

/** "Sell in the Ammu-Nation" (Custom Ammu-Nation Mod): a switch plus the few numbers it needs. */
export function ShopFields({ game, reference, value, onChange }: Props) {
  const toggle = async (on: boolean) => {
    if (!on) return onChange(null);
    try {
      onChange(await api.shopDefaults(reference || "PISTOL"));
    } catch {
      onChange({ category: 9, unlock: 0, anim: 0, price: 1000, ammo: 30 });
    }
  };
  const set = (p: Partial<Shop>) => value && onChange({ ...value, ...p });
  const num = (v: string) => Math.max(0, Math.floor(Number(v) || 0));

  return (
    <div className="shop">
      <label>
        <input type="checkbox" checked={value !== null} onChange={(e) => toggle(e.target.checked)} disabled={!game?.shop && value === null} />
        {t("Vender na Ammu-Nation")}
      </label>
      {!game?.shop && value === null && <div className="meta">{t("Precisa do Custom Ammu-Nation Mod instalado.")}</div>}
      {value && (
        <div className="line">
          <label className="fl">
            {t("Categoria")}
            <select value={value.category} onChange={(e) => set({ category: Number(e.target.value) })}>
              {CATEGORIES.map((c, i) => (
                <option key={c} value={i}>
                  {t(c)}
                </option>
              ))}
            </select>
          </label>
          <label className="fl">
            {t("Desbloqueio")}
            <select value={value.unlock} onChange={(e) => set({ unlock: Number(e.target.value) })}>
              {UNLOCKS.map((c, i) => (
                <option key={c} value={i}>
                  {t(c)}
                </option>
              ))}
            </select>
          </label>
          <label className="fl">
            {t("Preço")}
            <input type="number" min={0} value={value.price} onChange={(e) => set({ price: num(e.target.value) })} style={{ width: 90 }} />
          </label>
          <label className="fl">
            {t("Munição")}
            <input type="number" min={0} value={value.ammo} onChange={(e) => set({ ammo: num(e.target.value) })} style={{ width: 80 }} />
          </label>
        </div>
      )}
    </div>
  );
}
