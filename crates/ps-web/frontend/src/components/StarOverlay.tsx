import { useState } from 'react'
import type { MatchedStar } from '../types'

interface Props {
  imageUrl: string
  stars: MatchedStar[]
  hovered: number | null
  onHover: (index: number | null) => void
}

/**
 * Renders the uploaded image with an SVG overlay ring at each matched star's
 * pixel position. The SVG shares the image's natural pixel coordinate system
 * via viewBox, so star x/y map correctly at any display size. Fully offline.
 */
export default function StarOverlay({ imageUrl, stars, hovered, onHover }: Props) {
  const [dims, setDims] = useState<{ w: number; h: number } | null>(null)

  // Ring/label sizes are in image pixels; scale them so they render at a
  // consistent on-screen size regardless of the image's resolution.
  const scale = dims ? Math.max(dims.w, dims.h) / 900 : 1
  const ring = 11 * scale
  const hoveredStar = hovered !== null ? stars[hovered] : null

  return (
    <div className="panel relative overflow-hidden p-2">
      <div className="relative">
        <img
          src={imageUrl}
          alt="Solved star-field with matched stars marked"
          className="w-full rounded-lg"
          onLoad={(event) => {
            const img = event.currentTarget
            setDims({ w: img.naturalWidth, h: img.naturalHeight })
          }}
        />
        {dims && (
          <svg
            viewBox={`0 0 ${dims.w} ${dims.h}`}
            className="absolute inset-0 h-full w-full"
            data-testid="star-overlay"
          >
            {stars.map((star, i) => {
              const active = hovered === i
              return (
                <g
                  key={i}
                  className="cursor-pointer"
                  onMouseEnter={() => onHover(i)}
                  onMouseLeave={() => onHover(null)}
                >
                  {/* generous invisible hit area */}
                  <circle cx={star.x} cy={star.y} r={ring * 2.2} fill="transparent" />
                  <circle
                    cx={star.x}
                    cy={star.y}
                    r={active ? ring * 1.35 : ring}
                    fill="none"
                    stroke={active ? '#fbbf24' : '#34d399'}
                    strokeWidth={(active ? 2.4 : 1.5) * scale}
                    opacity={active ? 1 : 0.85}
                  />
                  {active && (
                    <text
                      x={star.x + ring * 1.8}
                      y={star.y - ring * 1.8}
                      fill="#fbbf24"
                      fontSize={16 * scale}
                      fontFamily="ui-monospace, monospace"
                      paintOrder="stroke"
                      stroke="#05070f"
                      strokeWidth={3 * scale}
                    >
                      HIP {star.cat_id}
                    </text>
                  )}
                </g>
              )
            })}
          </svg>
        )}
        {hoveredStar && dims && (
          <div
            className="pointer-events-none absolute z-10 -translate-x-1/2 rounded-lg border
              border-white/15 bg-space-900/95 px-3 py-2 text-xs shadow-xl backdrop-blur"
            style={{
              left: `${(hoveredStar.x / dims.w) * 100}%`,
              top: `${(hoveredStar.y / dims.h) * 100}%`,
              transform: `translate(-50%, ${
                hoveredStar.y / dims.h > 0.75 ? 'calc(-100% - 18px)' : '18px'
              })`,
            }}
          >
            <div className="font-mono-num font-semibold text-amber-300">
              HIP {hoveredStar.cat_id}
            </div>
            <div className="font-mono-num mt-1 grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 text-ink-muted">
              <span>mag</span>
              <span className="text-ink">{hoveredStar.mag.toFixed(2)}</span>
              <span>RA</span>
              <span className="text-ink">{hoveredStar.ra.toFixed(5)}°</span>
              <span>Dec</span>
              <span className="text-ink">{hoveredStar.dec.toFixed(5)}°</span>
              <span>px</span>
              <span className="text-ink">
                {hoveredStar.x.toFixed(1)}, {hoveredStar.y.toFixed(1)}
              </span>
            </div>
          </div>
        )}
      </div>
      <p className="px-2 py-1.5 text-xs text-ink-faint">
        {stars.length} matched stars ringed at their detected pixel positions —
        hover a ring for catalog details.
      </p>
    </div>
  )
}
