#!/usr/bin/env bash
# Run the nightly.yml gate job with a fake gh to verify CI/run logic.
set -euo pipefail

cd "$(dirname "$0")/.."

work=$(mktemp -d "/tmp/pitex-nightly-gate.XXXXXX")
trap 'rm -rf -- "$work"' EXIT

export PITEX_TEST_DIR="$work"
export PITEX_TEST_SHA="abcdef1234567890abcdef1234567890abcdef12"
export PITEX_GH_LOG="$work/gh.log"
export GITHUB_OUTPUT="$work/github_output"
export GITHUB_REPOSITORY="jaehwan-2ee/pitex"
export PUBLISH_INPUT="true"
export PATH="$work/bin:$PATH"

mkdir -p "$work/bin"

cat > "$work/bin/git" <<'EOF'
#!/bin/sh
echo "$PITEX_TEST_SHA"
EOF
chmod +x "$work/bin/git"

cat > "$work/bin/gh" <<'EOF'
#!/usr/bin/env bash
echo "$@" >> "$PITEX_GH_LOG"
cmd="$*"

# Loudly reject the old buggy parameter so it can never slip back in.
if [[ "$cmd" == *"actions/workflows/ci.yml/runs"* && "$cmd" == *"commit_sha="* ]]; then
  echo "ERROR: gate used commit_sha= instead of head_sha=" >&2
  exit 1
fi

if [[ "$cmd" == *"git/ref/tags/nightly"* ]]; then
  echo "${PITEX_TAG_SHA:-OLD_SHA}"
  exit 0
fi

if [[ "$cmd" == *"actions/workflows/ci.yml/runs"* && "$cmd" == *"head_sha="* ]]; then
  # Extract the head_sha value from the URL to validate it matches PITEX_TEST_SHA.
  param="${cmd#*head_sha=}"
  head_sha="${param%%&*}"
  if [ "$head_sha" != "$PITEX_TEST_SHA" ]; then
    echo '{"workflow_runs":[]}'
    exit 0
  fi

  mode="${PITEX_GATE_MODE:-no_run}"
  case "$mode" in
    no_run)
      echo '{"workflow_runs":[]}'
      ;;
    in_progress)
      echo "{\"workflow_runs\":[{\"id\":123,\"status\":\"in_progress\",\"conclusion\":null,\"head_sha\":\"$head_sha\"}]}"
      ;;
    failure)
      echo "{\"workflow_runs\":[{\"id\":123,\"status\":\"completed\",\"conclusion\":\"failure\",\"head_sha\":\"$head_sha\"}]}"
      ;;
    success)
      echo "{\"workflow_runs\":[{\"id\":123,\"status\":\"completed\",\"conclusion\":\"success\",\"head_sha\":\"$head_sha\"}]}"
      ;;
    success_wrong_sha)
      # A successful run for a different SHA must not be accepted.
      echo '{"workflow_runs":[{"id":123,"status":"completed","conclusion":"success","head_sha":"WRONG_SHA_123"}]}'
      ;;
  esac
  exit 0
fi

echo '{}'
exit 0
EOF
chmod +x "$work/bin/gh"

ruby -ryaml -e '
  data = YAML.load_file(".github/workflows/nightly.yml")
  run = data["jobs"]["gate"]["steps"].find { |s| s["id"] == "compute" }["run"]
  run = run.gsub("${{ inputs.force }}", "${INPUT_FORCE:-false}")
  run = run.gsub("${{ github.event.inputs.publish || '\''true'\'' }}", "${PUBLISH_INPUT:-true}")
  run = run.gsub("SHA=$(git rev-parse HEAD)", "SHA=${PITEX_TEST_SHA}")
  path = File.join(ENV["PITEX_TEST_DIR"], "gate.sh")
  File.write(path, "#!/usr/bin/env bash\nset -euo pipefail\n" + run)
  File.chmod(0755, path)
'

gate_output() { grep -E "^$1=" "$GITHUB_OUTPUT" | cut -d= -f2- || true; }

run_gate() {
  local mode="$1"
  export PITEX_GATE_MODE="$mode"
  > "$GITHUB_OUTPUT"
  bash "$work/gate.sh" 2>&1
}

echo "=== no CI run for this SHA ==="
out=$(run_gate no_run)
[ -z "$(gate_output version)" ] || { echo "FAIL: expected no version, got $(gate_output version)" >&2; exit 1; }
echo "$out" | grep -q "No successful CI run" || { echo "FAIL: missing no-run warning" >&2; exit 1; }
echo "$out"

echo "=== newest CI run still in_progress ==="
out=$(run_gate in_progress)
[ -z "$(gate_output version)" ] || { echo "FAIL: expected no version for in_progress" >&2; exit 1; }
echo "$out" | grep -q "No successful CI run" || { echo "FAIL: missing in_progress warning" >&2; exit 1; }
echo "$out"

echo "=== newest CI run failed ==="
out=$(run_gate failure)
[ -z "$(gate_output version)" ] || { echo "FAIL: expected no version for failure" >&2; exit 1; }
echo "$out" | grep -q "No successful CI run" || { echo "FAIL: missing failure warning" >&2; exit 1; }
echo "$out"

echo "=== successful CI run for a different SHA ==="
out=$(run_gate success_wrong_sha)
[ -z "$(gate_output version)" ] || { echo "FAIL: expected no version for wrong sha, got $(gate_output version)" >&2; exit 1; }
echo "$out"

echo "=== successful CI run for this SHA ==="
out=$(run_gate success)
version=$(gate_output version)
[[ "$version" =~ ^1\.0\.2-nightly\.[0-9]{12}$ ]] || { echo "FAIL: unexpected version format: $version" >&2; exit 1; }
[ "$(gate_output sha)" = "$PITEX_TEST_SHA" ] || { echo "FAIL: sha output mismatch" >&2; exit 1; }
echo "$out"
echo "version: $version"

echo "=== nightly tag already at SHA, force=false ==="
export PITEX_TAG_SHA="$PITEX_TEST_SHA"
export INPUT_FORCE="false"
out=$(run_gate success)
[ -z "$(gate_output version)" ] || { echo "FAIL: expected skip when tag at sha and not forced" >&2; exit 1; }
echo "$out" | grep -q "Current nightly already at" || { echo "FAIL: missing already-at warning" >&2; exit 1; }
echo "$out"

echo "=== nightly tag already at SHA, force=true ==="
export INPUT_FORCE="true"
out=$(run_gate success)
version=$(gate_output version)
[[ "$version" =~ ^1\.0\.2-nightly\.[0-9]{12}$ ]] || { echo "FAIL: unexpected forced version: $version" >&2; exit 1; }
echo "$out"
echo "version: $version"

echo "check-nightly-gate OK"
