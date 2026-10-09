"""Local rembg cut-out for vegetation cards and leaf sheets (T1e)."""

from __future__ import annotations

import io
from collections.abc import Callable
from pathlib import Path

from PIL import Image

REMBG_MODEL = "isnet-general-use"

Remover = Callable[[Image.Image], Image.Image]


def _rembg_remover() -> Remover:
    from rembg import new_session, remove  # lazy: heavy, optional dependency

    session = new_session(REMBG_MODEL)

    def remover(image: Image.Image) -> Image.Image:
        return remove(image, session=session)

    return remover


def cut_out(src: Path, dst: Path, *, remover: Remover | None = None) -> Path:
    """Write ``src`` with its background removed as an RGBA PNG at ``dst``."""
    image = Image.open(src).convert("RGB")
    cut = (remover or _rembg_remover())(image)
    if not isinstance(cut, Image.Image):
        cut = Image.open(io.BytesIO(cut))
    dst = Path(dst)
    dst.parent.mkdir(parents=True, exist_ok=True)
    cut.convert("RGBA").save(dst)
    return dst


def alpha_family(
    document: dict,
    only: list[str] | None = None,
    raw_dir: Path | None = None,
    remover: Remover | None = None,
) -> list[Path]:
    """Cut out the latest ok raw image of each ``alpha: true`` entry into ``alpha/<id>.png``.

    Existing outputs are kept; the rembg session is created once, on first use.
    """
    from cent_ans_tools.texture_factory.catalog import select
    from cent_ans_tools.texture_factory.generate import image_path, load_manifest

    manifest = load_manifest(document, raw_dir)
    written: list[Path] = []
    for entry in select(document, only):
        record = manifest.get(entry["id"])
        if not entry.get("alpha") or not record or record.get("status") != "ok":
            continue
        src = image_path(document, entry["id"], record["attempt"], raw_dir)
        dst = src.parent / "alpha" / f"{entry['id']}.png"
        if not dst.is_file():
            if remover is None:
                remover = _rembg_remover()
            cut_out(src, dst, remover=remover)
        written.append(dst)
    return written
