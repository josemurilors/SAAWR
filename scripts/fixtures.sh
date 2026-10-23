#!/usr/bin/env bash
# Copies the game-file fixtures from a local GTA SA install (they are Rockstar /
# fastman92 files and never go to Git) and builds the FLA golden file with sed,
# independently of the Rust code it checks.
# Usage: scripts/fixtures.sh "/d/Games/GTA San Andreas"
set -euo pipefail

GTA="${1:?uso: scripts/fixtures.sh <pasta do GTA SA>}"
OUT="$(cd "$(dirname "$0")/.." && pwd)/fixtures"
FLA_DIR="$(dirname "$(find "$GTA" -maxdepth 3 -iname '*fastman92limitAdjuster*.asi' | head -1)")"

for f in handling.cfg vehicles.ide carcols.dat cargrp.dat default.ide weapon.dat object.dat; do
  cp "$GTA/data/$f" "$OUT/"
done
cp "$FLA_DIR/data/gtasa_vehicleAudioSettings.cfg" "$FLA_DIR/data/gtasa_weapon_config.dat" "$OUT/"
cp "$FLA_DIR/fastman92limitAdjuster_GTASA.ini" "$OUT/fla_original.ini"

# expected sync for one car at ID 12500 (see fla::tests::golden_one_car_sync)
sed -b \
  -e 's/^#Count of killable model IDs = 800\r$/Count of killable model IDs = 12550\r/' \
  -e 's/^#Apply handling.cfg patch = 0\r$/Apply handling.cfg patch = 1\r/' \
  -e 's/^#Number of standard lines = 210\r$/Number of standard lines = 300\r/' \
  -e 's/^#Number of bike lines = 13\r$/Number of bike lines = 50\r/' \
  -e 's/^#Number of flying lines = 24\r$/Number of flying lines = 50\r/' \
  -e 's/^#Number of boat lines = 12\r$/Number of boat lines = 50\r/' \
  -e 's/^#Enable vehicle audio loader = 0\r$/Enable vehicle audio loader = 1\r/' \
  -e 's/^#Make paintjobs work for any ID = 0\r$/Make paintjobs work for any ID = 1\r/' \
  -e 's/^#Apply ID limit patch = 0\r$/Apply ID limit patch = 1\r/' \
  -e 's/^#Enable model special feature loader = 0\r$/Enable model special feature loader = 1\r/' \
  "$OUT/fla_original.ini" > "$OUT/fla_expected_sync_one_car.ini"

echo "fixtures prontas em $OUT"
