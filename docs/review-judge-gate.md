# `review/judge` — the U3 required check

Every PR to `rewrite` requires a passing `review/judge` commit status. The
review itself runs town-side (the judge model is served on the operator's
loopback, unreachable from CI); only the verdict is published here. Absence of
the status fails closed: the PR cannot merge until a verdict is posted.

- APPROVE → `success`. DISSENT or an incomplete review → `failure`.
- The full verdict is posted as a PR comment (audit artifact); the status is
  the enforcement.
- Human override (U4): a human may supersede the judge's verdict; the override
  is posted as the status with the login and reason recorded. There is no
  silent bypass — `enforce_admins` is on.
- Statuses bind to head SHAs: every new push needs a fresh verdict; a stale
  approval never carries.

Specified in `standards/universal/JUDGE-REVIEW.md`.
