import { useCallback, useEffect, useRef, useState } from 'react'
import { fetchHealth, solve } from './api'
import type { AdvancedParams, HealthResponse, SolveResponse } from './types'
import { EMPTY_ADVANCED } from './types'
import UploadCard from './components/UploadCard'
import StatusBanner from './components/StatusBanner'
import SolveResult from './components/SolveResult'

type Phase =
  | { kind: 'idle' }
  | { kind: 'solving' }
  | { kind: 'done'; result: SolveResponse }
  | { kind: 'error'; message: string }

export default function App() {
  const [health, setHealth] = useState<HealthResponse | null>(null)
  const [healthFailed, setHealthFailed] = useState(false)
  const [file, setFile] = useState<File | null>(null)
  const [previewUrl, setPreviewUrl] = useState<string | null>(null)
  const [fov, setFov] = useState('')
  const [advanced, setAdvanced] = useState<AdvancedParams>(EMPTY_ADVANCED)
  const [phase, setPhase] = useState<Phase>({ kind: 'idle' })
  const previewUrlRef = useRef<string | null>(null)

  useEffect(() => {
    fetchHealth()
      .then(setHealth)
      .catch(() => setHealthFailed(true))
  }, [])

  const handleFile = useCallback((next: File) => {
    if (previewUrlRef.current) URL.revokeObjectURL(previewUrlRef.current)
    const url = URL.createObjectURL(next)
    previewUrlRef.current = url
    setFile(next)
    setPreviewUrl(url)
    setPhase({ kind: 'idle' })
  }, [])

  const handleSolve = useCallback(async () => {
    if (!file || fov === '') return
    setPhase({ kind: 'solving' })
    try {
      const result = await solve(file, fov, advanced)
      setPhase({ kind: 'done', result })
    } catch (err) {
      setPhase({
        kind: 'error',
        message: err instanceof Error ? err.message : String(err),
      })
    }
  }, [file, fov, advanced])

  return (
    <div className="mx-auto max-w-5xl px-4 pb-24 pt-10 sm:px-6">
      <header className="mb-8 flex flex-wrap items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <img src="/favicon.svg" alt="" className="h-10 w-10" />
          <div>
            <h1 className="text-2xl font-bold tracking-tight">Plate Solver</h1>
            <p className="text-sm text-ink-muted">
              Upload a star-field image to solve for its sky position.
            </p>
          </div>
        </div>
        <HealthPill health={health} failed={healthFailed} />
      </header>

      <UploadCard
        file={file}
        previewUrl={previewUrl}
        fov={fov}
        advanced={advanced}
        solving={phase.kind === 'solving'}
        fovRange={health ? { min: health.min_fov, max: health.max_fov } : null}
        onFile={handleFile}
        onFovChange={setFov}
        onAdvancedChange={setAdvanced}
        onSolve={handleSolve}
      />

      <div className="mt-8">
        {phase.kind === 'error' && (
          <StatusBanner kind="error" title="Request failed" body={phase.message} />
        )}
        {phase.kind === 'done' && phase.result.status !== 'match_found' && (
          <StatusBanner
            kind="hint"
            title={STATUS_TITLES[phase.result.status] ?? phase.result.status}
            body={phase.result.hint}
          />
        )}
        {phase.kind === 'done' && phase.result.status === 'match_found' && (
          <SolveResult result={phase.result} imageUrl={previewUrl} />
        )}
      </div>
    </div>
  )
}

const STATUS_TITLES: Record<string, string> = {
  no_match: 'No match found',
  timeout: 'Solve timed out',
  cancelled: 'Solve cancelled',
  too_few: 'Too few stars detected',
}

function HealthPill({
  health,
  failed,
}: {
  health: HealthResponse | null
  failed: boolean
}) {
  if (failed) {
    return (
      <div className="panel flex items-center gap-2 px-3 py-1.5 text-xs text-ink-muted">
        <span className="h-2 w-2 rounded-full bg-red-400" />
        server unreachable
      </div>
    )
  }
  if (!health) {
    return (
      <div className="panel px-3 py-1.5 text-xs text-ink-faint">connecting…</div>
    )
  }
  return (
    <div className="panel flex items-center gap-2 px-3 py-1.5 text-xs text-ink-muted">
      <span className="h-2 w-2 rounded-full bg-star-ring" />
      <span>
        {health.star_catalog} · {health.num_patterns.toLocaleString()} patterns ·
        FOV {health.min_fov}°–{health.max_fov}°
      </span>
    </div>
  )
}
