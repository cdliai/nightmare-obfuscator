#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NIGHTMARE_BIN="${NIGHTMARE_BIN:-$ROOT/target/release/nightmare}"
WORK_ROOT="${NIGHTMARE_SMOKE_WORKDIR:-${RUNNER_TEMP:-${TMPDIR:-/tmp}}/nightmare-oss-smoke}"

TARGETS=(
  "hex|https://github.com/KokaKiwi/rust-hex|b2b4370b5bf021b98ee7adc92233e8de3f2de792|library,tests,doctests,public-api"
  "itoa|https://github.com/dtolnay/itoa|3ebb37c420c46dc5f274e804b4a0f00e906445ac|library,tests,public-api"
  "ripgrep|https://github.com/BurntSushi/ripgrep|0e8390a66fbcf6eeac1aeb0541b367663a597c79|binary,workspace,features,tests"
  "serde-json|https://github.com/serde-rs/json|535e0eba8eb58c51ead450d50d500d352e56bb93|serde,derives,tests,public-api"
  "bitflags|https://github.com/bitflags/bitflags|13513699141432af1dea2a6208e99e7bf21958db|macros,derives,tests,public-api"
)

usage() {
  cat <<'USAGE'
Usage:
  scripts/oss_smoke_matrix.sh --list
  scripts/oss_smoke_matrix.sh --target <name>
  scripts/oss_smoke_matrix.sh --all

Runs pinned real OSS Rust smoke targets through:
  original cargo test
  original cargo test --doc
  nightmare run --json
  machine-readable stage validation
USAGE
}

list_targets() {
  for target in "${TARGETS[@]}"; do
    IFS='|' read -r name repo ref coverage <<<"$target"
    printf '%s\t%s\t%s\t%s\n' "$name" "$repo" "$ref" "$coverage"
  done
}

find_target() {
  local wanted="$1"
  for target in "${TARGETS[@]}"; do
    IFS='|' read -r name repo ref coverage <<<"$target"
    if [[ "$name" == "$wanted" ]]; then
      printf '%s\n' "$target"
      return 0
    fi
  done
  return 1
}

run_target() {
  local target="$1"
  IFS='|' read -r name repo ref coverage <<<"$target"
  local checkout="$WORK_ROOT/$name/src"
  local output="$WORK_ROOT/$name/obfs"
  local config="$WORK_ROOT/$name/nightmare.toml"
  local result_json="$WORK_ROOT/$name/result.json"

  rm -rf "$WORK_ROOT/$name"
  mkdir -p "$WORK_ROOT/$name"

  git init "$checkout"
  git -C "$checkout" remote add origin "$repo"
  git -C "$checkout" fetch --depth 1 origin "$ref"
  git -C "$checkout" checkout --detach FETCH_HEAD
  test "$(git -C "$checkout" rev-parse HEAD)" = "$ref"

  cargo test --all-targets --manifest-path "$checkout/Cargo.toml"
  cargo test --doc --manifest-path "$checkout/Cargo.toml"

  cargo build --release --manifest-path "$ROOT/Cargo.toml"
  write_contract "$name" "$checkout" "$output" "$config"
  "$NIGHTMARE_BIN" run "$config" --json >"$result_json"
  assert_run_result "$result_json"
}

write_contract() {
  local name="$1"
  local source="$2"
  local output="$3"
  local config="$4"

  cat >"$config" <<TOML
schema_version = 1
source = "$source"
output = "$output"
profile = "balanced"
intensity = 7
selected_paths = ["src"]

[owner]
name = "Nightmare OSS Smoke"

[project]
name = "$name"

[checks]
verify_metadata = true
build = "cargo test --all-targets && cargo test --doc"

[features]
dead_code = true
flatten_control_flow = true
encrypt_strings = false
rename_identifiers = true
TOML
}

assert_run_result() {
  local result_json="$1"
  python3 - "$result_json" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
result = json.loads(path.read_text())
if result.get("schema_version") != 1:
    raise SystemExit(f"unexpected schema_version: {result.get('schema_version')!r}")
if result.get("status") != "passed":
    raise SystemExit(f"nightmare run did not pass: {result.get('status')!r}")

stages = {stage.get("name"): stage for stage in result.get("stages", [])}
for name in ("obfuscate", "verify", "build"):
    stage = stages.get(name)
    if stage is None:
        raise SystemExit(f"missing stage: {name}")
    if stage.get("status") != "passed":
        raise SystemExit(f"stage {name} did not pass: {stage!r}")

manifest = result.get("manifest_path")
if not manifest or not pathlib.Path(manifest).exists():
    raise SystemExit(f"missing manifest_path: {manifest!r}")
PY
}

case "${1:-}" in
  --list)
    list_targets
    ;;
  --target)
    if [[ $# -ne 2 ]]; then
      usage
      exit 2
    fi
    run_target "$(find_target "$2")"
    ;;
  --all)
    for target in "${TARGETS[@]}"; do
      run_target "$target"
    done
    ;;
  *)
    usage
    exit 2
    ;;
esac
