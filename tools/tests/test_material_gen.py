"""GA: tileable material pipeline and data/art/materials.yaml."""

import json
from pathlib import Path

import numpy as np
import pytest
import yaml
from jsonschema import Draft202012Validator
from PIL import Image

from cent_ans_tools import material_gen

ROOT = Path(__file__).resolve().parents[2]
SCHEMA_PATH = ROOT / "data" / "schemas" / "materials.schema.json"


def _hard_edged_image(size: int = 256) -> np.ndarray:
    """Smooth image whose opposite borders do NOT match (worst case for tiling)."""
    ramp = np.linspace(0, 255, size)
    red = np.tile(ramp, (size, 1))
    green = red.T
    blue = np.tile(128 + 60 * np.sin(np.linspace(0, 3 * np.pi, size)), (size, 1))
    return np.stack([red, green, blue], axis=-1).astype(np.uint8)


def _max_edge_gap(image: np.ndarray) -> int:
    signed = image.astype(np.int16)
    horizontal = np.abs(signed[:, 0] - signed[:, -1]).max()
    vertical = np.abs(signed[0, :] - signed[-1, :]).max()
    return int(max(horizontal, vertical))


def _materials_file(tmp_path: Path) -> Path:
    materials = tmp_path / "materials.yaml"
    materials.write_text(
        yaml.safe_dump(
            {
                "model": "test/model",
                "size": 256,
                "tile_size": 64,
                "materials": [{"id": "wool", "prompt": "x" * 30, "blend_width": 16}],
            }
        ),
        encoding="utf-8",
    )
    return materials


def test_make_tileable_edges_match():
    """Opposite edges differ by at most 4/255 after make_tileable."""
    image = _hard_edged_image()
    assert _max_edge_gap(image) > 200
    tiled = material_gen.make_tileable(image, blend_width=32)
    assert tiled.shape == image.shape
    assert tiled.dtype == np.uint8
    assert _max_edge_gap(tiled) <= 4


def test_make_tileable_has_no_central_seam():
    """The blended central cross has no jump larger than a few source steps."""
    tiled = material_gen.make_tileable(_hard_edged_image(), blend_width=32)
    signed = tiled.astype(np.int16)
    assert np.abs(np.diff(signed, axis=1)).max() <= 16
    assert np.abs(np.diff(signed, axis=0)).max() <= 16


def test_make_tileable_rejects_bad_blend():
    """A blend wider than half the image is refused."""
    with pytest.raises(ValueError):
        material_gen.make_tileable(_hard_edged_image(64), blend_width=40)


def test_derive_maps_shapes_and_channels():
    """Derived maps keep the albedo size and expected channel counts."""
    rng = np.random.default_rng(7)
    albedo = rng.integers(0, 256, size=(128, 96, 3), dtype=np.uint8)
    maps = material_gen.derive_maps(albedo)
    assert set(maps) == {"height", "normal", "roughness"}
    assert maps["height"].shape == (128, 96)
    assert maps["roughness"].shape == (128, 96)
    assert maps["normal"].shape == (128, 96, 3)
    for values in maps.values():
        assert values.dtype == np.uint8
    # Normals point out of the surface: blue stays in the upper half.
    assert maps["normal"][..., 2].min() >= 128


def test_derive_maps_normal_is_opengl():
    """OpenGL (Y+): a slope rising towards the image bottom faces up, green > 128."""
    rows = np.arange(64)
    luminance = 128 + 100 * np.sin(2 * np.pi * rows / 64)
    albedo = np.stack([np.tile(luminance[:, np.newaxis], (1, 64))] * 3, axis=-1)
    maps = material_gen.derive_maps(albedo.astype(np.uint8))
    # Around rows 0/63 the height rises with the row index (downwards): faces up.
    assert maps["normal"][[62, 63, 0, 1], :, 1].mean() > 132
    # Around row 32 it rises upwards: the surface faces down.
    assert maps["normal"][30:34, :, 1].mean() < 124
    assert abs(float(maps["normal"][..., 0].mean()) - 128) < 2


def test_derive_maps_are_tileable():
    """Derived maps of a tileable albedo stay tileable (wrapped filters)."""
    albedo = material_gen.make_tileable(_hard_edged_image(), blend_width=32)
    maps = material_gen.derive_maps(albedo)
    for name in ("height", "roughness"):
        assert _max_edge_gap(maps[name]) <= 24, name


def test_process_and_contact_sheet(tmp_path):
    """Process writes tile_size maps; contact_sheet writes a small labelled PNG."""
    materials = _materials_file(tmp_path)
    raw = tmp_path / "wool_raw.png"
    Image.fromarray(_hard_edged_image(300)[:, :280]).save(raw)
    paths = material_gen.process(
        "wool", raw, tmp_path / "out", materials_path=materials
    )
    assert set(paths) == {"albedo", "height", "normal", "roughness"}
    albedo = np.asarray(Image.open(paths["albedo"]))
    assert albedo.shape == (64, 64, 3)
    assert _max_edge_gap(albedo) <= 6
    assert np.asarray(Image.open(paths["normal"])).shape == (64, 64, 3)
    assert np.asarray(Image.open(paths["roughness"])).shape == (64, 64)
    sheet = material_gen.contact_sheet(
        {"wool": albedo, "mail": albedo[::-1]}, tmp_path / "sheet.png", panel=96
    )
    assert sheet.stat().st_size <= material_gen.SHEET_MAX_BYTES
    assert Image.open(sheet).size[1] > 2 * 96


def test_generate_refuses_outside_ga_section(tmp_path):
    """No paid call when the ledger's last section is not the GA one."""
    ledger = tmp_path / "budget.md"
    ledger.write_text(
        "# Budget\n\n## Autre chantier\n\n"
        "| Date | Service | Objet | Coût estimé | Coût réel | Cumul |\n"
        "|---|---|---|---|---|---|\n"
        "| 2026-09-28 | — | x | 0,00 $ | 0,00 $ | 0,00 $ |\n",
        encoding="utf-8",
    )
    with pytest.raises(RuntimeError, match="Assets générés GA"):
        material_gen.generate(
            "wool",
            tmp_path / "out",
            materials_path=_materials_file(tmp_path),
            budget_path=ledger,
        )


def test_generate_reuses_existing_raw(tmp_path):
    """An existing raw image is processed without touching the ledger or the network."""
    out = tmp_path / "out"
    out.mkdir()
    Image.fromarray(_hard_edged_image()).save(out / "wool_raw.png")
    albedo = material_gen.generate(
        "wool",
        out,
        materials_path=_materials_file(tmp_path),
        budget_path=tmp_path / "missing.md",
    )
    assert albedo.name == "wool_albedo.png"


def test_materials_yaml_matches_schema():
    """data/art/materials.yaml validates against materials.schema.json."""
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    document = material_gen.load_materials()
    errors = list(Draft202012Validator(schema).iter_errors(document))
    assert not errors, [error.message for error in errors]
    ids = [entry["id"] for entry in document["materials"]]
    assert len(ids) == len(set(ids))
    assert document["size"] % document["tile_size"] == 0
