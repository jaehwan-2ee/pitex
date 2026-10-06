#!/bin/sh
# Run only in a disposable Ubuntu container. No host package changes are allowed.
set -eu
if [ "${PITEX_APT_ISOLATED_TEST:-}" != 1 ] || [ ! -f /.dockerenv ] || [ "$(id -u)" -ne 0 ]; then
    printf 'Run this check as root in a disposable Ubuntu container with PITEX_APT_ISOLATED_TEST=1.\n' >&2
    exit 2
fi
installer=${1:-/src/Tools/install-deb.sh}
root=$(mktemp -d /tmp/pitex-deb-check.XXXXXXXXXX)
trap 'rm -rf -- "$root"' 0
trap 'exit 130' INT
trap 'exit 143' HUP TERM
mkdir -p "$root/home/Downloads folder's 한글" "$root/home/private-temp" "$root/package/DEBIAN" "$root/package/usr/share/pitex-apt-test" "$root/bin"
printf 'Disposable installer fixture.\n' > "$root/package/usr/share/pitex-apt-test/data"
chmod 700 "$root/home" "$root/home/Downloads folder's 한글" "$root/home/private-temp"
export TMPDIR="$root/home/private-temp"
export PITEX_DEB_TEST_STAGE="$root/stage-path" PITEX_DEB_TEST_SOURCE="$root/home/Downloads folder's 한글/Pitex.deb"
for version in 0.0.1 0.0.2; do
    cat > "$root/package/DEBIAN/control" <<EOF
Package: pitex
Version: $version
Architecture: all
Maintainer: Pitex Test <test@example.invalid>
Description: Disposable APT installer validation package
EOF
    dpkg-deb --build "$root/package" "$PITEX_DEB_TEST_SOURCE" >/dev/null
    chmod 600 "$PITEX_DEB_TEST_SOURCE"
    if [ "$version" = 0.0.1 ]; then
        if runuser -u _apt -- test -r "$PITEX_DEB_TEST_SOURCE"; then
            printf 'The control package must be unreadable to _apt.\n' >&2
            exit 1
        fi
        # Piped APT output defaults to quiet=1 and hides notices. Show them here.
        apt -o quiet=0 -o Debug::pkgAcquire::Worker=1 install -y "$PITEX_DEB_TEST_SOURCE" >"$root/direct.log" 2>&1
        if ! grep -q 'Download is performed unsandboxed as root' "$root/direct.log"; then
            cat "$root/direct.log" >&2
            exit 1
        fi
    fi
done

# Observe the real APT input, then run the distribution's unmodified APT.
cat > "$root/bin/apt" <<'EOF'
#!/bin/sh
set -eu
for package do :; done
case "$package" in /tmp/pitex-install.*/pitex.deb) ;; *) exit 41;; esac
test "$(stat -c %a "${package%/*}")" = 755
test "$(stat -c %a "$package")" = 644
cmp "$PITEX_DEB_TEST_SOURCE" "$package"
runuser -u _apt -- test -r "$package"
apt-config dump | grep -q '^APT::Sandbox::User "_apt";'
printf '%s\n' "$package" > "$PITEX_DEB_TEST_STAGE"
if [ "${PITEX_DEB_TEST_FAIL_APT:-}" = 1 ]; then exit 53; fi
exec /usr/bin/apt -o quiet=0 -o Debug::pkgAcquire::Worker=1 "$@"
EOF
chmod 755 "$root/bin/apt"
export PATH="$root/bin:$PATH"
before=$(sha256sum "$PITEX_DEB_TEST_SOURCE")
printf 'y\n' | sh "$installer" "$PITEX_DEB_TEST_SOURCE" >"$root/install.log" 2>&1
test "$(dpkg-query -W -f='${Status} ${Version}' pitex)" = 'install ok installed 0.0.2'
if grep -qi 'unsandboxed' "$root/install.log"; then
    cat "$root/install.log" >&2
    exit 1
fi
grep -q 'APT::Sandbox::User=_apt' "$root/install.log"
if grep -q 'Binary::file::APT::Sandbox::User=root\|Binary::copy::APT::Sandbox::User=root' "$root/install.log"; then
    printf 'APT disabled its file acquisition sandbox.\n' >&2
    exit 1
fi
test ! -e "$(cat "$PITEX_DEB_TEST_STAGE")"
test "$before" = "$(sha256sum "$PITEX_DEB_TEST_SOURCE")"
test "$(stat -c %a "$root/home")" = 700
test "$(stat -c %a "$root/home/Downloads folder's 한글")" = 700
test "$(stat -c %a "$PITEX_DEB_TEST_SOURCE")" = 600

# Exercise the normal user's sudo argument path. APT still runs for real in
# this disposable root container; the two small shims only select that branch.
cat > "$root/bin/id" <<'EOF'
#!/bin/sh
if [ "$*" = -u ]; then printf '1000\n'; else exec /usr/bin/id "$@"; fi
EOF
cat > "$root/bin/sudo" <<'EOF'
#!/bin/sh
printf '%s\n' "$*" > "$PITEX_DEB_TEST_STAGE.sudo"
exec "$@"
EOF
chmod 755 "$root/bin/id" "$root/bin/sudo"
printf 'y\n' | sh "$installer" "$PITEX_DEB_TEST_SOURCE" >"$root/sudo.log" 2>&1
grep -q '^apt install -- /tmp/pitex-install\.' "$PITEX_DEB_TEST_STAGE.sudo"
test ! -e "$(cat "$PITEX_DEB_TEST_STAGE")"

export PITEX_DEB_TEST_FAIL_APT=1
if sh "$installer" "$PITEX_DEB_TEST_SOURCE" >"$root/failure.log" 2>&1; then
    printf 'The installer did not propagate APT failure.\n' >&2
    exit 1
else
    test "$?" -eq 53
fi
test ! -e "$(cat "$PITEX_DEB_TEST_STAGE")"
printf 'PASS: %s; private Downloads/TMPDIR; sandbox user _apt; no notice; sudo arguments; source unchanged; cleanup on success and failure.\n' "$(/usr/bin/apt --version)"
