use std::path::Path;

/// Game files (Rockstar, fastman92) that are never committed.
const GAME_FIXTURES: [&str; 11] = [
    "handling.cfg",
    "vehicles.ide",
    "carcols.dat",
    "cargrp.dat",
    "default.ide",
    "weapon.dat",
    "object.dat",
    "gtasa_vehicleAudioSettings.cfg",
    "gtasa_weapon_config.dat",
    "fla_original.ini",
    "fla_expected_sync_one_car.ini",
];

/// False (and the test returns early) when `scripts/fixtures.sh` was not run,
/// e.g. on CI, which has no game install.
pub fn have_game_fixtures() -> bool {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures"));
    let ok = GAME_FIXTURES.iter().all(|f| dir.join(f).is_file());
    if !ok {
        eprintln!("pulado: fixtures do jogo ausentes (rode scripts/fixtures.sh)");
    }
    ok
}
