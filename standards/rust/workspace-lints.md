# `[workspace.lints]` snippet

Paste this into the consuming workspace's root `Cargo.toml`, and add
`[lints]\nworkspace = true` to each member crate. This is the mechanized lint
floor; a crate may tighten (e.g. add `#![forbid(unsafe_code)]`) but should not
loosen without a named override.

```toml
[workspace.lints.rust]
missing_docs = "deny"
unsafe_op_in_unsafe_fn = "deny"
unreachable_pub = "warn"
# Lint GROUPS need priority = -1 so the specific lints above win
# (Cargo ignores table order; otherwise clippy::lint_groups_priority fires).
rust_2018_idioms = { level = "warn", priority = -1 }

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
# Opt in per-rig when ready — pedantic is noisy but high-signal:
# pedantic = { level = "warn", priority = -1 }
```

Per member crate:

```toml
[lints]
workspace = true
```

The unsafe policy is expressed at the crate level, not here, so it is visible at
the top of each file:

- `#![forbid(unsafe_code)]` — crate contains no `unsafe`; census provably 0.
- `#![deny(unsafe_code)]` — crate has audited `unsafe`; each site carries an
  explicit `#[allow(unsafe_code)]` with a `// SAFETY:` comment. The audited set
  is `grep -rnE '^\s*#\[allow\(unsafe_code\)\]' src`.
```
