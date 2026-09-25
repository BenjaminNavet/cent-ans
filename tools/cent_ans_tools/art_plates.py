"""Illustrated plates (lot AR1): loading screens, event vignettes, ending screens.

Every plate is declared in ``data/ui/illustrations.json`` with its source:

* ``commons``: a public-domain illumination from Wikimedia Commons (Froissart, Grandes
  Chroniques, Vigiles de Charles VII…). The file is downloaded once into
  :data:`CACHE_DIR` (1 600 px wide rendition), then cropped with ``crop`` (centre ``x``,
  ``y`` as fractions of the image, ``scale`` of the largest box of the target ratio)
  and resized to the plate format;
* ``generated``: a gap filled by OpenRouter (``prompt``), through
  :func:`cent_ans_tools.portraits.generate` (envelope, global cap of ``docs/budget.md``,
  one budget row per batch).

Existing output files are never regenerated nor downloaded again (cache). Formats per
plate family: :data:`FORMATS` (JPEG, quality :data:`JPEG_QUALITY`).
"""

from __future__ import annotations

import io
import json
import urllib.parse
from dataclasses import dataclass
from pathlib import Path

import httpx
from PIL import Image

from cent_ans_tools.portraits import DATA_DIR, REPO_DIR, PortraitJob

ILLUSTRATIONS_FILE = DATA_DIR / "ui" / "illustrations.json"
GAME_DIR = REPO_DIR / "game"
CACHE_DIR = REPO_DIR / "tools" / ".cache" / "commons"
JPEG_QUALITY = 84
COMMONS_API = "https://commons.wikimedia.org/w/api.php"
USER_AGENT = "CentAnsTools/0.1 (https://github.com/BenjaminNavet; art pipeline)"
RENDITION_WIDTH = 1600

# Output size per plate family (width, height).
FORMATS = {
    "loading": (1600, 900),
    "vignette": (960, 400),
    "ending": (1600, 900),
}

GENERATED_STYLE = (
    "Style: 15th-century French manuscript miniature from a copy of Froissart's "
    "Chronicles (workshop of Loyset Liédet or the Master of the Getty Froissart): "
    "wide landscape scene, bright egg tempera colours, fine black ink outlines, gold "
    "leaf highlights, blue sky fading to white on the horizon, rolling green hills, "
    "period-accurate clothing and buildings. Every figure fits inside the middle 60 % "
    "of the image height. No text, no letters, no captions, no frame, no border, no "
    "modern elements."
)


@dataclass
class Plate:
    """One plate of ``data/ui/illustrations.json``."""

    id: str
    family: str
    image: str
    source: dict

    @property
    def out_path(self) -> Path:
        """Absolute path of the game file (``res://`` resolved)."""
        return GAME_DIR / self.image.removeprefix("res://")

    @property
    def size(self) -> tuple[int, int]:
        """Output size of the plate family."""
        return FORMATS[self.family]


def load(path: Path = ILLUSTRATIONS_FILE) -> dict:
    """Parsed ``illustrations.json``."""
    return json.loads(path.read_text(encoding="utf-8"))


def plates(data: dict | None = None) -> list[Plate]:
    """Every plate declared in the data, in file order."""
    data = data if data is not None else load()
    result = [
        Plate(screen["id"], "loading", screen["image"], screen["source"])
        for screen in data["loading"]["screens"]
    ]
    result += [
        Plate(vignette["id"], "vignette", vignette["image"], vignette["source"])
        for vignette in data["vignettes"]
    ]
    result += [
        Plate(ending["id"], "ending", ending["image"], ending["source"])
        for ending in data["endings"]
    ]
    return result


def crop_box(
    source_size: tuple[int, int], target_size: tuple[int, int], crop: dict | None
) -> tuple[int, int, int, int]:
    """Box of the target ratio, ``scale`` × the largest one, centred on (``x``, ``y``)."""
    crop = crop or {}
    width, height = source_size
    ratio = target_size[0] / target_size[1]
    if width / height > ratio:
        box_width, box_height = height * ratio, float(height)
    else:
        box_width, box_height = float(width), width / ratio
    scale = float(crop.get("scale", 1.0))
    box_width, box_height = box_width * scale, box_height * scale
    centre_x = float(crop.get("x", 0.5)) * width
    centre_y = float(crop.get("y", 0.5)) * height
    left = min(max(centre_x - box_width / 2, 0.0), width - box_width)
    top = min(max(centre_y - box_height / 2, 0.0), height - box_height)
    return (round(left), round(top), round(left + box_width), round(top + box_height))


def to_plate_jpg(
    image_bytes: bytes, size: tuple[int, int], crop: dict | None = None
) -> bytes:
    """Crop ``image_bytes`` per ``crop`` and resize to ``size``, as JPEG."""
    image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    band = image.crop(crop_box(image.size, size, crop))
    buffer = io.BytesIO()
    band.resize(size, Image.Resampling.LANCZOS).save(
        buffer, "JPEG", quality=JPEG_QUALITY, optimize=True, progressive=True
    )
    return buffer.getvalue()


def cache_path(file_title: str) -> Path:
    """Local cache of a Commons rendition."""
    name = file_title.removeprefix("File:").replace("/", "_")
    return CACHE_DIR / f"{Path(name).stem}.{RENDITION_WIDTH}.jpg"


def fetch_commons(file_title: str, client: httpx.Client | None = None) -> bytes:
    """Rendition of a Commons file (cached; network only on a cache miss)."""
    cached = cache_path(file_title)
    if cached.is_file():
        return cached.read_bytes()
    own_client = client is None
    client = client or httpx.Client(
        headers={"User-Agent": USER_AGENT}, timeout=60.0, follow_redirects=True
    )
    try:
        query = {
            "action": "query",
            "titles": file_title,
            "prop": "imageinfo",
            "iiprop": "url",
            "iiurlwidth": RENDITION_WIDTH,
            "format": "json",
        }
        response = client.get(f"{COMMONS_API}?{urllib.parse.urlencode(query)}")
        response.raise_for_status()
        page = next(iter(response.json()["query"]["pages"].values()))
        info = page["imageinfo"][0]
        image = client.get(info.get("thumburl") or info["url"])
        image.raise_for_status()
        data = image.content
    finally:
        if own_client:
            client.close()
    cached.parent.mkdir(parents=True, exist_ok=True)
    cached.write_bytes(data)
    return data


def missing(data: dict | None = None) -> list[Plate]:
    """Plates whose game file does not exist yet."""
    return [plate for plate in plates(data) if not plate.out_path.is_file()]


def build_commons(data: dict | None = None, force: bool = False) -> list[Path]:
    """Download and crop the public-domain plates (free); returns the written files.

    ``force`` re-crops existing files from the local cache (after a crop change).
    """
    written = []
    for plate in plates(data):
        commons = plate.source.get("commons")
        if commons is None or (plate.out_path.is_file() and not force):
            continue
        plate.out_path.parent.mkdir(parents=True, exist_ok=True)
        plate.out_path.write_bytes(
            to_plate_jpg(
                fetch_commons(commons["file"]), plate.size, commons.get("crop")
            )
        )
        written.append(plate.out_path)
    return written


def generation_jobs(data: dict | None = None) -> list[PortraitJob]:
    """Paid jobs for the generated plates that do not exist yet (never twice)."""
    jobs = []
    for plate in missing(data):
        generated = plate.source.get("generated")
        if generated is None:
            continue
        prompt = f"{generated['prompt']}\n{GENERATED_STYLE}"
        jobs.append(PortraitJob(plate.id, prompt, plate.out_path))
    return jobs


def family_of(job: PortraitJob, data: dict | None = None) -> str:
    """Plate family of a generation job."""
    return next(plate.family for plate in plates(data) if plate.id == job.character_id)


def convert_generated(image_bytes: bytes, family: str) -> bytes:
    """Crop a generated image to its plate family (centred, slightly high)."""
    return to_plate_jpg(image_bytes, FORMATS[family], {"x": 0.5, "y": 0.45})
