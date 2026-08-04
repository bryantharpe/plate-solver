// Supplementary browser smoke check for the ps-web UI — NOT part of the
// cargo gate (this repo has no CI/.github workflows; cargo fmt/clippy/test
// is the whole local gate). Run manually against a running server:
//
//   cargo run --release -p ps-web -- --db reference-solutions/cedar-solve/tetra3/data/default_database.npz &
//   node ps-web/frontend/e2e/smoke.mjs [base-url]
//
// Requires Playwright + a Chromium install reachable from this machine (the
// dev container ships one at /opt/pw-browsers/chromium via
// PLAYWRIGHT_BROWSERS_PATH; on another machine, install Playwright normally
// and drop the explicit executablePath below).
//
// Codifies the manual verification of the matched-star overlay, hover
// tooltip, and Aladin CDN-failure fallback done during development: uploads
// the medium-FOV reference image at FOV 11, asserts a match, that the
// overlay ring count equals the table row count, that hovering a ring shows
// a tooltip with catalog details, and that the Aladin section degrades to
// its fallback link when the CDN is unreachable (as it is in this sandbox).

import { fileURLToPath } from 'node:url'
import path from 'node:path'
import { createRequire } from 'node:module'

// Use createRequire (which honors NODE_PATH) instead of a static import, so
// this runs against a globally-installed Playwright without needing a
// package.json/node_modules of its own next to this script.
const { chromium } = createRequire(import.meta.url)('playwright')

const here = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(here, '../../..')
const referenceImage = path.join(
  repoRoot,
  'reference-solutions/cedar-solve/examples/data/medium_fov/2019-07-29T204726_Alt40_Azi-135_Try1.jpg',
)
const baseUrl = process.argv[2] ?? 'http://127.0.0.1:8080'

function assert(condition, message) {
  if (!condition) throw new Error(`FAIL: ${message}`)
  console.log(`ok - ${message}`)
}

const browser = await chromium.launch({
  executablePath: process.env.PLAYWRIGHT_CHROMIUM_PATH ?? '/opt/pw-browsers/chromium',
})
try {
  const page = await browser.newPage({ viewport: { width: 1360, height: 900 } })
  const pageErrors = []
  page.on('pageerror', (err) => pageErrors.push(err.message))

  await page.goto(baseUrl)
  await page.setInputFiles('input[type=file]', referenceImage)
  await page.fill('#fov_estimate', '11')
  await page.click('button[type=submit]')
  await page.waitForSelector('#result', { timeout: 60_000 })

  const raText = await page.locator('text=/230\\.6/').first().textContent()
  assert(raText != null, 'result shows an RA value near the reference solution (230.6°)')

  const rings = await page.locator('[data-testid=star-overlay] g').count()
  const rows = await page.locator('tbody tr').count()
  assert(rings > 0, `overlay renders at least one ring (got ${rings})`)
  assert(rings === rows, `overlay ring count (${rings}) equals table row count (${rows})`)

  await page.locator('[data-testid=star-overlay] g').first().hover()
  await page.waitForTimeout(300)
  const tooltip = await page.locator('text=/HIP \\d+/').count()
  assert(tooltip > 0, 'hovering a ring shows a catalog-id tooltip')

  const fallbackLink = await page.locator('a:has-text("Open in Aladin")').count()
  assert(fallbackLink > 0, 'an "Open in Aladin" link is present regardless of CDN availability')

  assert(pageErrors.length === 0, `no uncaught page errors (got: ${pageErrors.join('; ')})`)

  console.log('\nSMOKE PASS')
} finally {
  await browser.close()
}
