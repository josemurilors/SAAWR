#!/usr/bin/env python3
"""add_car.py — Add new vehicles to GTA SA (modloader + fastman92)

Usage:
  Put this script in <GTA_DIR>/ferramentas/
  Create a folder in modloader/novos-carros/<nome-do-veiculo>/
  Place .dff + .txd inside
  Create a "Nome do Veiculo 2012.txt" with the reference vehicle name inside
  Run: python add_car.py
"""

import os
import re
import sys
from pathlib import Path

from gta_utils import safe_write, read_lines, ENCODING, BEGIN_MARKER, END_MARKER, get_game_paths

# ─── Paths ───────────────────────────────────────────────────────────
_paths = get_game_paths()
BASE = _paths["modloader_vehicles"]
DATA = BASE / "data"
TRANSITO = BASE / "add-transito"
AUDIO_CFG = _paths["audio_cfg"]
VEHICLE_ID_START = _paths["vehicle_id_start"]

STOCK_IDS = set(range(400, 612))

# ─── Class → cargrp.dat mapping ─────────────────────────────────────
CLASS_TO_GROUPS = {
    "richfamily": [
        "POPCYCLE_GROUP_CASUAL_RICH",
        "POPCYCLE_GROUP_BUSINESS",
    ],
    "normal": ["POPCYCLE_GROUP_CASUAL_AVERAGE"],
    "poorfamily": ["POPCYCLE_GROUP_CASUAL_POOR"],
    "worker": ["POPCYCLE_GROUP_WORKERS"],
    "executive": [
        "POPCYCLE_GROUP_CASUAL_RICH",
        "POPCYCLE_GROUP_BUSINESS",
    ],
    "motorbike": [
        "POPCYCLE_GROUP_BEACHFOLK",
        "POPCYCLE_GROUP_PARKFOLK",
        "POPCYCLE_GROUP_CASUAL_AVERAGE",
        "POPCYCLE_GROUP_CASUAL_RICH",
    ],
    "moped": [
        "POPCYCLE_GROUP_BEACHFOLK",
        "POPCYCLE_GROUP_PARKFOLK",
    ],
    "taxi": [
        "POPCYCLE_GROUP_WORKERS",
        "POPCYCLE_GROUP_BUSINESS",
        "POPCYCLE_GROUP_CLUBBERS",
    ],
    "bicycle": [
        "POPCYCLE_GROUP_BEACHFOLK",
        "POPCYCLE_GROUP_PARKFOLK",
    ],
}

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
    no_year = re.sub(r"[-]?\d{4}$", "", folder_name)
    clean = re.sub(r"[^a-zA-Z0-9]", "", no_year)
    parts = re.split(r"[-_\s]+", no_year)
    first_word = parts[0].lower() if parts else clean.lower()
    numbers = "".join(re.findall(r"\d+", no_year))

    candidates = []

    if numbers:
        first_letter = first_word[0] if first_word else "x"
        for n_len in range(min(4, len(numbers)), 0, -1):
            candidates.append((first_letter + numbers[:n_len]).lower())

    candidates.append(first_word[:7])

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
        c = f"v{i}"
        if c not in existing_names:
            log(f"Nome curto: {c} (fallback)")
            return c

    return None


def get_used_ids():
    ids = set(STOCK_IDS)
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
    nid = VEHICLE_ID_START
    while nid in used:
        nid += 1
    return nid


def get_used_names():
    names = set()
    ide_path = DATA / "vehicles.ide"
    if ide_path.exists():
        for line in Path(ide_path).read_text(encoding=ENCODING).splitlines():
            line = line.strip()
            if not line or line.startswith("#") or line in ("cars", "end"):
                continue
            parts = [p.strip() for p in line.split(",")]
            if len(parts) >= 2:
                names.add(parts[1].split()[0].lower())
    if BASE.exists():
        for folder in BASE.iterdir():
            if not folder.is_dir():
                continue
            lf = folder / "linhas.txt"
            if not lf.exists():
                continue
            for line in Path(lf).read_text(encoding=ENCODING).splitlines():
                line = line.strip()
                if line and "," in line and not line.startswith("#"):
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
        if (f / "linhas.txt").exists():
            continue
        dff = list(f.glob("*.dff"))
        txt = [x for x in f.glob("*.txt") if x.name.lower() != "linhas.txt"]
        if dff and txt:
            pending.append(f)
    return pending


def find_vehicle_ide(model_name):
    model_upper = model_name.upper()
    for line in Path(DATA / "vehicles.ide").read_text(encoding=ENCODING).splitlines():
        line = line.strip()
        if not line or line.startswith("#") or line in ("cars", "end"):
            continue
        parts = [p.strip() for p in line.split(",")]
        if len(parts) >= 8 and parts[1].split()[0].upper() == model_upper:
            return parts
    return None


def find_vehicle_handling(handling_id):
    for line in Path(DATA / "handling.cfg").read_text(encoding=ENCODING).splitlines():
        s = line.strip()
        if not s or s.startswith(";") or s.startswith("#"):
            continue
        if s[0] in ("!", "%", "$"):
            continue
        first = s.split()[0].upper()
        if first == handling_id.upper():
            return s
    return None


def find_vehicle_audio(model_name):
    if not AUDIO_CFG.exists():
        return None
    for line in Path(AUDIO_CFG).read_text(encoding=ENCODING).splitlines():
        s = line.strip()
        if not s or s.startswith(";"):
            continue
        first = s.split()[0].lower()
        if first == model_name.lower():
            return s
    return None


def add_to_cargrp(model_name, vehicle_class, log=ok):
    groups = CLASS_TO_GROUPS.get(vehicle_class, [])
    if not groups:
        warn(f"Classe '{vehicle_class}' sem mapeamento de trânsito")
        return False

    path = TRANSITO / "cargrp.dat"
    if not path.exists():
        warn("add-transito/cargrp.dat não encontrado")
        return False

    lines = read_lines(path)

    modified = False
    for group in groups:
        tag = f"# {group}"
        for i, line in enumerate(lines):
            if tag not in line:
                continue
            if model_name in line.lower().split("#")[0].split(","):
                continue
            hp = line.index("#")
            before = line[:hp].rstrip().rstrip(",")
            lines[i] = before + f", {model_name}" + " " + line[hp:]
            modified = True

    if modified:
        safe_write(path, lines)
        log(f"Adicionado a {', '.join(groups)} no cargrp.dat")
    else:
        warn("Nenhum grupo encontrado para adicionar no cargrp.dat")
    return modified


def add_audio_entry(model_name, ref_model_name, log=ok):
    if not AUDIO_CFG.exists():
        warn("gtasa_vehicleAudioSettings.cfg não encontrado")
        return False

    ref = find_vehicle_audio(ref_model_name)
    if not ref:
        warn(f"Áudio de referência '{ref_model_name}' não encontrado")
        return False

    parts = ref.split()
    if len(parts) < 14:
        warn("Formato de áudio inesperado")
        return False

    new_line = model_name + ref[len(parts[0]):]
    lines = read_lines(AUDIO_CFG)

    begin_idx = None
    end_idx = None
    the_end_idx = None

    for i, line in enumerate(lines):
        stripped = line.rstrip("\n\r")
        if stripped == BEGIN_MARKER:
            begin_idx = i
        elif stripped == END_MARKER:
            end_idx = i
        elif stripped == ";the end":
            the_end_idx = i

    if begin_idx is not None and end_idx is not None:
        lines.insert(end_idx, new_line + "\n")
    elif the_end_idx is not None:
        indent = ""
        if the_end_idx > 0 and lines[the_end_idx - 1].strip() == "":
            insert_at = the_end_idx
        else:
            lines.insert(the_end_idx, "\n")
            insert_at = the_end_idx
        lines.insert(insert_at, END_MARKER + "\n")
        lines.insert(insert_at, new_line + "\n")
        lines.insert(insert_at, BEGIN_MARKER + "\n")
    else:
        lines.append("\n" + BEGIN_MARKER + "\n")
        lines.append(new_line + "\n")
        lines.append(END_MARKER + "\n")

    safe_write(AUDIO_CFG, lines)
    log(f"Áudio adicionado (clonado de {ref_model_name})")
    return True


# ─── Core ────────────────────────────────────────────────────────────

def process_vehicle(folder):
    folder_name = folder.name
    print(f"\n{BOLD}{'─'*50}{RESET}")
    print(f"{BOLD}Pasta: {folder_name}{RESET}")
    print(f"{BOLD}{'─'*50}{RESET}")

    # ── Read .txt ──────────────────────────────────────────────────
    txt_files = [x for x in folder.glob("*.txt") if x.name.lower() != "linhas.txt"]
    txt_file = txt_files[0]
    display_name_raw = txt_file.stem

    txt_content = txt_file.read_text(encoding=ENCODING).strip()
    parts = txt_content.split("\n", 1)
    if len(parts) >= 2 and parts[1].strip():
        short_name_override = parts[0].strip()
        ref_vehicle = parts[1].strip()
    else:
        short_name_override = None
        ref_vehicle = parts[0].strip()

    display_name = format_display_name(display_name_raw)

    info(f".txt: '{display_name_raw}' → display: '{display_name}'")
    info(f"Clone de: {ref_vehicle}")

    if not ref_vehicle:
        fail("Conteúdo do .txt vazio — informe o veículo para clonar")
        return

    # ── Validate reference ─────────────────────────────────────────
    ref_ide = find_vehicle_ide(ref_vehicle)
    if not ref_ide:
        fail(f"Veículo referência '{ref_vehicle}' não encontrado em data/vehicles.ide")
        return
    info(f"Referência: {ref_ide[1]} (tipo: {ref_ide[3]}, classe: {ref_ide[7]})")
    ok("Referência encontrada no vehicles.ide")

    ref_handling_id = ref_ide[4]
    ref_handling = find_vehicle_handling(ref_handling_id)
    if not ref_handling:
        fail(f"Handling de '{ref_handling_id}' não encontrado em data/handling.cfg")
        return
    ok("Handling de referência encontrado")

    # ── Next ID ────────────────────────────────────────────────────
    vehicle_type = ref_ide[3]
    vehicle_class = ref_ide[7]
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

    # ── Build IDE line ──────────────────────────────────────────────
    new_ide_parts = list(ref_ide)
    new_ide_parts[0] = str(next_id)
    new_ide_parts[1] = short_name
    new_ide_parts[2] = short_name
    new_ide_parts[4] = short_name_upper
    new_ide_parts[5] = short_name_upper if len(short_name_upper) <= 7 else short_name_upper[:7]
    ide_line = ", ".join(new_ide_parts)

    # ── Build handling line ────────────────────────────────────────
    handling_line = short_name_upper + ref_handling[len(ref_handling.split()[0]):]

    # ── Build carcols line ─────────────────────────────────────────
    carcols_line = f"{short_name}, 0,0, 1,1, 2,2, 3,3, 4,4, 5,5, 6,6, 7,7"

    # ── Build linhas.txt ──────────────────────────────────────────
    linhas_content = (
        f"Vehicles.ide\n"
        f"{ide_line}\n"
        f"\n"
        f"Handling.cfg\n"
        f"{handling_line}\n"
        f"\n"
        f"Carcols.dat\n"
        f"{carcols_line}\n"
        f"\n"
        f"Carmods.dat\n"
    )
    (folder / "linhas.txt").write_text(linhas_content, encoding=ENCODING)
    ok("linhas.txt criado")

    # ── Build .fxt ─────────────────────────────────────────────────
    fxt_content = f"{short_name} {display_name}\n"
    (folder / f"{short_name}.fxt").write_text(fxt_content, encoding=ENCODING)
    ok(f"{short_name}.fxt criado ('{display_name}')")

    # ── Audio ──────────────────────────────────────────────────────
    add_audio_entry(short_name, ref_vehicle)

    # ── Transit ────────────────────────────────────────────────────
    add_to_cargrp(short_name, vehicle_class)

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

    return new_folder


def sync_transit():
    if not BASE.exists():
        return

    path = TRANSITO / "cargrp.dat"
    if not path.exists():
        warn("add-transito/cargrp.dat não encontrado")
        return

    lines = read_lines(path)

    fixed = 0
    for folder in sorted(BASE.iterdir()):
        if not folder.is_dir():
            continue
        lf = folder / "linhas.txt"
        if not lf.exists():
            continue

        for line in Path(lf).read_text(encoding=ENCODING).splitlines():
            line = line.strip()
            if not line or "," not in line or line.startswith("#"):
                continue
            parts = [p.strip() for p in line.split(",")]
            if len(parts) < 8:
                continue
            model = parts[1].lower()
            vehicle_class = parts[7].lower()

            groups = CLASS_TO_GROUPS.get(vehicle_class, [])
            if not groups:
                continue

            already = False
            for l in lines:
                if model in l.lower():
                    already = True
                    break
            if already:
                break

            for group in groups:
                tag = f"# {group}"
                for i, l in enumerate(lines):
                    if tag not in l:
                        continue
                    if model in l.lower().split("#")[0].split(","):
                        continue
                    hp = l.index("#")
                    before = l[:hp].rstrip().rstrip(",")
                    lines[i] = before + f", {model}" + " " + l[hp:]
                    fixed += 1
                    break
            break

    if fixed:
        safe_write(path, lines)
        ok(f"{fixed} entrada(s) adicionada(s) ao cargrp.dat")
    else:
        info("Todos os veículos já estão no trânsito.")


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "--sync":
        sync_transit()
        return

    if not BASE.exists():
        fail(f"Pasta não encontrada: {BASE}")
        sys.exit(1)

    pending = find_pending_folders()
    if not pending:
        info("Nenhum veículo pendente encontrado.")
        info("Crie uma pasta com .dff + .txd + [Nome].txt em:")
        info(f"  {BASE}")
        return

    print(f"{BOLD}Veículos pendentes encontrados: {len(pending)}{RESET}")

    for folder in pending:
        process_vehicle(folder)

    print(f"\n{GREEN}{BOLD}Todos processados! Ative no jogo e teste.{RESET}")


if __name__ == "__main__":
    main()
