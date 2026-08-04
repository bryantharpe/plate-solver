import type { MatchedStar } from '../types'

interface Props {
  stars: MatchedStar[]
  hovered: number | null
  onHover: (index: number | null) => void
}

const HEADERS = ['Cat ID', 'x', 'y', 'RA (deg)', 'Dec (deg)', 'Mag']

export default function StarsTable({ stars, hovered, onHover }: Props) {
  return (
    <div className="panel max-h-80 overflow-y-auto">
      <table className="w-full text-sm" style={{ fontVariantNumeric: 'tabular-nums' }}>
        <thead className="sticky top-0 z-10 bg-space-800/95 backdrop-blur">
          <tr>
            {HEADERS.map((label) => (
              <th
                key={label}
                className="px-4 py-2.5 text-right text-[11px] font-semibold uppercase
                  tracking-wider text-ink-muted first:text-left"
              >
                {label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {stars.map((star, i) => (
            <tr
              key={i}
              onMouseEnter={() => onHover(i)}
              onMouseLeave={() => onHover(null)}
              className={`border-t border-white/5 transition-colors ${
                hovered === i ? 'bg-amber-400/10' : 'hover:bg-white/5'
              }`}
            >
              <td className="px-4 py-1.5 text-left font-medium text-accent-bright">
                {star.cat_id}
              </td>
              <td className="px-4 py-1.5 text-right text-ink-muted">
                {star.x.toFixed(2)}
              </td>
              <td className="px-4 py-1.5 text-right text-ink-muted">
                {star.y.toFixed(2)}
              </td>
              <td className="px-4 py-1.5 text-right">{star.ra.toFixed(6)}</td>
              <td className="px-4 py-1.5 text-right">{star.dec.toFixed(6)}</td>
              <td className="px-4 py-1.5 text-right">{star.mag.toFixed(2)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
