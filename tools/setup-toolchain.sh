#!/usr/bin/env zsh
#
# plate-solver — toolchain bootstrap. Run this, then RESTART Claude Code.
# Installs what S1 verifies (cargo/rustc + protoc) + components F1 uses (clippy, rustfmt).
# The Python reference env is set up by the loop (S3), so it's intentionally NOT here.
# Idempotent — safe to re-run.
#
# Usage:  zsh tools/setup-toolchain.sh

set -uo pipefail
echo "==> plate-solver toolchain bootstrap (arch: $(uname -m))"

# --- 0. Corporate TLS: this machine has a corp CA bundle; make Rust tooling trust it.
#        (macOS cargo usually uses the system keychain, but this de-risks crate downloads.)
CA="/certs/corp-ca-bundle.pem"
if [[ -f "$CA" ]]; then
  echo "==> corp CA detected ($CA) — pointing cargo/git/curl at it"
  export CURL_CA_BUNDLE="$CA" CARGO_HTTP_CAINFO="$CA"
  git config --global http.sslCAInfo "$CA" 2>/dev/null || true
  mkdir -p "$HOME/.cargo"
  if [[ ! -f "$HOME/.cargo/config.toml" ]] || ! grep -q '\[http\]' "$HOME/.cargo/config.toml" 2>/dev/null; then
    printf '\n[http]\ncainfo = "%s"\n' "$CA" >> "$HOME/.cargo/config.toml"
  fi
fi

# --- 1. Rust (cargo, rustc) via rustup.
if command -v cargo >/dev/null 2>&1; then
  echo "==> rust: present ($(cargo --version))"
else
  echo "==> rust: installing via rustup…"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
fi
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

# Make sure the shells Claude Code spawns find cargo after restart.
ensure_cargo_in() {
  grep -q '.cargo/env' "$1" 2>/dev/null || \
    printf '\n# Rust (plate-solver bootstrap)\n[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"\n' >> "$1"
}
touch "$HOME/.zshrc"; ensure_cargo_in "$HOME/.zshrc"
[[ -e "$HOME/.zprofile" ]] && ensure_cargo_in "$HOME/.zprofile"

echo "==> rust: ensuring clippy + rustfmt"
rustup component add clippy rustfmt >/dev/null 2>&1 || true

# --- 2. protoc (tonic-build needs it for ps-grpc / feat-06).
if command -v protoc >/dev/null 2>&1; then
  echo "==> protoc: present ($(protoc --version))"
elif command -v brew >/dev/null 2>&1; then
  echo "==> protoc: installing via Homebrew…"
  brew install protobuf
else
  echo "!!  protoc missing and no Homebrew. Install brew (https://brew.sh) then 'brew install protobuf',"
  echo "    or grab a release from https://github.com/protocolbuffers/protobuf/releases and put protoc on PATH."
fi

# --- 3. Verify (mirrors S1).
echo; echo "==> verification"
ready=1
for t in rustc cargo protoc; do
  if command -v "$t" >/dev/null 2>&1; then
    printf '    [ok]   %-7s %s\n' "$t" "$($t --version 2>/dev/null | head -1)"
  else
    printf '    [MISS] %-7s\n' "$t"; ready=0
  fi
done
cargo clippy --version >/dev/null 2>&1 && printf '    [ok]   clippy  %s\n' "$(cargo clippy --version 2>/dev/null)" || echo "    [warn] clippy missing (used only by F1)"
rustfmt --version      >/dev/null 2>&1 && printf '    [ok]   rustfmt %s\n' "$(rustfmt --version 2>/dev/null)"      || echo "    [warn] rustfmt missing (used only by F1)"

echo
if [[ "$ready" -eq 1 ]]; then
  echo "==> READY ✅  Restart Claude Code (so .claude/agents register), then run the grind skill on plan.md."
else
  echo "==> NOT READY ❌  install the [MISS] tool(s), re-run this script, then restart Claude Code."
fi
