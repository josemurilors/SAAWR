#!/usr/bin/env bash
# Compiles the CLEO script of the clothes catalog (extras/cleo/awr-roupas.sc) with gta3sc
# (https://github.com/thelink2012/gta3sc, MIT; release 0.9.8 was used). The result,
# extras/cleo/awr-roupas.cs, is committed: the app embeds it, so building the app does not
# need gta3sc. --guesser is only there because the gtasa profile turns on -fswitch/-farrays/
# -fconst, which gta3sc keeps behind it; the script uses none of them.
# Usage: scripts/build-cleo.sh <pasta do gta3sc, a que tem gta3sc.exe e config/>
set -euo pipefail

G3="${1:?uso: scripts/build-cleo.sh <pasta do gta3sc>}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

"$G3/gta3sc.exe" compile --config=gtasa --cs --guesser \
  "$ROOT/extras/cleo/awr-roupas.sc" -o "$ROOT/extras/cleo/awr-roupas.cs"
echo "extras/cleo/awr-roupas.cs pronto ($(wc -c < "$ROOT/extras/cleo/awr-roupas.cs") bytes)"
