import type { MatchFound } from '../types'

export default function StatGrid({ result }: Props) {
  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-4 sm:grid-cols-2">
        <HeroCard
          label="Right ascension"
          value={`${result.ra_deg.toFixed(6)}°`}
          sub={result.ra_hms}
        />
        <HeroCard
          label="Declination"
          value={`${result.dec_deg.toFixed(6)}°`}
          sub={result.dec_dms}
        />
      </div>

      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        <StatTile label="Roll" value={`${result.roll_deg.toFixed(2)}°`} />
        <StatTile label="Solved FOV" value={`${result.fov_deg.toFixed(3)}°`} />
        <StatTile label="Matches" value={String(result.matches)} />
        <StatTile label="Solve time" value={`${result.t_solve_ms.toFixed(1)} ms`} />
        <StatTile label="RMSE" value={`${result.rmse.toFixed(2)} px`} />
        <StatTile label="P90 error" value={`${result.p90e.toFixed(2)} px`} />
        <StatTile label="Max error" value={`${result.maxe.toFixed(2)} px`} />
        <StatTile label="False-match prob" value={result.prob.toExponential(2)} />
      </div>
    </div>
  )
}

interface Props {
  result: MatchFound
}

function HeroCard({
  label,
  value,
  sub,
}: {
  label: string
  value: string
  sub: string
}) {
  return (
    <div className="panel relative overflow-hidden px-5 py-4">
      <div
        className="pointer-events-none absolute -right-8 -top-8 h-28 w-28 rounded-full
          bg-accent/10 blur-2xl"
      />
      <div className="text-xs font-semibold uppercase tracking-widest text-ink-muted">
        {label}
      </div>
      <div className="mt-1 text-3xl font-semibold text-accent-bright sm:text-4xl">
        {value}
      </div>
      <div className="font-mono-num mt-1 text-sm text-ink-muted">{sub}</div>
    </div>
  )
}

function StatTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="panel px-4 py-3">
      <div className="text-[11px] font-semibold uppercase tracking-wider text-ink-faint">
        {label}
      </div>
      <div className="mt-0.5 text-lg font-semibold text-ink">{value}</div>
    </div>
  )
}
