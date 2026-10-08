"""AR1: data/ui/illustrations.json (loading screens, event vignettes, endings) and its pipeline."""

import io
import json
import re
from pathlib import Path

from PIL import Image

from cent_ans_tools import art_plates

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
EVENTS_RS = ROOT / "core" / "crates" / "sim-campaign" / "src" / "events.rs"
MAX_TOTAL_BYTES = 25 * 1024 * 1024


def _data() -> dict:
    return json.loads((DATA / "ui" / "illustrations.json").read_text(encoding="utf-8"))


def _core_event_kinds() -> set[str]:
    """Snake-case variants of ``EventKind`` (serde ``rename_all = "snake_case"``)."""
    body = EVENTS_RS.read_text(encoding="utf-8").split("pub enum EventKind {", 1)[1]
    body = body.split("}", 1)[0]
    names = re.findall(r"^\s*([A-Z][A-Za-z]+),", body, flags=re.MULTILINE)
    return {re.sub(r"(?<!^)([A-Z])", r"_\1", name).lower() for name in names}


def test_ids_unique_and_images_exist() -> None:
    """Every plate has a unique id and its image file exists, all under 25 MB."""
    plates = art_plates.plates(_data())
    ids = [plate.id for plate in plates]
    assert len(ids) == len(set(ids))
    total = 0
    for plate in plates:
        assert plate.out_path.is_file(), plate.image
        with Image.open(plate.out_path) as image:
            assert image.size == plate.size, (plate.id, image.size)
        total += plate.out_path.stat().st_size
    assert total < MAX_TOTAL_BYTES


def test_counts_and_contexts() -> None:
    """6-12 loading screens covering every context, 12-16 vignettes, the 4 endings."""
    data = _data()
    screens = data["loading"]["screens"]
    assert 6 <= len(screens) <= 12
    contexts = {context for screen in screens for context in screen["contexts"]}
    assert contexts == {"battle", "siege", "naval", "campaign"}
    assert 12 <= len(data["vignettes"]) <= 16
    outcomes = {ending["outcome"] for ending in data["endings"]}
    assert outcomes == {
        "battle_victory",
        "battle_defeat",
        "campaign_victory",
        "campaign_defeat",
    }
    assert data["loading"]["min_seconds"] <= data["loading"]["max_seconds"]


def test_vignette_kinds_are_core_event_kinds() -> None:
    """Each vignette kind is an ``EventKind`` of the core, used by one vignette only."""
    known = _core_event_kinds()
    assert "plague" in known and "war_declared" in known
    seen: set[str] = set()
    for vignette in _data()["vignettes"]:
        for kind in vignette["kinds"]:
            assert kind in known, (vignette["id"], kind)
            assert kind not in seen, kind
            seen.add(kind)


def test_crop_box_keeps_ratio_and_bounds() -> None:
    """The crop box has the target ratio and stays inside the source image."""
    box = art_plates.crop_box((1000, 1000), (1600, 900), {"x": 0.5, "y": 0.9})
    left, top, right, bottom = box
    assert (left, right) == (0, 1000)
    assert bottom == 1000 and abs((right - left) / (bottom - top) - 16 / 9) < 0.01
    small = art_plates.crop_box((1000, 1000), (960, 400), {"scale": 0.5})
    assert small[2] - small[0] == 500


def test_to_plate_jpg_size() -> None:
    """Conversion outputs a JPEG of the plate size."""
    buffer = io.BytesIO()
    Image.new("RGB", (300, 400), "red").save(buffer, "PNG")
    out = art_plates.to_plate_jpg(buffer.getvalue(), (160, 90))
    with Image.open(io.BytesIO(out)) as image:
        assert image.format == "JPEG" and image.size == (160, 90)


def test_generation_never_redoes_existing_files() -> None:
    """With every plate on disk, there is nothing to generate (no paid call)."""
    assert art_plates.generation_jobs(_data()) == []
