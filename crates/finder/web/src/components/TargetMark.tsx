// A target belongs to the session, outside the five switchable chart layers.
import type { Target, TargetPlacement } from "../api";
import { DirectionArrow } from "./NextTargets";
import { FamilyShape } from "./Symbol";

interface Props {
  target: Target;
  placement: TargetPlacement;
  W: number;
  H: number;
  /** Image pixels per CSS pixel, as used by the chart's other furniture. */
  u: number;
}

export function TargetMark({ target, placement: p, W, H, u }: Props) {
  const id = target.messier ?? target.id;
  if (!p.on_frame) {
    // Intersect the centre-to-target ray with an inset neatline. The inset leaves
    // the whole arrow visible, even where the ray meets a corner.
    const angle = (p.direction_deg * Math.PI) / 180;
    const dx = Math.sin(angle);
    const dy = -Math.cos(angle);
    const inset = 12 * u;
    const reach = Math.min((W / 2 - inset) / Math.abs(dx), (H / 2 - inset) / Math.abs(dy));
    const x = W / 2 + dx * reach;
    const y = H / 2 + dy * reach;
    const label = `${id} · ${Math.round(p.distance_deg)}°`;
    const right = x > W / 2;
    const labelY = y < 32 * u ? 25 * u : y > H - 32 * u ? -18 * u : 4 * u;
    return (
      <g data-target={id} transform={`translate(${x} ${y})`} className={`object-${target.family} pointer-events-none`}>
        <title>{`Target ${label}`}</title>
        <svg x={-10 * u} y={-10 * u} width={20 * u} height={20 * u} viewBox="0 0 15 15">
          <DirectionArrow degrees={p.direction_deg} />
        </svg>
        <text
          x={(right ? -16 : 16) * u}
          y={labelY}
          textAnchor={right ? "end" : "start"}
          fontSize={13 * u}
          strokeWidth={3 * u}
          className="object-label"
        >
          {label}
        </text>
      </g>
    );
  }

  const r = Math.max(8 * u, Math.min(p.axes[0], 60 * u));
  const minor = Math.max(3 * u, r * (p.axes[0] > 0 ? p.axes[1] / p.axes[0] : 0.5));
  const reach = r + 6 * u;
  const tick = 6 * u;
  const labelRight = p.x < W / 2;
  const labelX = (labelRight ? 1 : -1) * (reach + 5 * u);
  return (
    <g
      data-target={id}
      transform={`translate(${p.x} ${p.y})`}
      className={`object-${target.family} pointer-events-none`}
    >
      <title>{`Target ${id}`}</title>
      <g className="target-symbol">
        {target.family === "galaxy" ? (
          <ellipse
            rx={r}
            ry={minor}
            transform={`rotate(${p.angle - 90})`}
            fill="none"
            stroke="currentColor"
            strokeWidth={1.6}
            vectorEffect="non-scaling-stroke"
          />
        ) : (
          <FamilyShape family={target.family} r={r} />
        )}
      </g>
      <path
        className="target-reticle"
        d={`M${-reach} ${-reach + tick}V${-reach}H${-reach + tick}M${reach - tick} ${-reach}H${reach}V${-reach + tick}M${reach} ${reach - tick}V${reach}H${reach - tick}M${-reach + tick} ${reach}H${-reach}V${reach - tick}`}
        fill="none"
        stroke="var(--color-on-photo)"
        strokeWidth={1.5 * u}
      />
      <text
        x={labelX}
        y={4 * u}
        textAnchor={labelRight ? "start" : "end"}
        fontSize={13 * u}
        strokeWidth={3 * u}
        className="object-label"
      >
        {id}
      </text>
    </g>
  );
}
