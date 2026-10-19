// Deep-sky symbols in the star-atlas convention every observer reads: square for a diffuse
// nebula, dotted circle for an open cluster, crossed circle for a globular, ellipse for a
// galaxy, rayed circle for a planetary nebula.

import type { Family } from "../api";

interface Props {
  family: Family;
  /** Size in CSS pixels (icon use) when `r` is not given. */
  size?: number;
  className?: string;
}

/** A family symbol drawn in a 24-unit box; stroke follows `currentColor`. */
export function FamilyIcon({ family, size = 18, className }: Props) {
  return (
    <svg viewBox="-12 -12 24 24" width={size} height={size} className={className} aria-hidden="true">
      <FamilyShape family={family} r={8} />
    </svg>
  );
}

/** The symbol's shape at radius `r`, centered on the origin. */
export function FamilyShape({ family, r }: { family: Family; r: number }) {
  const common = {
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.6,
    vectorEffect: "non-scaling-stroke" as const,
  };
  switch (family) {
    case "nebula":
      return <rect x={-r} y={-r} width={2 * r} height={2 * r} {...common} />;
    case "open_cluster":
      return <circle r={r} {...common} strokeDasharray="2 2.5" />;
    case "globular_cluster":
      return (
        <g {...common}>
          <circle r={r} />
          <path d={`M${-r} 0H${r}M0 ${-r}V${r}`} />
        </g>
      );
    case "galaxy":
      return <ellipse rx={r} ry={r * 0.5} {...common} />;
    case "planetary_nebula":
      return (
        <g {...common}>
          <circle r={r * 0.55} />
          <path d={`M0 ${-r}V${-r * 0.55}M0 ${r * 0.55}V${r}M${-r} 0H${-r * 0.55}M${r * 0.55} 0H${r}`} />
        </g>
      );
    case "supernova_remnant":
      return (
        <g {...common}>
          <rect x={-r} y={-r} width={2 * r} height={2 * r} />
          <path d={`M${-r * 0.5} 0H${r * 0.5}M0 ${-r * 0.5}V${r * 0.5}`} />
        </g>
      );
    case "star":
      return (
        <g {...common}>
          <circle r={r * 0.35} />
          <circle r={r * 0.35} cx={r * 0.6} />
        </g>
      );
  }
}

export const FAMILY_LABEL: Record<Family, string> = {
  nebula: "Nebula",
  planetary_nebula: "Planetary",
  supernova_remnant: "Remnant",
  open_cluster: "Open cluster",
  globular_cluster: "Globular",
  galaxy: "Galaxy",
  star: "Star group",
};
