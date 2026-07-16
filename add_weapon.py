#!/usr/bin/env python3
"""add_weapon.py — Add new weapons to GTA SA (modloader + fastman92)

Usage:
  Put this script in <GTA_DIR>/ferramentas/
  Create a folder in modloader/novas-armas/<nome-da-arma>/
  Place .dff + .txd inside (model and icon)
  Create a "Nome da Arma.txt" with the reference weapon name inside
  Run: python add_weapon.py
"""

import os
import re
import sys
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
WEAPON_ID_START = _paths["weapon_id_start"]
WEAPON_CONFIG_FILE = _paths["weapon_config_file"]

# Stock weapon IDs from default.ide (weap section)
STOCK_WEAPON_IDS = set(range(321, 374))

# ─── Terminal helpers ────────────────────────────────────────────────

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


def format_display_name(name):
    m = re.search(r"\b(\d{4})$", name)
    if m:
        year = m.group(1)[-2:]
        return name[: m.start()] + f"'{year}"
    return name


def generate_short_name(folder_name, existing_names, log=ok):
    clean = re.sub(r"[^a-zA-Z0-9]", "", folder_name)
    parts = re.split(r"[-_\s]+", folder_name)
    first_word = parts[0].lower() if parts else clean.lower()

    candidates = [first_word[:7]]

    for i in range(min(7, len(clean)), 2, -1):
        candidates.append(clean[:i].lower())

    seen = set()
    for c in candidates:
        c = c.lower()
        if c in seen:
            continue
        seen.add(c)
        if c not in existing_names and len(c) <= 7 and len(c) >= 2:
            log(f"Nome curto: {c} ({len(c)} chars)")
            return c

    for i in range(1, 1000):
        c = f"w{i}"
        if c not in existing_names:
            log(f"Nome curto: {c} (fallback)")
            return c

    return None


def get_used_ids():
    ids = set(STOCK_WEAPON_IDS)
    if not BASE.exists():
        return ids
    for f in BASE.iterdir():
        if f.is_dir():
            m = re.match(r"^(\d+)", f.name)
            if m:
                ids.add(int(m.group(1)))
    return ids


def get_next_available_id():
    used = get_used_ids()
    nid = WEAPON_ID_START
    while nid in used:
        nid += 1
    return nid


def get_used_names():
    names = set()
    ide_path = DATA / "default.ide"
    if ide_path.exists():
        for line in Path(ide_path).read_text(encoding=ENCODING).splitlines():
            line = line.strip()
            if not line or line.startswith("#") or line in ("weap", "end", "objs", "tobj", "cars", "hier", "anim"):
                continue
            parts = [p.strip() for p in line.split(",")]
            if len(parts) >= 2:
                names.add(parts[1].split()[0].lower())
    if BASE.exists():
        for folder in BASE.iterdir():
            if not folder.is_dir():
                continue
            lf = folder / "linhas_arma.txt"
            if not lf.exists():
                continue
            for line in Path(lf).read_text(encoding=ENCODING).splitlines():
                line = line.strip()
                if line and not line.startswith("#") and not line.startswith("Weapons.ide"):
                    parts = [p.strip() for p in line.split(",")]
                    if len(parts) >= 2:
                        names.add(parts[1].lower())
                    break
    return names


def find_pending_folders():
    if not BASE.exists():
        return []
    pending = []
    for f in sorted(BASE.iterdir()):
        if not f.is_dir():
            continue
        if (f / "linhas_arma.txt").exists():
            continue
        dff = list(f.glob("*.dff"))
        txt = [x for x in f.glob("*.txt") if x.name.lower() != "linhas_arma.txt"]
        if dff and txt:
            pending.append(f)
    return pending


def find_weapon_ide(model_name):
    """Find weapon in default.ide weap section."""
    model_upper = model_name.upper()
    default_ide = DATA / "default.ide"
    if not default_ide.exists():
        return None

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
        if len(parts) >= 2 and parts[1].split()[0].upper() == model_upper:
            return parts
    return None


def find_weapon_config(weapon_name):
    """Find weapon in gtasa_weapon_config.dat."""
    if not WEAPON_CONFIG_FILE.exists():
        return None
    for line in Path(WEAPON_CONFIG_FILE).read_text(encoding=ENCODING).splitlines():
        s = line.strip()
        if not s or s.startswith("#") or s.startswith("!"):
            continue
        parts = s.split()
        if len(parts) >= 2 and parts[1].upper() == weapon_name.upper():
            return s
    return None


def find_weapon_dat_line(weapon_name):
    """Find weapon behavior line in weapon.dat."""
    weapon_dat = DATA / "weapon.dat"
    if not weapon_dat.exists():
        return None

    lines = Path(weapon_dat).read_text(encoding=ENCODING).splitlines()
    name_upper = weapon_name.upper()

    for line in lines:
        s = line.strip()
        if not s:
            continue
        if s.startswith("$"):
            parts = s.split()
            if len(parts) >= 2 and parts[1].upper() == name_upper:
                return s
        elif s.startswith("£"):
            parts = s.split()
            if len(parts) >= 2 and parts[1].upper() == name_upper:
                return s
    return None


# ─── Core ────────────────────────────────────────────────────────────

def process_weapon(folder):
    folder_name = folder.name
    print(f"\n{BOLD}{'─'*50}{RESET}")
    print(f"{BOLD}Pasta: {folder_name}{RESET}")
    print(f"{BOLD}{'─'*50}{RESET}")

    # ── Read .txt ──────────────────────────────────────────────────
    txt_files = [x for x in folder.glob("*.txt") if x.name.lower() != "linhas_arma.txt"]
    txt_file = txt_files[0]
    display_name_raw = txt_file.stem

    txt_content = txt_file.read_text(encoding=ENCODING).strip()
    parts = txt_content.split("\n", 1)
    if len(parts) >= 2 and parts[1].strip():
        short_name_override = parts[0].strip()
        ref_weapon = parts[1].strip()
    else:
        short_name_override = None
        ref_weapon = parts[0].strip()

    display_name = format_display_name(display_name_raw)

    info(f".txt: '{display_name_raw}' → display: '{display_name}'")
    info(f"Clone de: {ref_weapon}")

    if not ref_weapon:
        fail("Conteúdo do .txt vazio — informe a arma para clonar")
        return

    # ── Validate reference ─────────────────────────────────────────
    ref_ide = find_weapon_ide(ref_weapon)
    if not ref_ide:
        fail(f"Arma referência '{ref_weapon}' não encontrada em data/default.ide (seção weap)")
        return
    ok(f"Referência encontrada no default.ide (ID: {ref_ide[0]})")

    ref_config = find_weapon_config(ref_weapon)
    if not ref_config:
        warn(f"Arma '{ref_weapon}' não encontrada no gtasa_weapon_config.dat")
        warn("Continuando sem config do fastman92...")

    ref_dat = find_weapon_dat_line(ref_weapon)
    if ref_dat:
        ok("Linha de comportamento encontrada no weapon.dat")
    else:
        warn(f"Arma '{ref_weapon}' não encontrada no weapon.dat")

    # ── Next ID ────────────────────────────────────────────────────
    next_id = get_next_available_id()
    ok(f"Próximo ID: {next_id}")

    # ── Short name ─────────────────────────────────────────────────
    used_names = get_used_names()
    if short_name_override:
        short_name = short_name_override.lower()
        if short_name in used_names:
            fail(f"Nome curto '{short_name}' já está em uso (override ignorado)")
            return
        if len(short_name) > 7 or len(short_name) < 2:
            fail(f"Nome curto deve ter 2-7 caracteres (override: {len(short_name)})")
            return
        ok(f"Nome curto (override): {short_name}")
    else:
        short_name = generate_short_name(folder_name, used_names)
        if not short_name:
            fail("Não foi possível gerar nome curto único")
            return
    short_name_upper = short_name.upper()

    # ── Rename .dff / .txd ─────────────────────────────────────────
    for ext in (".dff", ".txd"):
        files = list(folder.glob(f"*{ext}"))
        if files:
            src = files[0]
            dst = folder / f"{short_name}{ext}"
            if src.name.lower() != dst.name.lower():
                os.rename(src, dst)
                ok(f"{src.name} → {dst.name}")
            else:
                ok(f"{dst.name} já está com nome correto")
        else:
            warn(f"Arquivo *{ext} não encontrado")

    # ── Rename icon TXD to {short_name}icon ────────────────────────
    icon_files = list(folder.glob(f"*icon*"))
    if icon_files:
        src = icon_files[0]
        dst = folder / f"{short_name}icon.txd"
        if src.name.lower() != dst.name.lower():
            os.rename(src, dst)
            ok(f"{src.name} → {dst.name}")

    # ── Build IDE line (Weapons.ide) ──────────────────────────────
    ide_line = f"{next_id}, {short_name}, {short_name}, {ref_ide[3]}, {ref_ide[4]}, {ref_ide[5]}, {ref_ide[6]}"

    # ── Build weapon.dat line ──────────────────────────────────────
    if ref_dat:
        # Replace the weapon name in the line (after $ or £)
        marker = ref_dat[0]  # $ or £
        rest = ref_dat[1:].strip()
        # Find the weapon name and replace it
        parts = rest.split(None, 1)
        if len(parts) >= 2:
            weapon_dat_line = f"{marker} {short_name_upper}\t\t{parts[1]}"
        else:
            weapon_dat_line = f"{marker} {short_name_upper}"
    else:
        weapon_dat_line = None

    # ── Build fastman92 config line ────────────────────────────────
    if ref_config:
        config_parts = ref_config.split()
        config_parts[0] = str(next_id)
        config_parts[1] = short_name_upper
        fastman_line = "    ".join(config_parts[:10])
        if len(config_parts) > 10:
            fastman_line += "    " + "    ".join(config_parts[10:])
    else:
        fastman_line = f"{next_id}    {short_name_upper}    -1    1    0    0    128    128    128    1.0    {ref_weapon.upper()}"

    # ── Build linhas_arma.txt ──────────────────────────────────────
    linhas_content = (
        f"Weapons.ide\n"
        f"{ide_line}\n"
        f"\n"
        f"weapon.dat\n"
        f"{weapon_dat_line}\n"
        if weapon_dat_line else
        f"Weapons.ide\n"
        f"{ide_line}\n"
    )
    (folder / "linhas_arma.txt").write_text(linhas_content, encoding=ENCODING)
    ok("linhas_arma.txt criado")

    # ── Build .fxt ─────────────────────────────────────────────────
    fxt_content = f"{short_name} {display_name}\n"
    (folder / f"{short_name}.fxt").write_text(fxt_content, encoding=ENCODING)
    ok(f"{short_name}.fxt criado ('{display_name}')")

    # ── Rename folder ──────────────────────────────────────────────
    clean_name = re.sub(r"[^\w-]", "-", folder_name).strip("-").lower()
    clean_name = re.sub(r"-+", "-", clean_name)
    new_folder_name = f"{next_id}-{clean_name}"
    new_folder = folder.parent / new_folder_name
    if folder.name != new_folder_name:
        os.rename(folder, new_folder)
        ok(f"Pasta renomeada: {folder_name} → {new_folder_name}")
    else:
        new_folder = folder

    # ── Summary ────────────────────────────────────────────────────
    print(f"\n  {GREEN}{BOLD}✅ {display_name} (ID {next_id}, {short_name}) pronto!{RESET}")
    print(f"  {CYAN}Pasta: {new_folder.name}{RESET}")
    print(f"  {YELLOW}Próximo passo: Execute o aplicativo 'Gerar Free IDs List' do fastman92{RESET}")

    return new_folder


def main():
    if not BASE.exists():
        fail(f"Pasta não encontrada: {BASE}")
        info("Crie a pasta modloader/novas-armas/ primeiro")
        sys.exit(1)

    pending = find_pending_folders()
    if not pending:
        info("Nenhuma arma pendente encontrada.")
        info("Crie uma pasta com .dff + .txd + [Nome].txt em:")
        info(f"  {BASE}")
        return

    print(f"{BOLD}Armas pendentes encontradas: {len(pending)}{RESET}")

    for folder in pending:
        process_weapon(folder)

    print(f"\n{GREEN}{BOLD}Todos processados!{RESET}")
    print(f"{YELLOW}Próximo passo:{RESET}")
    print(f"  1. Execute 'Gerar Free IDs List' no fastman92")
    print(f"  2. Execute o jogo e teste as armas")


if __name__ == "__main__":
    main()
