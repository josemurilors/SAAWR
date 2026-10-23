export type Severity = "warn" | "error";
export interface Finding { severity: Severity; msg: string }
export interface RefDto { model: string; kind: string }
export interface WeaponDto { name: string; melee: boolean }
export interface GroupDto { index: number; label: string; len: number }
export interface GameDto {
  root: string;
  modloader: boolean;
  fla: boolean;
  ola: boolean;
  refs: RefDto[];
  weapons: WeaponDto[];
  groups: GroupDto[];
}
export interface ModDto {
  key: string;
  id: number | null;
  enabled: boolean;
  foreign: boolean;
  kind: "vehicle" | "weapon" | null;
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
  folder: string;
  extras: Extras;
}
export type TrafficKind = "auto" | "none" | "group";
export interface TrafficChoice { kind: TrafficKind; index?: number }
/** A queue item with what the user can change on it. */
export interface QItem extends ItemDto {
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
}
export interface InjectResult { id: number; ok: boolean; message: string }
export interface Problem { id: number; problem: string | null }
export interface Check { id: number; short: string; weapon: boolean; reference: string }
