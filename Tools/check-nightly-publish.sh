#!/usr/bin/env bash
# Run the nightly.yml publish job in a sandbox on Linux, with dummy artifacts
# and a realistic fake gh, to verify the release/cask logic end to end.
set -euo pipefail

cd "$(dirname "$0")/.."

work=$(mktemp -d "/tmp/pitex-nightly-publish.XXXXXX")
trap 'rm -rf -- "$work"' EXIT

export PITEX_TEST_DIR="$work"
export PITEX_GH_LOG="$work/gh.log"
export VERSION="1.0.2-nightly.202610060000"
export SHA="abcdef1234567890abcdef1234567890abcdef12"
export GITHUB_REPOSITORY="jaehwan-2ee/pitex"
export GH_TOKEN="fake-token"
export HOMEBREW_TAP_TOKEN="fake-cask-token"
export RUNNER_TEMP="$work/tmp"

mkdir -p "$work/bin" "$work/dist" "$work/Tools" "$work/Casks" "$work/tmp"

# Dummy installer script (content does not matter for this check)
cat > "$work/Tools/install-deb.sh" <<'EOF'
#!/bin/sh
echo install-pitex-deb.sh
EOF
chmod +x "$work/Tools/install-deb.sh"

# Dummy artifacts
for a in \
  "Pitex-Nightly-${VERSION}-macos-arm64.dmg" \
  "Pitex-Nightly-${VERSION}-ubuntu24.04-amd64.deb" \
  "Pitex-Nightly-${VERSION}-ubuntu22.04-amd64.deb" \
  "Pitex-Nightly-${VERSION}-windows-amd64.zip" \
  "Pitex-Nightly-${VERSION}-windows-setup.exe"; do
  echo "artifact $a" > "$work/dist/$a"
done

# Existing-cask fixture for the "existing cask" test
cat > "$work/pitex-nightly-existing.rb" <<'EOF'
cask "pitex@nightly" do
  version "0.0.0-old"
  sha256 "OLD_SHA256"
  url "https://github.com/jaehwan-2ee/pitex/releases/download/nightly/Pitex-Nightly-#{version}-macos-arm64.dmg"
  name "Pitex Nightly"
  desc "Native LaTeX environment"
  homepage "https://github.com/jaehwan-2ee/pitex"
  depends_on arch: :arm64
  depends_on macos: :sequoia
  app "Pitex Nightly.app"
  zap trash: [
    "~/Library/Application Support/Pitex Nightly",
    "~/Library/Caches/Pitex Nightly",
    "~/Library/Caches/app.pitex.desktop.nightly",
    "~/Library/Preferences/app.pitex.desktop.nightly.plist",
  ]
end
EOF

NOT_FOUND_404='{"message":"Not Found","documentation_url":"https://docs.github.com/rest/reference/repos","status":"404"}'

cat > "$work/bin/gh" <<'EOF'
#!/usr/bin/env bash
echo "$@" >> "$PITEX_GH_LOG"
mode=${PITEX_NIGHTLY_TEST_MODE:-new}
cmd="$*"

if [ "$cmd" = "release view nightly" ]; then
  if [ "$mode" = "existing" ]; then
    exit 0
  else
    echo "$NOT_FOUND_404"
    exit 1
  fi
fi

if [ "$cmd" = "release view nightly --json assets -q .assets[].name" ]; then
  if [ "$mode" = "existing" ]; then
    echo "Pitex-Nightly-${VERSION}-macos-arm64.dmg"
    echo "stale-old-asset.exe"
  fi
  exit 0
fi

if [[ "$cmd" == "release create nightly"* || "$cmd" == "release edit nightly"* || "$cmd" == "release upload nightly"* || "$cmd" == "release delete-asset nightly"* ]]; then
  exit 0
fi

if [[ "$cmd" == *"git/ref/tags/nightly"* ]]; then
  if [ "$mode" = "existing" ]; then
    echo '{"object":{"sha":"OLD_TAG_SHA"}}'
    exit 0
  else
    echo "$NOT_FOUND_404"
    exit 1
  fi
fi

if [[ "$cmd" == *"git/refs/tags/nightly"* && "$cmd" == *"-X PATCH"* ]]; then
  # PATCH must use typed boolean -F, never raw string -f force=...
  if [[ "$cmd" == *"-f force="* ]]; then
    echo "ERROR: PATCH force must use -F, not -f" >&2
    exit 1
  fi
  if [[ "$cmd" != *"-F force=true"* ]]; then
    echo "ERROR: PATCH missing -F force=true" >&2
    exit 1
  fi
  exit 0
fi

if [[ "$cmd" == *"git/refs"* && "$cmd" == *"-X POST"* ]]; then
  exit 0
fi

if [[ "$cmd" == *"homebrew-tap/contents/Casks/pitex@nightly.rb"* && "$cmd" == *"-q .sha"* ]]; then
  if [ "$mode" = "existing" ]; then
    echo "1111111111111111111111111111111111111111"
    exit 0
  else
    echo "$NOT_FOUND_404"
    exit 1
  fi
fi

if [[ "$cmd" == *"homebrew-tap/contents/Casks/pitex@nightly.rb"* && "$cmd" == *"-q .content"* ]]; then
  if [ "$mode" = "existing" ]; then
    base64 -w0 "$PITEX_TEST_DIR/pitex-nightly-existing.rb"
    exit 0
  else
    echo "$NOT_FOUND_404"
    exit 1
  fi
fi

if [[ "$cmd" == *"homebrew-tap/contents/Casks/pitex@nightly.rb"* && "$cmd" == *"-X PUT"* ]]; then
  for arg in "$@"; do
    case "$arg" in
      content=*)
        content="${arg#content=}"
        printf '%s' "$content" | base64 -d > "$PITEX_TEST_DIR/Casks/pitex@nightly.rb"
        ;;
    esac
  done
  exit 0
fi

echo '{}'
exit 0
EOF
chmod +x "$work/bin/gh"
export PATH="$work/bin:$PATH"

# Extract publish job run blocks from nightly.yml
ruby -ryaml -e '
  data = YAML.load_file(".github/workflows/nightly.yml")
  data["jobs"]["publish"]["steps"].each_with_index do |step, idx|
    next unless step["run"]
    path = File.join(ENV["PITEX_TEST_DIR"], sprintf("run-%02d.sh", idx))
    File.write(path, "#!/usr/bin/env bash\nset -euo pipefail\n" + step["run"])
    File.chmod(0755, path)
  end
'

run_mode() {
  local mode="$1"
  local prev_sha="$2"
  export PITEX_NIGHTLY_TEST_MODE="$mode"
  export PREV_SHA="$prev_sha"
  > "$PITEX_GH_LOG"
  for script in "$work"/run-*.sh; do
    (cd "$work" && bash "$script")
  done
}

assert_contains() {
  local file="$1"
  local needle="$2"
  local msg="$3"
  grep -qF -- "$needle" "$file" || { echo "FAIL: $msg" >&2; exit 1; }
}

assert_count() {
  local file="$1"
  local needle="$2"
  local expected="$3"
  local msg="$4"
  local got
  got=$(grep -cF -- "$needle" "$file" || true)
  [ "$got" -eq "$expected" ] || { echo "FAIL: $msg (expected $expected, got $got)" >&2; exit 1; }
}

no_forbidden_output() {
  if grep -rE 'Not Found|"message"' "$work/dist" "$work/tmp" >/dev/null 2>&1; then
    echo "FAIL: found 404/error text in dist or notes" >&2
    exit 1
  fi
}

check_common() {
  # SHA256SUMS lists every artifact plus the installer script
  assert_contains "$work/dist/SHA256SUMS" "install-pitex-deb.sh" "SHA256SUMS missing installer"
  for a in \
    "Pitex-Nightly-${VERSION}-macos-arm64.dmg" \
    "Pitex-Nightly-${VERSION}-ubuntu24.04-amd64.deb" \
    "Pitex-Nightly-${VERSION}-ubuntu22.04-amd64.deb" \
    "Pitex-Nightly-${VERSION}-windows-amd64.zip" \
    "Pitex-Nightly-${VERSION}-windows-setup.exe"; do
    assert_contains "$work/dist/SHA256SUMS" "$a" "SHA256SUMS missing $a"
  done

  # nightly.json has version and commit
  [ "$(jq -r .version "$work/dist/nightly.json")" = "$VERSION" ] || { echo "FAIL: nightly.json version" >&2; exit 1; }
  [ "$(jq -r .commit "$work/dist/nightly.json")" = "$SHA" ] || { echo "FAIL: nightly.json commit" >&2; exit 1; }

  # notes live outside dist and never contain 404/error text
  [ -f "$work/tmp/nightly-notes.md" ] || { echo "FAIL: nightly-notes.md missing" >&2; exit 1; }
  no_forbidden_output

  # nightly-notes.md is not uploaded as part of dist/*
  upload_line=$(grep "release upload" "$PITEX_GH_LOG" | head -1 || true)
  if grep -qF "nightly-notes.md" <<<"$upload_line"; then
    echo "FAIL: nightly-notes.md was uploaded with dist assets" >&2
    exit 1
  fi

  # tag moved via the git/refs API (POST for new, PATCH for existing)
  if [ "$PITEX_NIGHTLY_TEST_MODE" = "existing" ]; then
    assert_contains "$PITEX_GH_LOG" "git/refs/tags/nightly" "tag PATCH"
    assert_contains "$PITEX_GH_LOG" "-F force=true" "PATCH uses typed -F force=true"
  else
    assert_contains "$PITEX_GH_LOG" "git/refs" "tag POST"
  fi
  if grep -qF -- "-f force=true" "$PITEX_GH_LOG"; then
    echo "FAIL: found raw string -f force=true in gh log" >&2
    exit 1
  fi

  # uploads happen before any delete-asset
  local upload_line delete_line
  upload_line=$(grep -n "release upload" "$PITEX_GH_LOG" | head -1 | cut -d: -f1)
  delete_line=$(grep -n "release delete-asset" "$PITEX_GH_LOG" | head -1 | cut -d: -f1 || echo 9999)
  [ "$upload_line" -lt "$delete_line" ] || { echo "FAIL: upload not before delete" >&2; exit 1; }

  # cask PUT happened and the generated cask is valid
  assert_contains "$PITEX_GH_LOG" "homebrew-tap/contents/Casks/pitex@nightly.rb" "cask API call"
  ruby -c "$work/Casks/pitex@nightly.rb" >/dev/null || { echo "FAIL: cask ruby syntax" >&2; exit 1; }
  assert_contains "$work/Casks/pitex@nightly.rb" "version \"${VERSION}\"" "cask version"
  assert_contains "$work/Casks/pitex@nightly.rb" "app \"Pitex Nightly.app\"" "cask app"
  assert_contains "$work/Casks/pitex@nightly.rb" "Pitex-Nightly-#{version}-macos-arm64.dmg" "cask url template"
  if grep -qE 'VERSION_PLACEHOLDER|SHA256_PLACEHOLDER' "$work/Casks/pitex@nightly.rb"; then
    echo "FAIL: cask contains placeholders" >&2; exit 1
  fi
}

echo "=== new release / new cask ==="
run_mode new ""
check_common
assert_count "$PITEX_GH_LOG" "release create nightly" 1 "release create"
assert_count "$PITEX_GH_LOG" "release delete-asset" 0 "no delete in new mode"

# First-run cask PUT must not include a sha field (creating a new file).
put_line=$(grep -F -- "-X PUT" "$PITEX_GH_LOG" | grep -F -- "pitex@nightly.rb" || true)
[ -n "$put_line" ] || { echo "FAIL: no cask PUT" >&2; exit 1; }
if grep -qF -- "-f sha=" <<<"$put_line"; then
  echo "FAIL: new cask PUT should not set sha" >&2
  exit 1
fi

# No previous nightly means no compare link in notes
if grep -qF "compare/" "$work/tmp/nightly-notes.md"; then
  echo "FAIL: new release notes should not have a compare link" >&2
  exit 1
fi

echo "=== existing release / existing cask ==="
run_mode existing "1111111111111111111111111111111111111111"
check_common
assert_count "$PITEX_GH_LOG" "release edit nightly" 1 "release edit"
assert_count "$PITEX_GH_LOG" "release delete-asset nightly stale-old-asset.exe" 1 "stale asset deleted"
assert_count "$PITEX_GH_LOG" "release delete-asset" 1 "only one delete"
assert_contains "$work/tmp/nightly-notes.md" "https://github.com/${GITHUB_REPOSITORY}/compare/1111111111111111111111111111111111111111...${SHA}" "notes compare link"

# Existing-cask PUT must include the current blob sha
put_line=$(grep -F -- "-X PUT" "$PITEX_GH_LOG" | grep -F -- "pitex@nightly.rb" || true)
if ! grep -qF -- "-f sha=" <<<"$put_line"; then
  echo "FAIL: existing cask PUT should set sha" >&2
  exit 1
fi

echo "check-nightly-publish OK"
