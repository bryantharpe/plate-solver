import { useEffect, useRef, useState } from 'react'
import type { MatchFound } from '../types'

const ALADIN_URL = 'https://aladin.cds.unistra.fr/AladinLite/api/v3/latest/aladin.js'
const LOAD_TIMEOUT_MS = 10_000

/* eslint-disable @typescript-eslint/no-explicit-any */
declare global {
  interface Window {
    A?: any
  }
}

// Module-level cache: inject the CDN script once, share the promise across
// mounts (also guards React StrictMode's double effect invocation).
let aladinPromise: Promise<any> | null = null

function loadAladin(): Promise<any> {
  if (!aladinPromise) {
    aladinPromise = new Promise((resolve, reject) => {
      if (window.A?.init) {
        resolve(window.A)
        return
      }
      const script = document.createElement('script')
      script.src = ALADIN_URL
      const timer = setTimeout(() => {
        script.remove()
        reject(new Error('timed out'))
      }, LOAD_TIMEOUT_MS)
      script.onload = () => {
        clearTimeout(timer)
        window.A?.init ? resolve(window.A) : reject(new Error('missing A.init'))
      }
      script.onerror = () => {
        clearTimeout(timer)
        reject(new Error('failed to load'))
      }
      document.head.appendChild(script)
    })
    // Allow a retry on the next result if this attempt failed.
    aladinPromise.catch(() => {
      aladinPromise = null
    })
  }
  return aladinPromise
}

interface Props {
  result: MatchFound
}

export default function AladinView({ result }: Props) {
  const divRef = useRef<HTMLDivElement>(null)
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    let cancelled = false
    const container = divRef.current
    setFailed(false)

    loadAladin()
      .then(async (A) => {
        await A.init
        if (cancelled || !container) return
        container.innerHTML = ''
        const aladin = A.aladin(container, {
          target: `${result.ra_deg} ${result.dec_deg}`,
          fov: result.fov_deg * 2,
          cooFrame: 'ICRS',
        })
        const catalog = A.catalog({
          name: 'Matched stars',
          color: '#34d399',
          sourceSize: 10,
          shape: 'circle',
        })
        catalog.addSources(
          result.matched_stars.map((s) =>
            A.source(s.ra, s.dec, { cat_id: s.cat_id, mag: s.mag, x: s.x, y: s.y }),
          ),
        )
        aladin.addCatalog(catalog)
      })
      .catch(() => {
        if (!cancelled) setFailed(true)
      })

    return () => {
      cancelled = true
      if (container) container.innerHTML = ''
    }
  }, [result])

  const fallbackHref =
    'https://aladin.cds.unistra.fr/AladinLite/?target=' +
    encodeURIComponent(`${result.ra_deg} ${result.dec_deg}`) +
    '&fov=' +
    encodeURIComponent(String(result.fov_deg * 2))

  return (
    <div className="panel overflow-hidden">
      {failed ? (
        <p className="px-4 py-6 text-sm text-ink-muted">
          Interactive sky view unavailable (Aladin script failed to load) — use
          the link below.
        </p>
      ) : (
        <div ref={divRef} className="h-105 w-full" />
      )}
      <p className="border-t border-white/10 px-4 py-2 text-xs">
        <a
          href={fallbackHref}
          target="_blank"
          rel="noopener noreferrer"
          className="text-accent-bright underline-offset-2 hover:underline"
        >
          Open in Aladin ↗
        </a>
      </p>
    </div>
  )
}
