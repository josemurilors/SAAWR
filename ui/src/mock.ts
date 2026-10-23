import type { Api } from "./api";
import type { ItemDto, ModDto, Snapshot } from "./types";

// Fake data for `npm run dev` in a plain browser, to work on the look without the game.
const delay = (ms = 250) => new Promise((r) => setTimeout(r, ms));

const refs = [
  ["landstal", "car"], ["uranus", "car"], ["infernus", "car"], ["sanchez", "bike"],
  ["pcj600", "bike"], ["predator", "boat"], ["launch", "boat"], ["beagle", "plane"],
  ["dodo", "plane"], ["maverick", "heli"],
].map(([model, kind]) => ({ model, kind }));

const weapons = [
  ["PISTOL", false], ["DESERT_EAGLE", false], ["M4", false], ["AK47", false],
  ["DILDO1", true], ["KATANA", true],
].map(([name, melee]) => ({ name: name as string, melee: melee as boolean }));

const groups = ["workers", "business", "clubbers", "farmers", "beachfolk", "casual average", "boats"].map(
  (label, index) => ({ index, label: `${index} - ${label}`, len: 8 + index }),
);

const mk = (m: Partial<ModDto>): ModDto => ({
  key: "", id: null, enabled: true, foreign: false, kind: "vehicle", name: "", short: "",
  reference: "", vehicleType: "car", weaponTypeId: null, parts: 0, paintjobs: 0, ivfc: false,
  packHandling: false, traffic: "como a referência", ...m,
});

let mods: ModDto[] = [
  mk({ key: "c1", id: 12502, name: "VW New Beetle", short: "beetle", reference: "uranus", parts: 14, paintjobs: 4, ivfc: true, packHandling: true }),
  mk({ key: "c2", id: 12517, name: "BMW F800 GS", short: "bmw800g", reference: "sanchez", vehicleType: "bike", traffic: "sem tráfego" }),
  mk({ key: "c3", id: 12518, name: "Launch Marinha", short: "mlaunch", reference: "launch", vehicleType: "boat", enabled: false, traffic: "6 - boats" }),
  mk({ key: "c4", id: 12519, name: "King Air C90B", short: "kingair", reference: "beagle", vehicleType: "plane" }),
  mk({ key: "w1", id: 12400, kind: "weapon", name: "luger", short: "luger", reference: "PISTOL", vehicleType: null, weaponTypeId: 70, traffic: "" }),
  mk({ key: "w2", id: 12402, kind: "weapon", name: "yardstick", short: "yardstick", reference: "DILDO1", vehicleType: null, weaponTypeId: 72, enabled: false, traffic: "" }),
  mk({ key: "f1", id: 12500, foreign: true, kind: null, name: "12500-spike1", vehicleType: null, traffic: "" }),
];

const root = "D:/Games/GTA SA - teste";
const snap = (): Snapshot => ({
  gtaPath: root,
  game: { root, modloader: true, fla: true, ola: true, refs, weapons, groups },
  mods: [...mods],
  findings: [
    { severity: "warn", msg: "outro cargrp.dat no modloader (modloader/outro/cargrp.dat); o tráfego do AWR só funciona sem ele" },
  ],
});

let nextId = 100;
const none = { parts: false, paintjobs: false, ivfc: false, handling: false, weaponLines: false, cleo: false };
const sample = (): ItemDto[] => [
  { id: ++nextId, name: "Nissan Skyline GT-R", short: "nissans", reference: "infernus", weapon: false, folder: "C:/mods/gtr",
    extras: { ...none, parts: true, paintjobs: true, handling: true } },
  { id: ++nextId, name: "Ducati", short: "ducati", reference: "pcj600", weapon: false, folder: "C:/mods/ducati", extras: none },
  { id: ++nextId, name: "AUG", short: "aug", reference: "M4", weapon: true, folder: "C:/mods/aug",
    extras: { ...none, weaponLines: true } },
  { id: ++nextId, name: "Mod sem referência", short: "semref", reference: "", weapon: false, folder: "C:/mods/x",
    extras: { ...none, cleo: true } },
];

export const mockApi: Api = {
  startup: async () => snap(),
  snapshot: async () => snap(),
  openGame: async () => snap(),
  scanPaths: async () => {
    await delay();
    return sample();
  },
  forgetItems: async () => {},
  validate: async (checks) =>
    checks.map((c) => ({
      id: c.id,
      problem: !c.reference ? "escolha a referência" : c.short === "aug" ? "nome curto já usado" : null,
    })),
  inject: async (reqs) => {
    await delay(900);
    return reqs.map((r) => ({ id: r.id, ok: true, message: `${root}/modloader/awr-cars/125xx-${r.short}` }));
  },
  setEnabled: async (key, enabled) => {
    await delay(150);
    mods = mods.map((m) => (m.key === key ? { ...m, enabled } : m));
    return key;
  },
  remove: async (key) => {
    await delay(150);
    mods = mods.filter((m) => m.key !== key);
    return "removido";
  },
  repair: async () => "reparado (0 injeção(ões) interrompida(s) limpa(s))",
};
