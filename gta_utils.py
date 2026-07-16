import shutil
from pathlib import Path

ENCODING = "cp1252"

BEGIN_MARKER = "# BEGIN GENERATED VEHICLES"
END_MARKER = "# END GENERATED VEHICLES"


def safe_write(path: Path, lines: list[str]) -> None:
    path = Path(path)
    if path.exists():
        bak = path.with_suffix(path.suffix + ".bak")
        shutil.copy2(path, bak)
    path.write_text("".join(lines), encoding=ENCODING)


def read_lines(path: Path) -> list[str]:
    return Path(path).read_text(encoding=ENCODING).splitlines(keepends=True)
