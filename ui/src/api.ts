import { invoke } from "@tauri-apps/api/core";
import type {
  Check, Clothes, InjectRequest, InjectResult, ItemDto, Problem, Shop, Snapshot, StoreInfo, TextureFit, TrafficChoice,
} from "./types";
import { mockApi } from "./mock";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export interface Api {
  startup(): Promise<Snapshot>;
  snapshot(): Promise<Snapshot>;
  openGame(path: string): Promise<Snapshot>;
  scanPaths(paths: string[]): Promise<ItemDto[]>;
  forgetItems(ids: number[]): Promise<void>;
  suggestShort(id: number | null, name: string, weapon: boolean): Promise<string>;
  hudName(name: string): Promise<string>;
  validate(checks: Check[]): Promise<Problem[]>;
  inject(requests: InjectRequest[]): Promise<InjectResult[]>;
  setEnabled(key: string, enabled: boolean): Promise<string>;
  remove(key: string): Promise<string>;
  editMod(key: string, name: string, traffic: TrafficChoice | null, audio: boolean | null, shop: Shop | null, setShop: boolean, pickups: string[] | null, clothes: Clothes | null): Promise<string>;
  /** The catalog's shops, the original models and whether CLEO is installed. */
  storeInfo(): Promise<StoreInfo>;
  /** Whether a queued texture will be scaled to the CJ in use; `model` "" = its own model. */
  textureFit(id: number, model: string, bodyPart: number): Promise<TextureFit>;
  shopDefaults(reference: string): Promise<Shop>;
  repair(): Promise<string>;
  restore(): Promise<string>;
}

const real: Api = {
  startup: () => invoke("startup"),
  snapshot: () => invoke("snapshot"),
  openGame: (path) => invoke("open_game", { path }),
  scanPaths: (paths) => invoke("scan_paths", { paths }),
  forgetItems: (ids) => invoke("forget_items", { ids }),
  suggestShort: (id, name, weapon) => invoke("suggest_short", { id, name, weapon }),
  hudName: (name) => invoke("hud_name", { name }),
  validate: (checks) => invoke("validate", { checks }),
  inject: (requests) => invoke("inject", { requests }),
  setEnabled: (key, enabled) => invoke("set_enabled", { key, enabled }),
  remove: (key) => invoke("remove", { key }),
  editMod: (key, name, traffic, audio, shop, setShop, pickups, clothes) =>
    invoke("edit_mod", { key, name, traffic, audio, shop, setShop, pickups, clothes }),
  storeInfo: () => invoke("store_info"),
  textureFit: (id, model, bodyPart) => invoke("texture_fit", { id, model, bodyPart }),
  shopDefaults: (reference) => invoke("shop_defaults", { reference }),
  repair: () => invoke("repair"),
  restore: () => invoke("restore"),
};

/** In a plain browser (`npm run dev`) the app runs on fake data, to work on the look. */
export const api: Api = inTauri ? real : mockApi;

/** Tauri rejects with the error string; make it a message either way. */
export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);
}
