#!/bin/sh
# Browser tests, one run at a time on this machine.
#
# playwright.config.ts starts its finder servers on fixed loopback ports, and the
# factory's builders and gates share this host's network. Two runs at once would fight
# over those ports, and the second would fail before a test ran. So a run builds the
# release server first, outside the lock, then waits its turn on a lock file in the
# Playwright browser cache, which every builder and gate on the host mounts from the
# same folder.
set -eu

# Run from the page's folder with its own Playwright, as `npm run e2e` does.
cd "$(dirname "$0")/.."
PATH="$PWD/node_modules/.bin:$PATH"

# Against a server that is already running (FINDER_URL), nothing here binds a port.
if [ -n "${FINDER_URL:-}" ]; then
  exec playwright test "$@"
fi

cargo build --release --locked -p finder

lock_dir="${PLAYWRIGHT_BROWSERS_PATH:-$HOME/.cache/ms-playwright}"
mkdir -p "$lock_dir"
if ! command -v flock >/dev/null 2>&1; then
  echo "e2e: flock is not installed, so this run does not wait for other runs" >&2
  exec playwright test "$@"
fi
exec flock --verbose --wait 3600 "$lock_dir/.finder-e2e.lock" playwright test "$@"
