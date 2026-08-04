import { useCallback, useRef, useState } from 'react'
import type { AdvancedParams } from '../types'
import AdvancedFields from './AdvancedFields'

interface Props {
  file: File | null
  previewUrl: string | null
  fov: string
  advanced: AdvancedParams
  solving: boolean
  fovRange: { min: number; max: number } | null
  onFile: (file: File) => void
  onFovChange: (fov: string) => void
  onAdvancedChange: (advanced: AdvancedParams) => void
  onSolve: () => void
}

const ACCEPTED_TYPES = ['image/jpeg', 'image/png']

export default function UploadCard({
  file,
  previewUrl,
  fov,
  advanced,
  solving,
  fovRange,
  onFile,
  onFovChange,
  onAdvancedChange,
  onSolve,
}: Props) {
  const inputRef = useRef<HTMLInputElement>(null)
  const [dragging, setDragging] = useState(false)

  const acceptFile = useCallback(
    (candidate: File | undefined) => {
      if (candidate && ACCEPTED_TYPES.includes(candidate.type)) onFile(candidate)
    },
    [onFile],
  )

  const handleDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault()
      setDragging(false)
      acceptFile(event.dataTransfer.files[0])
    },
    [acceptFile],
  )

  const canSolve = file !== null && fov !== '' && !solving

  return (
    <form
      id="solve-form"
      className="panel p-5 sm:p-6"
      onSubmit={(event) => {
        event.preventDefault()
        if (canSolve) onSolve()
      }}
    >
      <div className="grid gap-5 sm:grid-cols-[1.4fr_1fr]">
        <button
          type="button"
          onClick={() => inputRef.current?.click()}
          onDragOver={(event) => {
            event.preventDefault()
            setDragging(true)
          }}
          onDragLeave={() => setDragging(false)}
          onDrop={handleDrop}
          className={`group relative flex min-h-44 flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed px-4 py-6 text-center transition ${
            dragging
              ? 'border-accent bg-accent/10'
              : 'border-white/15 bg-space-900/40 hover:border-accent/50 hover:bg-space-900/70'
          }`}
        >
          {previewUrl ? (
            <>
              <img
                src={previewUrl}
                alt="Selected star-field preview"
                className="max-h-48 rounded-lg border border-white/10 object-contain"
              />
              <span className="mt-1 max-w-full truncate text-xs text-ink-muted">
                {file?.name} — click or drop to replace
              </span>
            </>
          ) : (
            <>
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                className="h-9 w-9 text-ink-faint transition group-hover:text-accent"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M12 16.5V7m0 0l-3.5 3.5M12 7l3.5 3.5M4.5 19.5h15"
                />
              </svg>
              <span className="text-sm font-medium text-ink">
                Drop a star-field image here
              </span>
              <span className="text-xs text-ink-muted">
                or click to browse — JPEG or PNG
              </span>
            </>
          )}
          <input
            ref={inputRef}
            id="image"
            type="file"
            accept={ACCEPTED_TYPES.join(',')}
            className="hidden"
            onChange={(event) => acceptFile(event.target.files?.[0])}
          />
        </button>

        <div className="flex flex-col gap-4">
          <div>
            <label htmlFor="fov_estimate" className="field-label">
              FOV estimate (degrees)
            </label>
            <input
              id="fov_estimate"
              type="number"
              step="any"
              min="0"
              required
              value={fov}
              onChange={(event) => onFovChange(event.target.value)}
              placeholder={fovRange ? `${fovRange.min}–${fovRange.max}` : ''}
              className="field-input font-mono-num"
            />
            <p className="mt-1 text-xs text-ink-faint">
              {fovRange
                ? `Supported range: ${fovRange.min}° – ${fovRange.max}°`
                : 'Loading supported FOV range…'}
            </p>
          </div>

          <AdvancedFields advanced={advanced} onChange={onAdvancedChange} />

          <button
            type="submit"
            disabled={!canSolve}
            className="mt-auto inline-flex items-center justify-center gap-2 rounded-lg
              bg-gradient-to-r from-indigo-500 to-violet-500 px-5 py-2.5 text-sm font-semibold
              text-white shadow-lg shadow-indigo-950/50 transition
              hover:from-indigo-400 hover:to-violet-400
              disabled:cursor-not-allowed disabled:opacity-40 disabled:saturate-50"
          >
            {solving && (
              <span
                className="h-4 w-4 animate-spin rounded-full border-2 border-white/40
                  border-t-white"
              />
            )}
            {solving ? 'Solving…' : 'Solve'}
          </button>
        </div>
      </div>
    </form>
  )
}
