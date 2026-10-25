export type Severity = "warn" | "error";
export interface Finding { severity: Severity; msg: string }
/** How a weapon is sold by the Custom Ammu-Nation Mod. */
export interface Shop { category: number; unlock: number; anim: number; price: number; ammo: number }
/** How a clothing item is sold: body part and shop sections as in `shopping.dat`. */
export interface Clothes {
  bodyPart: number;
  /** Original model the texture is worn on; "" with the item's own .dff. */
  model: string;
  price: number;
  shops: string[];
}
export interface ShopSlots {
  section: string;
  kind: "clothes" | "haircuts";
  /** Free places per body part. */
  free: Record<number, number>;
  /** Another mod changes this shop: AWR leaves it alone. */
  blocked: boolean;
}
export interface StoreInfo {
  shops: ShopSlots[];
  /** [model, body part] of the original clothing models. */
  models: [string, number][];
  pricesFree: number;
  /** Price sections ("Clothes", "Haircuts") another mod changes: nothing of those can be added. */
  blocked: string[];
}
/** A queued clothing texture against the clothes of the CJ in use. */
export interface TextureFit {
  have: [number, number] | null;
  action: "keep" | "resize" | "mismatch";
  want: [number, number] | null;
}
export interface RefDto { model: string; kind: string }
export interface WeaponDto { name: string; melee: boolean }
export interface GroupDto { index: number; label: string; len: number }
export interface GameDto {
  root: string;
  modloader: boolean;
  fla: boolean;
  ola: boolean;
  /** The Custom Ammu-Nation Mod is installed. */
  shop: boolean;
  /** The Create Weapon Pickups mod is installed. */
  pickups: boolean;
  refs: RefDto[];
  weapons: WeaponDto[];
  groups: GroupDto[];
}
export interface ModDto {
  key: string;
  id: number | null;
  enabled: boolean;
  foreign: boolean;
  kind: "vehicle" | "weapon" | "clothes" | null;
  name: string;
  short: string;
  reference: string;
  vehicleType: string | null;
  weaponTypeId: number | null;
  parts: number;
  paintjobs: number;
  ivfc: boolean;
  packHandling: boolean;
  traffic: string;
  /** The traffic setting for the editor: "auto", "none" or "group:N". */
  trafficChoice: string;
  audio: boolean;
  shop: Shop | null;
  pickups: string[];
  clothes: Clothes | null;
}
export interface Snapshot {
  gtaPath: string;
  game: GameDto | null;
  mods: ModDto[];
  findings: Finding[];
}
export interface Extras {
  parts: boolean;
  paintjobs: boolean;
  ivfc: boolean;
  handling: boolean;
  weaponLines: boolean;
  cleo: boolean;
}
export interface ItemDto {
  id: number;
  name: string;
  short: string;
  reference: string;
  weapon: boolean;
  /** Only a .txd: it can only become clothing. */
  textureOnly: boolean;
  folder: string;
  extras: Extras;
}
export type TrafficKind = "auto" | "none" | "group";
export interface TrafficChoice { kind: TrafficKind; index?: number }
/** A queue item with what the user can change on it. */
export type Special = "none" | "silencer" | "jetpack" | "binoculars";
export interface QItem extends ItemDto {
  /** The user typed the short himself: stop suggesting it from the name. */
  shortEdited: boolean;
  special: Special;
  /** Weapons: inherit sound/behavior from this weapon ("" = the reference). */
  soundLike: string;
  shop: Shop | null;
  pickups: string[];
  /** Set: inject as clothing. */
  clothes: Clothes | null;
  include: boolean;
  traffic: TrafficChoice;
  audio: boolean;
  useHandling: boolean;
  useCleo: boolean;
}
export interface InjectRequest {
  id: number;
  name: string;
  short: string;
  reference: string;
  weapon: boolean;
  traffic: TrafficChoice;
  audio: boolean;
  useHandling: boolean;
  useCleo: boolean;
  special: Special | null;
  soundLike: string | null;
  shop: Shop | null;
  pickups: string[];
  clothes: Clothes | null;
}
export interface InjectResult { id: number; ok: boolean; message: string }
export interface Problem { id: number; problem: string | null }
export interface Check { id: number; short: string; weapon: boolean; reference: string; clothes: boolean }
