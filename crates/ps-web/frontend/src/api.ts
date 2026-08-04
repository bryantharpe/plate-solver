import type { AdvancedParams, HealthResponse, SolveResponse } from './types'

export async function fetchHealth(): Promise<HealthResponse> {
  const res = await fetch('/healthz')
  if (!res.ok) throw new Error(`healthz returned ${res.status}`)
  return res.json()
}

/**
 * POST /api/solve. Field names must stay byte-identical to the server's
 * multipart contract; empty optional fields are omitted, not sent as "".
 * Throws with the server's `error` message on non-2xx responses.
 */
export async function solve(
  image: File,
  fovEstimate: string,
  advanced: AdvancedParams,
): Promise<SolveResponse> {
  const form = new FormData()
  form.append('image', image)
  form.append('fov_estimate', fovEstimate)
  for (const [name, value] of Object.entries(advanced)) {
    if (value !== '') form.append(name, value)
  }

  const res = await fetch('/api/solve', { method: 'POST', body: form })
  const data = await res.json()
  if (!res.ok) {
    throw new Error(data.error || `Request failed with status ${res.status}`)
  }
  return data as SolveResponse
}
