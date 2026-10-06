#!/bin/sh
# Install a Pitex package from a private folder without disabling APT's sandbox.
set -eu

if [ "$#" -ne 1 ]; then
    printf 'Usage: sh install-deb.sh PATH-TO-PITEX.deb\n' >&2
    exit 2
fi
if [ ! -f "$1" ] || [ ! -r "$1" ]; then
    printf 'Cannot read the package: %s\n' "$1" >&2
    exit 1
fi

# Do not use TMPDIR: it can point inside a private home folder.
staging=$(mktemp -d /tmp/pitex-install.XXXXXXXXXX)
trap 'rm -rf -- "$staging"' 0
trap 'exit 130' INT
trap 'exit 143' HUP TERM
cp -- "$1" "$staging/pitex.deb"
if [ "$(dpkg-deb --field "$staging/pitex.deb" Package)" != pitex ]; then
    printf 'The package is not a Pitex package.\n' >&2
    exit 1
fi
chmod 644 "$staging/pitex.deb"
chmod 755 "$staging"

if [ "$(id -u)" -eq 0 ]; then
    apt install -- "$staging/pitex.deb"
else
    sudo apt install -- "$staging/pitex.deb"
fi
