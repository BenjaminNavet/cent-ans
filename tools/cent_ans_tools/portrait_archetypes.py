"""Living portraits (lot DA2): archetype bank and aged variants of major figures.

Everything is driven by ``data/portraits/archetypes.json`` (schema
``data/schemas/portrait_archetypes.schema.json``):

- **archetypes**: one image per (rank, sex, age band, dress culture, face) cell,
  saved as ``game/assets/<archetype_dir>/<rank>_<sex>_<band>_<culture>_<face>.jpg``.
  The face index picks a stable face description (hair, face shape, eyes) so that
  the same index across age bands reads as the same "line" of face; the game
  chooses the index by hashing the character id (``living_portrait.gd``);
- **aged variants**: for the major historical figures who outlive their 1337
  portrait, ``game/assets/<aged_dir>/<character>_<band>.jpg``, prompted from the
  character data (:func:`cent_ans_tools.portraits.build_prompt`) with the existing
  portrait sent as a likeness reference.

Prompts always end with the shared :data:`cent_ans_tools.portraits.STYLE` block;
paid batches go through :func:`cent_ans_tools.portraits.generate` (envelope,
global cap, one ``docs/budget.md`` row per batch). Images are 512x512 JPEG
(bible DA § 5) to keep ``game/assets/portraits`` well under 40 MB.
"""

from __future__ import annotations

import io
import json
from dataclasses import dataclass
from pathlib import Path

from PIL import Image

from cent_ans_tools import portraits
from cent_ans_tools.portraits import PortraitJob

REPO_DIR = Path(__file__).resolve().parents[2]
DATA_DIR = REPO_DIR / "data"
ASSETS_DIR = REPO_DIR / "game" / "assets"
CONFIG_PATH = DATA_DIR / "portraits" / "archetypes.json"
JPEG_QUALITY = 86

# Archetypes carry no arms: the game paints the faction shield in the frame.
NO_ARMS = (
    "No coat of arms, no heraldic shield, no badge: plain unmarked clothing "
    "(this is a generic figure, the arms are added by the game)."
)


@dataclass(frozen=True)
class ArchetypeSpec:
    """One archetype image of the bank."""

    rank: str
    sex: str
    band: str
    culture: str
    face: int

    @property
    def key(self) -> str:
        """File stem, also the key the game rebuilds from a character."""
        return f"{self.rank}_{self.sex}_{self.band}_{self.culture}_{self.face}"


def load_config(path: Path = CONFIG_PATH) -> dict:
    """The archetype configuration (validated against its schema by the tests)."""
    return json.loads(path.read_text(encoding="utf-8"))


def band_for_age(config: dict, age: int) -> str:
    """Age band id for ``age`` (last band has no ``max_age``)."""
    for band in config["age_bands"]:
        if "max_age" not in band or age <= band["max_age"]:
            return band["id"]
    return config["age_bands"][-1]["id"]


def iter_archetypes(config: dict) -> list[ArchetypeSpec]:
    """Every archetype image of the bank, in data order."""
    specs = []
    for cell in config["cells"]:
        for sex in cell["sexes"]:
            for band in cell["bands"]:
                for culture in cell["cultures"]:
                    for face in range(cell["faces"]):
                        specs.append(
                            ArchetypeSpec(cell["rank"], sex, band, culture, face)
                        )
    return specs


def _by_id(items: list[dict]) -> dict[str, dict]:
    return {item["id"]: item for item in items}


def build_archetype_prompt(config: dict, spec: ArchetypeSpec) -> str:
    """Prompt of one archetype, from the data only, ending with the shared STYLE."""
    ranks = _by_id(config["ranks"])
    bands = _by_id(config["age_bands"])
    cultures = _by_id(config["cultures"])
    rank_text = ranks[spec.rank]["prompt"][spec.sex]
    faces = config["faces"][spec.sex]
    face_text = faces[spec.face % len(faces)]
    person = "woman" if spec.sex == "female" else "man"
    if spec.band == "child":
        person = "girl" if spec.sex == "female" else "boy"
    lines = [
        f"Portrait of an anonymous {person}: {rank_text}.",
        f"Age: {bands[spec.band]['prompt']}.",
        f"Face: {face_text}"
        + (" (hair now grey or white)" if spec.band == "old" else "")
        + ".",
    ]
    if spec.culture != "any":
        lines.append(f"Dress: {cultures[spec.culture]['prompt']}.")
    lines.append(NO_ARMS)
    lines.append(portraits.STYLE)
    return "\n".join(lines)


def build_aged_prompt(
    config: dict,
    character: dict,
    band: str,
    factions: dict[str, dict],
    traits: dict[str, dict],
) -> str:
    """Aged variant prompt: the character prompt with the new age and a likeness note."""
    bands = _by_id(config["age_bands"])
    depicted = (
        f"depicted later in life as {bands[band]['prompt']}; same person as the "
        "reference portrait attached (keep the likeness, features and colouring, "
        "only older)"
    )
    return portraits.build_prompt(character, factions, traits, depicted=depicted)


def to_archetype_jpg(image_bytes: bytes, size: int = 512) -> bytes:
    """Square crop (top-biased for tall images), resize, JPEG."""
    image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    width, height = image.size
    side = min(width, height)
    left = (width - side) // 2
    top = 0 if height > width else (height - side) // 2
    square = image.crop((left, top, left + side, top + side))
    buffer = io.BytesIO()
    square.resize((size, size), Image.Resampling.LANCZOS).save(
        buffer, "JPEG", quality=JPEG_QUALITY, optimize=True, progressive=True
    )
    return buffer.getvalue()


def plan(
    config: dict | None = None,
    *,
    assets_dir: Path = ASSETS_DIR,
    data_dir: Path = DATA_DIR,
    archetypes: bool = True,
    aged: bool = True,
    only: list[str] | None = None,
    limit: int | None = None,
) -> list[PortraitJob]:
    """Missing images (idempotent), archetypes first then aged variants.

    ``only`` restricts to the given keys (archetype stems or ``<character>_<band>``),
    used for the 3-image style probe.
    """
    config = config or load_config()
    jobs: list[PortraitJob] = []
    if archetypes:
        out_dir = assets_dir / config["archetype_dir"]
        for spec in iter_archetypes(config):
            out_path = out_dir / f"{spec.key}.jpg"
            if out_path.exists() or (only is not None and spec.key not in only):
                continue
            jobs.append(
                PortraitJob(spec.key, build_archetype_prompt(config, spec), out_path)
            )
    if aged:
        characters = portraits._load_dir(data_dir / "characters")
        factions = portraits._load_dir(data_dir / "factions")
        traits = portraits._load_dir(data_dir / "traits")
        out_dir = assets_dir / config["aged_dir"]
        for variant in config["aged_variants"]:
            key = f"{variant['character']}_{variant['band']}"
            out_path = out_dir / f"{key}.jpg"
            if out_path.exists() or (only is not None and key not in only):
                continue
            character = characters[variant["character"]]
            reference = assets_dir / "portraits" / f"{variant['character']}.png"
            jobs.append(
                PortraitJob(
                    key,
                    build_aged_prompt(
                        config, character, variant["band"], factions, traits
                    ),
                    out_path,
                    reference if reference.exists() else None,
                )
            )
    return jobs[:limit] if limit is not None else jobs
