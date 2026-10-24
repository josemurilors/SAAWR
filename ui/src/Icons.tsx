import type { SVGProps } from "react";

export type VehicleKind = "car" | "bike" | "boat" | "plane";
export type WeaponClass =
  | "melee" | "pistol" | "smg" | "shotgun" | "rifle" | "sniper" | "launcher" | "thrown" | "heavy" | "spray" | "gear";
/** What an icon can draw. */
export type Kind = VehicleKind | WeaponClass;
/** Filter families of the Instalados tab. */
export type Family = VehicleKind | "weapon";

const WEAPON_CLASSES: Record<string, WeaponClass> = {
  PISTOL: "pistol", PISTOL_SILENCED: "pistol", DESERT_EAGLE: "pistol",
  MICRO_UZI: "smg", TEC9: "smg", MP5: "smg",
  SHOTGUN: "shotgun", SAWNOFF: "shotgun", SPAS12: "shotgun",
  AK47: "rifle", M4: "rifle",
  COUNTRYRIFLE: "sniper", SNIPERRIFLE: "sniper",
  RLAUNCHER: "launcher", RLAUNCHER_HS: "launcher",
  GRENADE: "thrown", TEARGAS: "thrown", MOLOTOV: "thrown", SATCHEL_CHARGE: "thrown", DETONATOR: "thrown",
  MINIGUN: "heavy", FTHROWER: "heavy",
  SPRAYCAN: "spray", EXTINGUISHER: "spray",
  CAMERA: "gear", NIGHTVISION: "gear", INFRARED: "gear", JETPACK: "gear", PARACHUTE: "gear",
};

/** Weapon family by the original weapon it was cloned from (`weapon.dat` name). */
export function weaponClass(reference: string | null | undefined): WeaponClass {
  return WEAPON_CLASSES[(reference ?? "").toUpperCase()] ?? "melee";
}

/** Vehicles by the IDE type column (car, bike, bmx, boat, plane, heli, quad, ...); weapons by reference. */
export function kindOf(vehicleType: string | null | undefined, weapon = false, reference?: string | null): Kind {
  if (weapon) return weaponClass(reference);
  switch (vehicleType) {
    case "bike":
    case "bmx":
      return "bike";
    case "boat":
      return "boat";
    case "plane":
    case "heli":
      return "plane";
    default:
      return "car";
  }
}

export const familyOf = (k: Kind): Family => (k === "car" || k === "bike" || k === "boat" || k === "plane" ? k : "weapon");

export const FAMILY_LABEL: Record<Family, string> = {
  car: "Carros",
  bike: "Motos",
  boat: "Barcos",
  plane: "Aviões",
  weapon: "Armas",
};

export const WEAPON_CLASS_LABEL: Record<WeaponClass, string> = {
  melee: "corpo a corpo", pistol: "pistola", smg: "submetralhadora", shotgun: "escopeta", rifle: "fuzil",
  sniper: "sniper", launcher: "lançador", thrown: "arremesso", heavy: "arma pesada", spray: "spray/extintor", gear: "equipamento",
};

const base: SVGProps<SVGSVGElement> = {
  viewBox: "0 0 34 34",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.8,
  strokeLinecap: "round",
  strokeLinejoin: "round",
  width: 34,
  height: 34,
};

const PATHS: Record<Kind, string> = {
  car: "M3 22v-6l6-1 4-6h10l4 6 4 1v6zM10 19a4 4 0 100 8 4 4 0 000-8zM24 19a4 4 0 100 8 4 4 0 000-8z",
  bike: "M8 17a6 6 0 100 12 6 6 0 000-12zM26 17a6 6 0 100 12 6 6 0 000-12zM8 23l7-9h7l4 9M22 14l2-5h4",
  boat: "M3 20h28l-5 8H8zM11 20l2-7h8l2 7M17 13V6",
  plane: "M4 17h26M14 17l5-13h4l-2 13M14 17l5 13h4l-2-13M4 17l-2-5",
  melee: "M5 29l14-14M18 9l7 7-4 4-7-7zM5 29l-2 2",
  pistol: "M3 12h21v5h-8l-2 11H9l1-11H3zM24 14h7",
  smg: "M3 13h22v5H15l-1 9h-4l1-9H3zM25 15h6M19 18v5h-3",
  shotgun: "M2 16h30v4H2zM6 16V13h14v3M20 20l-1 6h-5l1-6M5 20l-2 6",
  rifle: "M3 14h7l3-2h14v4h5v3H15l-2 8H9l2-8H3zM12 12v-2h7v2",
  sniper: "M2 18h30M9 11h12v4H9zM12 15v3M27 18l3 6M6 18l-3 5",
  launcher: "M3 13h24v8H3zM27 11v12M27 17h5M10 21l-1 7h4l1-7M3 17H1",
  thrown: "M17 13a8 8 0 100 16 8 8 0 000-16zM14 13V9h6v4M20 10l5-3",
  heavy: "M3 14h20M3 18h20M3 22h20M23 12v12h5V12zM28 18h4M10 24l-1 5h4l1-5",
  spray: "M11 12h11v17H11zM13 8h7v4h-7zM22 17h5M26 14l3-2M26 20l3 2",
  gear: "M11 14a6 6 0 100 12 6 6 0 000-12zM23 14a6 6 0 100 12 6 6 0 000-12zM17 20h0M5 20H2M29 20h3M11 14V9h12v5",
};

export function KindIcon({ kind, ...p }: { kind: Kind } & SVGProps<SVGSVGElement>) {
  return (
    <svg {...base} {...p}>
      <path d={PATHS[kind]} />
    </svg>
  );
}

export function Mark(p: SVGProps<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 28 28" width={26} height={26} {...p}>
      <rect x="1.5" y="1.5" width="25" height="25" rx="4" fill="#1A1A1A" stroke="#F2F0E6" strokeWidth="2.5" />
      <path d="M14 7v14M7 14h14" stroke="#F2F0E6" strokeWidth="2.6" strokeLinecap="round" />
    </svg>
  );
}

const small: SVGProps<SVGSVGElement> = {
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 2,
  strokeLinecap: "round",
  strokeLinejoin: "round",
  width: 16,
  height: 16,
};

export const Folder = (p: SVGProps<SVGSVGElement>) => (
  <svg {...small} {...p}>
    <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
  </svg>
);
export const Upload = (p: SVGProps<SVGSVGElement>) => (
  <svg {...small} {...p}>
    <path d="M12 16V4m0 0L7 9m5-5l5 5M4 20h16" />
  </svg>
);
export const Trash = (p: SVGProps<SVGSVGElement>) => (
  <svg {...small} {...p}>
    <path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" />
  </svg>
);
export const Close = (p: SVGProps<SVGSVGElement>) => (
  <svg {...small} {...p}>
    <path d="M6 6l12 12M18 6L6 18" />
  </svg>
);
export const Wrench = (p: SVGProps<SVGSVGElement>) => (
  <svg {...small} {...p}>
    <path d="M14 6a4 4 0 0 0 5 5l-9 9a2.1 2.1 0 0 1-3-3l9-9a4 4 0 0 0-2-2z" />
  </svg>
);
export const Copy = (p: SVGProps<SVGSVGElement>) => (
  <svg {...small} {...p}>
    <rect x="9" y="9" width="11" height="11" rx="2" />
    <path d="M5 15V6a2 2 0 0 1 2-2h8" />
  </svg>
);
