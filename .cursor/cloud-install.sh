#!/usr/bin/env bash
# Idempotent Cloud Agent bootstrap for the Linux pitex workspace.
# Toolchains.lock.json pins Node and npm. It does not pin Rust.
# CI installs rustup stable for Linux/; PreviewEngine/rust-toolchain.toml
# pins nightly-2026-10-04 for the embedded helpers.
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive
export CARGO_HOME=/usr/local/cargo
export RUSTUP_HOME=/usr/local/rustup
export PATH="${CARGO_HOME}/bin:/usr/local/bin:${PATH}"

NODE_VERSION=24.15.0
NPM_VERSION=12.0.2
NODE_PREFIX=/usr/local/lib/nodejs

sudo apt-get update
sudo apt-get install -y \
  -o Dpkg::Options::=--force-confdef \
  -o Dpkg::Options::=--force-confold \
  build-essential pkg-config libssl-dev \
  libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev \
  libvte-2.91-gtk4-dev libpoppler-glib-dev libwebkitgtk-6.0-dev \
  libteckit-dev libharfbuzz-dev libgraphite2-dev libicu-dev \
  libfreetype-dev libfontconfig-dev libpng-dev zlib1g-dev \
  xvfb gdb dbus-x11 \
  ca-certificates curl xz-utils ruby

if ! "${NODE_PREFIX}/bin/node" -v 2>/dev/null | grep -qx "v${NODE_VERSION}"; then
  tmp="$(mktemp)"
  curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz" -o "${tmp}"
  sudo mkdir -p "${NODE_PREFIX}"
  sudo tar -xJf "${tmp}" -C "${NODE_PREFIX}" --strip-components=1 --no-same-owner
  rm -f "${tmp}"
fi

for cmd in node npm npx corepack; do
  sudo ln -sfn "${NODE_PREFIX}/bin/${cmd}" "/usr/local/bin/${cmd}"
  ln -sfn "${NODE_PREFIX}/bin/${cmd}" "${CARGO_HOME}/bin/${cmd}"
done

if ! /usr/local/bin/npm -v 2>/dev/null | grep -qx "${NPM_VERSION}"; then
  sudo /usr/local/bin/npm install -g "npm@${NPM_VERSION}"
fi

if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain none --profile minimal --no-modify-path
fi

rustup toolchain install stable --profile minimal
rustup component add rustfmt clippy --toolchain stable
rustup default stable
rustup toolchain install nightly-2026-10-04 --profile minimal

echo "node $(/usr/local/bin/node -v) npm $(/usr/local/bin/npm -v)"
rustup show
