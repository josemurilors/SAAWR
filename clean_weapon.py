#!/usr/bin/env python3
"""clean_weapon.py — List installed weapons and clean residues from deleted ones.

Usage:
  python clean_weapon.py          → lista instalados e mostra resíduos
  python clean_weapon.py clean    → lista e já pergunta se quer remover
"""

import re
from pathlib import Path

from gta_utils import (
    safe_write,
    read_lines,
    ENCODING,
    BEGIN_WEAPON_MARKER,
    END_WEAPON_MARKER,
    get_game_paths,
)

# ─── Paths ───────────────────────────────────────────────────────────
_paths = get_game_paths()
BASE = _paths["modloader_weapons"]
DATA = _paths["game_data"]
WEAPON_CONFIG_FILE = _paths["weapon_config_file"]
WEAPON_LOADER = _paths["weapon_loader"]
WEAPONS_IDE = _paths["weapons_ide"]

# ─── Cores ───────────────────────────────────────────────────────────
RED = "\033[91m"
GREEN = "\033[92m"
YELLOW = "\033[93m"
CYAN = "\033[96m"
BOLD = "\033[1m"
RESET = "\033[0m"


def ok(msg):
    print(f"  {GREEN}✓{RESET} {msg}")


def warn(msg):
    print(f"  {YELLOW}⚠{RESET} {msg}")


def fail(msg):
    print(f"  {RED}✗{RESET} {msg}")


def info(msg):
    print(f"  {CYAN}→{RESET} {msg}")


# ─── Stock data ──────────────────────────────────────────────────────

def load_stock_weapon_names():
    names = set()
    default_ide = DATA / "default.ide"
    if not default_ide.exists():
        return names

    in_weap = False
    for line in Path(default_ide).read_text(encoding=ENCODING).splitlines():
        s = line.strip()
        if s.lower() == "weap":
            in_weap = True
            continue
        if in_weap and s.lower() in ("end", "tobj", "hier", "anim", "cars"):
            break
        if not s or s.startswith("#"):
            continue
        parts = [p.strip() for p in s.split(",")]
        if len(parts) >= 2:
            names.add(parts[1].lower())

    # Also load from gtasa_weapon_config.dat (uses different naming)
    if WEAPON_CONFIG_FILE.exists():
        for line in Path(WEAPON_CONFIG_FILE).read_text(encoding=ENCODING).splitlines():
            s = line.strip()
            if not s or s.startswith("#") or s.startswith("!"):
                continue
            parts = s.split()
            if len(parts) >= 2:
                names.add(parts[1].lower())

    return names


# ─── Installed weapons ──────────────────────────────────────────────

def get_installed():
    installed = []
    if not BASE.exists():
        return installed
    for folder in sorted(BASE.iterdir()):
        if not folder.is_dir():
            continue
        lf = folder / "linhas_arma.txt"
        if not lf.exists():
            continue
        model = None
        for line in Path(lf).read_text(encoding=ENCODING).splitlines():
            s = line.strip()
            if s and "," in s and not s.startswith("#"):
                parts = [x.strip() for x in s.split(",")]
                if len(parts) >= 2:
                    model = parts[1].lower()
                    break
        display = model
        if model:
            for fxt in folder.glob("*.fxt"):
                c = fxt.read_text(encoding=ENCODING).strip()
                if " " in c:
                    display = c.split(" ", 1)[1]
                    break
        installed.append({
            "folder": folder.name,
            "model": model,
            "display": display or model or "???",
        })
    return installed


# ─── Weapon config residues ─────────────────────────────────────────

def find_weapon_config_residues(installed_models, stock_names):
    if not WEAPON_CONFIG_FILE.exists():
        return [], None

    lines = read_lines(WEAPON_CONFIG_FILE)
    residues = []

    for i, line in enumerate(lines):
        s = line.strip()
        if not s or s.startswith("#") or s.startswith("!"):
            continue
        parts = s.split()
        if len(parts) >= 2:
            model = parts[1].lower()
            if model not in stock_names and model not in installed_models:
                residues.append({"idx": i, "model": model, "line": s})

    return residues, lines


def remove_weapon_config_residues(residues, lines):
    if not residues:
        return False

    # Keep only lines that are NOT residues
    keep_indices = set()
    for i, line in enumerate(lines):
        keep_indices.add(i)

    for r in residues:
        keep_indices.discard(r["idx"])

    new_lines = [lines[i] for i in sorted(keep_indices)]
    safe_write(WEAPON_CONFIG_FILE, new_lines)
    return True


# ─── WeaponLoader.txt residues ───────────────────────────────────────

def find_weapon_loader_residues(installed_models, stock_names):
    if not WEAPON_LOADER.exists():
        return [], None

    lines = read_lines(WEAPON_LOADER)
    residues = []

    for i, line in enumerate(lines):
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        # Check if this is a weapon.dat line (starts with $)
        if s.startswith("$"):
            parts = s.split()
            if len(parts) >= 2:
                model = parts[1].lower()
                if model not in stock_names and model not in installed_models:
                    residues.append({"idx": i, "model": model, "line": s})

    return residues, lines


def remove_weapon_loader_residues(residues, lines):
    if not residues:
        return False

    keep_indices = set()
    for i, line in enumerate(lines):
        keep_indices.add(i)

    for r in residues:
        keep_indices.discard(r["idx"])

    new_lines = [lines[i] for i in sorted(keep_indices)]
    safe_write(WEAPON_LOADER, new_lines)
    return True


# ─── Weapons.ide residues ────────────────────────────────────────────

def find_weapons_ide_residues(installed_models, stock_names):
    if not WEAPONS_IDE.exists():
        return [], None

    lines = read_lines(WEAPONS_IDE)
    residues = []

    in_weap = False
    for i, line in enumerate(lines):
        s = line.strip()
        if s.lower() == "weap":
            in_weap = True
            continue
        if in_weap and s.lower() in ("end",):
            in_weap = False
            continue
        if not in_weap:
            continue
        if not s or s.startswith("#"):
            continue
        parts = [p.strip() for p in s.split(",")]
        if len(parts) >= 2:
            model = parts[1].lower()
            if model not in stock_names and model not in installed_models:
                residues.append({"idx": i, "model": model, "line": s})

    return residues, lines


def remove_weapons_ide_residues(residues, lines):
    if not residues:
        return False

    keep_indices = set()
    for i, line in enumerate(lines):
        keep_indices.add(i)

    for r in residues:
        keep_indices.discard(r["idx"])

    new_lines = [lines[i] for i in sorted(keep_indices)]
    safe_write(WEAPONS_IDE, new_lines)
    return True


# ─── Main ────────────────────────────────────────────────────────────

def main():
    stock_names = load_stock_weapon_names()
    info(f"Armas originais carregadas: {len(stock_names)}")

    installed = get_installed()
    if not installed:
        warn("Nenhuma arma instalada encontrada em novas-armas/")
        return

    installed_models = {v["model"] for v in installed if v["model"]}

    print(f"\n{BOLD}Armas instaladas:{RESET}")
    print(f"  {'ID':<8} {'Modelo':<15} {'Display':<30} {'Pasta'}")
    print(f"  {'─'*8} {'─'*15} {'─'*30} {'─'*30}")
    for v in installed:
        vid = ""
        m = re.match(r"(\d+)", v["folder"])
        if m:
            vid = m.group(1)
        print(f"  {vid:<8} {v['model'] or '?':<15} {v['display']:<30} {v['folder']}")

    config_res, config_lines = find_weapon_config_residues(installed_models, stock_names)
    loader_res, loader_lines = find_weapon_loader_residues(installed_models, stock_names)
    ide_res, ide_lines = find_weapons_ide_residues(installed_models, stock_names)

    total = len(config_res) + len(loader_res) + len(ide_res)
    if total == 0:
        print(f"\n{GREEN}{BOLD}Nenhum resíduo encontrado. Tudo limpo!{RESET}")
        return

    print(f"\n{YELLOW}{BOLD}Resíduos encontrados ({total}):{RESET}")

    if config_res:
        print(f"\n  {CYAN}Config (gtasa_weapon_config.dat):{RESET}")
        for r in config_res:
            print(f"    {RED}{r['model']:<15}{RESET} → linha {r['idx']+1}")

    if loader_res:
        print(f"\n  {CYAN}WeaponLoader.txt:{RESET}")
        for r in loader_res:
            print(f"    {RED}{r['model']:<15}{RESET} → linha {r['idx']+1}")

    if ide_res:
        print(f"\n  {CYAN}Weapons.ide:{RESET}")
        for r in ide_res:
            print(f"    {RED}{r['model']:<15}{RESET} → linha {r['idx']+1}")

    ans = input(f"\n{BOLD}Remover resíduos? (s/N): {RESET}").strip().lower()
    if ans != "s":
        info("Nada foi alterado.")
        return

    if config_res and config_lines:
        remove_weapon_config_residues(config_res, config_lines)
        ok(f"{len(config_res)} entr. de config removidas")

    if loader_res and loader_lines:
        remove_weapon_loader_residues(loader_res, loader_lines)
        ok(f"{len(loader_res)} entr. de WeaponLoader removidas")

    if ide_res and ide_lines:
        remove_weapons_ide_residues(ide_res, ide_lines)
        ok(f"{len(ide_res)} entr. de Weapons.ide removidas")

    print(f"\n{GREEN}{BOLD}Resíduos removidos!{RESET}")


if __name__ == "__main__":
    main()
