import { useState } from 'react'
import type { MatchFound } from '../types'
import StatGrid from './StatGrid'
import StarOverlay from './StarOverlay'
import StarsTable from './StarsTable'
import AladinView from './AladinView'

interface Props {
  result: MatchFound
  imageUrl: string | null
}

export default function SolveResult({ result, imageUrl }: Props) {
  // Index into matched_stars shared by the overlay and the table so hovering
  // either one highlights the same star in both.
  const [hovered, setHovered] = useState<number | null>(null)

  return (
    <div id="result" className="flex flex-col gap-8">
      <StatGrid result={result} />

      {imageUrl && (
        <section>
          <SectionTitle>Matched stars on your image</SectionTitle>
          <StarOverlay
            imageUrl={imageUrl}
            stars={result.matched_stars}
            hovered={hovered}
            onHover={setHovered}
          />
        </section>
      )}

      <section>
        <SectionTitle>Matched stars ({result.matched_stars.length})</SectionTitle>
        <StarsTable
          stars={result.matched_stars}
          hovered={hovered}
          onHover={setHovered}
        />
      </section>

      <section>
        <SectionTitle>Sky view</SectionTitle>
        <AladinView result={result} />
      </section>
    </div>
  )
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h2 className="mb-3 text-xs font-semibold uppercase tracking-widest text-ink-muted">
      {children}
    </h2>
  )
}
