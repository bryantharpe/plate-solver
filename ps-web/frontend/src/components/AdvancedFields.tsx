import type { AdvancedParams } from '../types'

interface Props {
  advanced: AdvancedParams
  onChange: (advanced: AdvancedParams) => void
}

const FIELDS: {
  name: keyof AdvancedParams
  label: string
  placeholder: string
}[] = [
  { name: 'timeout_ms', label: 'Timeout (ms)', placeholder: '30000' },
  { name: 'match_radius', label: 'Match radius', placeholder: '0.01' },
  { name: 'match_threshold', label: 'Match threshold', placeholder: '0.00001' },
  { name: 'fov_max_error', label: 'FOV max error (deg)', placeholder: 'auto' },
  { name: 'distortion', label: 'Distortion coefficient', placeholder: 'estimated' },
]

export default function AdvancedFields({ advanced, onChange }: Props) {
  return (
    <details className="group rounded-lg border border-white/10 bg-space-900/40">
      <summary
        className="flex cursor-pointer select-none items-center gap-2 px-3 py-2 text-xs
          font-semibold uppercase tracking-wider text-ink-muted transition hover:text-ink"
      >
        <svg
          viewBox="0 0 16 16"
          fill="currentColor"
          className="h-3 w-3 transition-transform group-open:rotate-90"
        >
          <path d="M6 4l4 4-4 4V4z" />
        </svg>
        Advanced
      </summary>
      <div className="grid gap-3 px-3 pb-3 pt-1">
        {FIELDS.map(({ name, label, placeholder }) => (
          <div key={name}>
            <label htmlFor={name} className="field-label">
              {label}
            </label>
            <input
              id={name}
              type="number"
              step="any"
              min={name === 'distortion' ? undefined : '0'}
              placeholder={placeholder}
              value={advanced[name]}
              onChange={(event) =>
                onChange({ ...advanced, [name]: event.target.value })
              }
              className="field-input font-mono-num"
            />
          </div>
        ))}
      </div>
    </details>
  )
}
