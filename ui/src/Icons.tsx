import type { SVGProps } from "react";

export type Kind = "car" | "bike" | "boat" | "plane" | "weapon";

/** From the IDE type column (car, bike, bmx, boat, plane, heli, quad, ...). */
export function kindOf(vehicleType: string | null | undefined, weapon = false): Kind {
  if (weapon) return "weapon";
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

export const KIND_LABEL: Record<Kind, string> = {
  car: "Carros",
  bike: "Motos",
  boat: "Barcos",
  plane: "Aviões",
  weapon: "Armas",
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

export function KindIcon({ kind, ...p }: { kind: Kind } & SVGProps<SVGSVGElement>) {
  return (
    <svg {...base} {...p}>
      {kind === "car" && (
        <>
          <path d="M3 22v-6l6-1 4-6h10l4 6 4 1v6z" />
          <circle cx="10" cy="23" r="4" />
          <circle cx="24" cy="23" r="4" />
        </>
      )}
      {kind === "bike" && (
        <>
          <circle cx="8" cy="23" r="6" />
          <circle cx="26" cy="23" r="6" />
          <path d="M8 23l7-9h7l4 9M22 14l2-5h4" />
        </>
      )}
      {kind === "boat" && <path d="M3 20h28l-5 8H8zM11 20l2-7h8l2 7M17 13V6" />}
      {kind === "plane" && <path d="M4 17h26M14 17l5-13h4l-2 13M14 17l5 13h4l-2-13M4 17l-2-5" />}
      {kind === "weapon" && <path d="M3 12h21v5h-8l-2 11H9l1-11H3zM24 14h7" />}
    </svg>
  );
}

export function Mark(p: SVGProps<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 28 28" width={26} height={26} {...p}>
      <rect x="1.5" y="1.5" width="25" height="25" rx="4" fill="#141414" stroke="#d4af37" strokeWidth="2.5" />
      <path d="M14 7v14M7 14h14" stroke="#d4af37" strokeWidth="2.6" strokeLinecap="round" />
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
