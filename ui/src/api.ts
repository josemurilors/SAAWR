import { invoke } from "@tauri-apps/api/core";
import type {
  Check, InjectRequest, InjectResult, ItemDto, Problem, Snapshot,
} from "./types";
import { mockApi } from "./mock";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export interface Api {
  startup(): Promise<Snapshot>;
  snapshot(): Promise<Snapshot>;
  openGame(path: string): Promise<Snapshot>;
  scanPaths(paths: string[]): Promise<ItemDto[]>;
  forgetItems(ids: number[]): Promise<void>;
  validate(checks: Check[]): Promise<Problem[]>;
  inject(requests: InjectRequest[]): Promise<InjectResult[]>;
  setEnabled(key: string, enabled: boolean): Promise<string>;
  remove(key: string): Promise<string>;
  repair(): Promise<string>;
}

const real: Api = {
  startup: () => invoke("startup"),
  snapshot: () => invoke("snapshot"),
  openGame: (path) => invoke("open_game", { path }),
  scanPaths: (paths) => invoke("scan_paths", { paths }),
  forgetItems: (ids) => invoke("forget_items", { ids }),
  validate: (checks) => invoke("validate", { checks }),
  inject: (requests) => invoke("inject", { requests }),
  setEnabled: (key, enabled) => invoke("set_enabled", { key, enabled }),
  remove: (key) => invoke("remove", { key }),
  repair: () => invoke("repair"),
};

/** In a plain browser (`npm run dev`) the app runs on fake data, to work on the look. */
export const api: Api = inTauri ? real : mockApi;

/** Tauri rejects with the error string; make it a message either way. */
export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);
}
