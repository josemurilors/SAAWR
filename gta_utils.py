import configparser
import shutil
from pathlib import Path

ENCODING = "cp1252"

BEGIN_MARKER = "# BEGIN GENERATED VEHICLES"
END_MARKER = "# END GENERATED VEHICLES"

BEGIN_WEAPON_MARKER = "# BEGIN GENERATED WEAPONS"
END_WEAPON_MARKER = "# END GENERATED WEAPONS"


def get_config():
    config = configparser.ConfigParser()
    config_path = Path(__file__).parent / "config.ini"
    if not config_path.exists():
        print(f"  Erro: config.ini não encontrado em {config_path}")
        print("  Copie config.ini.example para config.ini e ajuste os caminhos.")
        exit(1)
    config.read(config_path, encoding=ENCODING)
    return config


def get_game_paths():
    config = get_config()
    game_dir = Path(config.get("paths", "game_dir"))
    modloader_vehicles = game_dir / config.get("paths", "modloader_vehicles")
    modloader_weapons = game_dir / config.get("paths", "modloader_weapons")
    fastman92_dir = game_dir / config.get("paths", "fastman92_dir")
    game_data = game_dir / config.get("paths", "game_data")
    weapon_id_start = config.getint("weapon_config", "weapon_id_start")
    vehicle_id_start = config.getint("weapon_config", "vehicle_id_start")
    weapon_config_file = fastman92_dir / config.get("weapon_config", "weapon_config_file")
    return {
        "game_dir": game_dir,
        "modloader_vehicles": modloader_vehicles,
        "modloader_weapons": modloader_weapons,
        "fastman92_dir": fastman92_dir,
        "game_data": game_data,
        "weapon_id_start": weapon_id_start,
        "vehicle_id_start": vehicle_id_start,
        "weapon_config_file": weapon_config_file,
        "audio_cfg": fastman92_dir / "data" / "gtasa_vehicleAudioSettings.cfg",
        "weapon_loader": modloader_weapons / "WeaponLoader.txt",
        "weapons_ide": modloader_weapons / "Weapons.ide",
    }


def safe_write(path: Path, lines: list[str]) -> None:
    path = Path(path)
    if path.exists():
        bak = path.with_suffix(path.suffix + ".bak")
        shutil.copy2(path, bak)
    path.write_text("".join(lines), encoding=ENCODING)


def read_lines(path: Path) -> list[str]:
    return Path(path).read_text(encoding=ENCODING).splitlines(keepends=True)
