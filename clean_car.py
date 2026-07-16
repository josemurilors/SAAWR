#!/usr/bin/env python3
"""clean_car.py — List installed vehicles and clean residues from deleted ones.

Usage:
  python clean_car.py          → lista instalados e mostra resíduos
  python clean_car.py clean    → lista e já pergunta se quer remover
"""

import re
from pathlib import Path

from gta_utils import safe_write, read_lines, ENCODING, BEGIN_MARKER, END_MARKER

# ─── Paths ───────────────────────────────────────────────────────────
GAME_DIR = Path(__file__).resolve().parent.parent
BASE = GAME_DIR / "modloader" / "novos-carros"
DATA = BASE / "data"
TRANSITO = BASE / "add-transito"
AUDIO_CFG = (
    GAME_DIR
    / "modloader"
    / "$fastman92 limit adjuster"
    / "data"
    / "gtasa_vehicleAudioSettings.cfg"
)
CARG_PATH = TRANSITO / "cargrp.dat"

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

def load_stock_names():
    names = set()
    p = DATA / "vehicles.ide"
    if not p.exists():
        return names
    for line in Path(p).read_text(encoding=ENCODING).splitlines():
        line = line.strip()
        if not line or line.startswith("#") or line in ("cars", "end"):
            continue
        parts = [x.strip() for x in line.split(",")]
        if len(parts) >= 2:
            names.add(parts[1].split()[0].lower())
    return names


# ─── Installed vehicles ──────────────────────────────────────────────

def get_installed():
    installed = []
    if not BASE.exists():
        return installed
    for folder in sorted(BASE.iterdir()):
        if not folder.is_dir():
            continue
        lf = folder / "linhas.txt"
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


# ─── Audio residues (marker-based) ───────────────────────────────────

def find_audio_block():
    if not AUDIO_CFG.exists():
        return None, None, None
    lines = read_lines(AUDIO_CFG)

    begin_idx = None
    end_idx = None

    for i, line in enumerate(lines):
        stripped = line.rstrip("\n\r")
        if stripped == BEGIN_MARKER:
            begin_idx = i
        elif stripped == END_MARKER:
            end_idx = i

    return lines, begin_idx, end_idx


def find_audio_residues(installed_models, stock_names):
    lines, begin_idx, end_idx = find_audio_block()
    if lines is None:
        return [], None

    if begin_idx is None or end_idx is None:
        return _legacy_find_audio_residues(installed_models, stock_names)

    residues = []
    for i in range(begin_idx + 1, end_idx):
        s = lines[i].strip()
        if not s or s.startswith(";"):
            continue
        model = s.split()[0].lower()
        if model not in stock_names and model not in installed_models:
            residues.append({"idx": i, "model": model, "line": s})

    return residues, lines


def _legacy_find_audio_residues(installed_models, stock_names):
    lines = read_lines(AUDIO_CFG)
    residues = []
    in_section = False

    for i, line in enumerate(lines):
        s = line.strip()
        if "added vehicles" in s:
            in_section = True
            continue
        if s == ";the end":
            break
        if not in_section:
            continue
        if not s or s.startswith(";"):
            continue
        model = s.split()[0].lower()
        if model not in stock_names and model not in installed_models:
            residues.append({"idx": i, "model": model, "line": s})

    return residues, lines


def remove_audio_residues(residues, all_lines, installed_models):
    if not residues:
        return False

    lines, begin_idx, end_idx = find_audio_block()

    if begin_idx is not None and end_idx is not None:
        managed_block = []
        for i in range(begin_idx + 1, end_idx):
            s = lines[i].strip()
            if not s or s.startswith(";"):
                continue
            model = s.split()[0].lower()
            if model in installed_models:
                managed_block.append(lines[i])
        new_lines = lines[:begin_idx + 1] + managed_block + lines[end_idx:]
        safe_write(AUDIO_CFG, new_lines)
        return True

    for r in reversed(residues):
        del all_lines[r["idx"]]
    safe_write(AUDIO_CFG, all_lines)
    return True


# ─── cargrp residues ─────────────────────────────────────────────────

def find_cargrp_residues(installed_models, stock_names):
    if not CARG_PATH.exists():
        return [], None

    lines = read_lines(CARG_PATH)
    residues = []

    for i, line in enumerate(lines):
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        before_comment = s.split("#")[0].strip()
        models = [m.strip() for m in before_comment.split(",")]
        for model in models:
            ml = model.lower()
            if ml and ml not in stock_names and ml not in installed_models:
                residues.append({
                    "idx": i,
                    "model": ml,
                    "line": s,
                })

    return residues, lines


def remove_cargrp_residues(residues, lines):
    if not residues:
        return False
    mod = False
    by_idx = {}
    for r in residues:
        by_idx.setdefault(r["idx"], []).append(r["model"])

    for idx, models in by_idx.items():
        line = lines[idx]
        hp = line.index("#")
        before = line[:hp].rstrip().rstrip(",")
        parts = [m.strip() for m in before.split(",")]
        clean = [p for p in parts if p.lower() not in models]
        if len(clean) != len(parts):
            lines[idx] = ", ".join(clean) + " " + line[hp:]
            mod = True

    if mod:
        safe_write(CARG_PATH, lines)
    return mod


# ─── Main ────────────────────────────────────────────────────────────

def main():
    stock_names = load_stock_names()
    info(f"Veículos originais carregados: {len(stock_names)}")

    installed = get_installed()
    if not installed:
        warn("Nenhum veículo instalado encontrado em novos-carros/")
        return

    installed_models = {v["model"] for v in installed if v["model"]}

    print(f"\n{BOLD}Veículos instalados:{RESET}")
    print(f"  {'ID':<8} {'Modelo':<10} {'Display':<35} {'Pasta'}")
    print(f"  {'─'*8} {'─'*10} {'─'*35} {'─'*30}")
    for v in installed:
        vid = ""
        m = re.match(r"(\d+)", v["folder"])
        if m:
            vid = m.group(1)
        print(f"  {vid:<8} {v['model'] or '?':<10} {v['display']:<35} {v['folder']}")

    audio_res, audio_lines = find_audio_residues(installed_models, stock_names)
    cargrp_res, cargrp_lines = find_cargrp_residues(installed_models, stock_names)

    total = len(audio_res) + len(cargrp_res)
    if total == 0:
        print(f"\n{GREEN}{BOLD}Nenhum resíduo encontrado. Tudo limpo!{RESET}")
        return

    print(f"\n{YELLOW}{BOLD}Resíduos encontrados ({total}):{RESET}")

    if audio_res:
        print(f"\n  {CYAN}Áudio (gtasa_vehicleAudioSettings.cfg):{RESET}")
        for r in audio_res:
            print(f"    {RED}{r['model']:<10}{RESET} → linha {r['idx']+1}")

    if cargrp_res:
        print(f"\n  {CYAN}Trânsito (cargrp.dat):{RESET}")
        for r in cargrp_res:
            group = ""
            if "#" in r["line"]:
                group = r["line"].split("#")[1].strip()
            print(f"    {RED}{r['model']:<10}{RESET} → {group}")

    ans = input(f"\n{BOLD}Remover resíduos? (s/N): {RESET}").strip().lower()
    if ans != "s":
        info("Nada foi alterado.")
        return

    if audio_res and audio_lines:
        remove_audio_residues(audio_res, audio_lines, installed_models)
        ok(f"{len(audio_res)} entr. de áudio removidas")

    if cargrp_res and cargrp_lines:
        remove_cargrp_residues(cargrp_res, cargrp_lines)
        ok(f"{len(cargrp_res)} entr. de trânsito removidas")

    print(f"\n{GREEN}{BOLD}Resíduos removidos!{RESET}")


if __name__ == "__main__":
    main()
