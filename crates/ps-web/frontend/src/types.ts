export interface HealthResponse {
  status: string
  version: string
  star_catalog: string
  min_fov: number
  max_fov: number
  num_patterns: number
}

export interface MatchedStar {
  x: number
  y: number
  ra: number
  dec: number
  mag: number
  cat_id: number
}

export interface MatchFound {
  status: 'match_found'
  ra_deg: number
  ra_hms: string
  dec_deg: number
  dec_dms: string
  roll_deg: number
  fov_deg: number
  rmse: number
  p90e: number
  maxe: number
  matches: number
  prob: number
  t_solve_ms: number
  matched_stars: MatchedStar[]
}

export interface NoMatch {
  status: 'no_match' | 'timeout' | 'cancelled' | 'too_few'
  hint: string
}

export type SolveResponse = MatchFound | NoMatch

export interface AdvancedParams {
  timeout_ms: string
  match_radius: string
  match_threshold: string
  fov_max_error: string
  distortion: string
}

export const EMPTY_ADVANCED: AdvancedParams = {
  timeout_ms: '',
  match_radius: '',
  match_threshold: '',
  fov_max_error: '',
  distortion: '',
}
