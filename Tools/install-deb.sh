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

PITEX_DEB_PACKAGE=$(dpkg-deb --field "$1" Package)
case "$PITEX_DEB_PACKAGE" in
    pitex|pitex-nightly) ;;
    *)
        printf 'The package is not a Pitex package (Package: %s).\n' "$PITEX_DEB_PACKAGE" >&2
        exit 1
        ;;
esac

# Do not use TMPDIR: it can point inside a private home folder.
staging=$(mktemp -d "/tmp/$PITEX_DEB_PACKAGE-install.XXXXXXXXXX")
trap 'rm -rf -- "$staging"' 0
trap 'exit 130' INT
trap 'exit 143' HUP TERM
cp -- "$1" "$staging/$PITEX_DEB_PACKAGE.deb"
chmod 644 "$staging/$PITEX_DEB_PACKAGE.deb"
chmod 755 "$staging"

if [ "$(id -u)" -eq 0 ]; then
    apt install -- "$staging/$PITEX_DEB_PACKAGE.deb"
else
    sudo apt install -- "$staging/$PITEX_DEB_PACKAGE.deb"
fi
