#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C TZ=UTC PYTHONHASHSEED=0
exec python3 "$(dirname "${BASH_SOURCE[0]}")/Tools/benchmark-live-compile.py"
