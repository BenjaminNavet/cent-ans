"""Settlement map markers (lot DA3): catalogue, schema, prompts and atlas assembly."""

import glob
import json
from pathlib import Path

import numpy as np
from jsonschema import Draft202012Validator
from PIL import Image, ImageDraw

from cent_ans_tools import map_markers

DATA = Path(__file__).resolve().parents[2] / "data"


def _catalog() -> dict:
    return map_markers.load_catalog()


def test_catalog_matches_schema() -> None:
    """The catalogue exists and matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "settlement_markers.schema.json").read_text(
            encoding="utf-8"
        )
    )
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_catalog()))
    assert not errors, [error.message for error in errors]


def test_catalog_is_consistent() -> None:
    """Cells are unique and fit the atlas; every referenced pictogram and id exists."""
    catalog = _catalog()
    ids = {p["id"] for p in catalog["pictograms"]}
    cells = [p["cell"] for p in catalog["pictograms"]]
    assert len(cells) == len(set(cells))
    assert len(ids) == len(catalog["pictograms"])
    for ranks in catalog["kinds"].values():
        assert set(ranks.values()) <= ids
    assert catalog["port_badge"] in ids
    settlement_ids = set()
    for path in glob.glob(str(DATA / "settlements" / "prov_*.json")):
        settlement_ids.update(
            e["id"] for e in json.loads(Path(path).read_text(encoding="utf-8"))
        )
    for rule in catalog["rank_rules"]:
        assert set(rule.get("ids", [])) <= settlement_ids, rule
    distances = [tier["max_distance"] for tier in catalog["visibility"]]
    assert distances == sorted(distances)


def test_prompt_ends_with_style_and_forbids_text() -> None:
    """Prompts are built from the data and end with the shared style block."""
    pictogram = _catalog()["pictograms"][0]
    prompt = map_markers.build_prompt(pictogram)
    assert pictogram["subject"] in prompt
    assert prompt.endswith(map_markers.STYLE)
    assert "no text" in map_markers.STYLE


def test_plan_is_idempotent_and_filters(tmp_path: Path) -> None:
    """Existing raw paintings are skipped; ``only`` restricts the probe."""
    catalog = _catalog()
    all_jobs = map_markers.plan(catalog, raw_dir=tmp_path)
    assert len(all_jobs) == len(catalog["pictograms"])
    first = catalog["pictograms"][0]["id"]
    map_markers.raw_path(first, tmp_path).write_bytes(b"x")
    assert first not in [
        j.character_id for j in map_markers.plan(catalog, raw_dir=tmp_path)
    ]
    probe = map_markers.plan(catalog, raw_dir=tmp_path, only=["castle", "abbey"])
    assert [j.character_id for j in probe] == ["castle", "abbey"]


def _painting(background: tuple[int, int, int]) -> Image.Image:
    """A synthetic 'painting': inked light square on a gradient wash."""
    image = Image.new("RGB", (200, 200), background)
    pixels = np.asarray(image, dtype=np.float32)
    ramp = np.linspace(0.92, 1.0, 200, dtype=np.float32)[:, None, None]
    image = Image.fromarray((pixels * ramp).astype(np.uint8))
    draw = ImageDraw.Draw(image)
    draw.rectangle(
        (60, 50, 140, 150), fill=(245, 240, 225), outline=(40, 25, 10), width=4
    )
    return image


def test_cut_out_keeps_light_subject_inside_ink() -> None:
    """The wash is removed, the light stone inside the ink contour is kept."""
    cutout = map_markers.cut_out(_painting((248, 225, 140)))
    alpha = np.asarray(cutout.getchannel("A"))
    assert alpha[5, 5] == 0 and alpha[195, 195] == 0
    assert alpha[100, 100] == 255


def test_build_atlas_places_cells(tmp_path: Path) -> None:
    """Each raw painting lands in its cell, with the ink contour; missing ones listed."""
    catalog = _catalog()
    first = catalog["pictograms"][0]
    _painting((250, 206, 73)).save(map_markers.raw_path(first["id"], tmp_path))
    out = tmp_path / "atlas.png"
    path, missing = map_markers.build_atlas(catalog, raw_dir=tmp_path, out_path=out)
    atlas = Image.open(path)
    cell_px = catalog["atlas"]["cell_px"]
    columns = catalog["atlas"]["columns"]
    assert atlas.width == cell_px * columns
    assert first["id"] not in missing
    assert len(missing) == len(catalog["pictograms"]) - 1
    col, row = first["cell"] % columns, first["cell"] // columns
    cell = np.asarray(
        atlas.crop(
            (col * cell_px, row * cell_px, (col + 1) * cell_px, (row + 1) * cell_px)
        )
    )
    assert cell[..., 3].max() == 255
    assert cell[0, 0, 3] == 0
