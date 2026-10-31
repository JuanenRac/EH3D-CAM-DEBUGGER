#!/usr/bin/env bash
# Runs eh3d-camdebug with its sample. Extra arguments replace the sample: ./run.sh [ARGUMENTS]
set -u
cd "$(dirname "$0")"
pause() { [ -n "${NOPAUSE:-}" ] || { read -r -p "Press Enter to close..." _ || true; }; }
trap pause EXIT
if [ "$#" -eq 0 ]; then set -- check examples/sample.nc --table 100,80 --clamp 20,-5,30,5,6; fi
cargo run --release --quiet -- "$@"
