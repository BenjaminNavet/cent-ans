"""Validates data/ui/fa_ui_assets.json and the cutting of the FA5 interface ornaments."""

import json
from pathlib import Path

import numpy as np
from jsonschema import Draft202012Validator
from PIL import Image

from cent_ans_tools import fa_ui_assets

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
ASSETS = ROOT / "game" / "assets" / "ui" / "fa"
MATTE = {"low": 0.12, "high": 0.3, "background_blur_px": 8, "feather_px": 0.5}


def _catalogue() -> dict:
    return fa_ui_assets.load_catalogue()


def _cuts(catalogue: dict) -> list[tuple[str, dict]]:
    return [(kind, cut) for kind in fa_ui_assets.KINDS for cut in catalogue[kind]]


def test_cuts_reference_known_sources_and_unique_ids() -> None:
    """Every cut names a declared source, every source is used, ids are unique per kind."""
    catalogue = _catalogue()
    used = set()
    for kind in fa_ui_assets.KINDS:
        ids = [cut["id"] for cut in catalogue[kind]]
        assert len(ids) == len(set(ids)), kind
    for kind, cut in _cuts(catalogue):
        assert cut["source"] in catalogue["sources"], (kind, cut["id"])
        used.add(cut["source"])
        if "rim" in cut:
            assert cut["rim"]["source"] in catalogue["sources"], (kind, cut["id"])
            used.add(cut["rim"]["source"])
        if "box" in cut:
            left, top, right, bottom = cut["box"]
            assert right > left and bottom > top, (kind, cut["id"])
    assert used == set(catalogue["sources"])


def test_versioned_files_match_catalogue() -> None:
    """Each cut has its PNG within its size budget, and no stray PNG is left behind."""
    catalogue = _catalogue()
    expected = set()
    for kind, cut in _cuts(catalogue):
        path = ASSETS / kind / f"{cut['id']}.png"
        expected.add(path)
        assert path.exists(), path
        margin = 0
        if "shadow" in cut:
            shadow = cut["shadow"]
            margin = 2 * (
                int(np.ceil(shadow["blur_px"] * 2.5))
                + max(map(abs, shadow["offset_px"]))
            )
        assert max(Image.open(path).size) <= cut["size"] + margin, path
    assert set(ASSETS.glob("*/*.png")) == expected
    total = sum(path.stat().st_size for path in expected)
    assert total <= 6 * 1024 * 1024


def test_source_file_lists_every_cut() -> None:
    """`SOURCE.md` is the one the catalogue generates (object, institution, licence, URL)."""
    text = (ASSETS / "SOURCE.md").read_text(encoding="utf-8")
    assert text == fa_ui_assets.sources_markdown(_catalogue())
    for kind, cut in _cuts(_catalogue()):
        assert f"`{kind}/{cut['id']}.png`" in text


def test_lift_removes_the_support_and_keeps_the_subject() -> None:
    """A dark blot on shaded parchment comes out opaque, the parchment transparent."""
    shade = np.linspace(0.85, 0.95, 64, dtype=np.float32)[None, :, None]
    region = np.ones((64, 64, 3), dtype=np.float32) * np.array([1.0, 0.94, 0.8]) * shade
    region[20:44, 20:44] = [0.1, 0.15, 0.5]
    region[30:34, 30:34] = [0.95, 0.9, 0.78]  # pale highlight enclosed by paint
    lifted = fa_ui_assets.lift(region, {**MATTE, "fill_holes_px": 64})
    alpha = lifted[..., 3]
    assert alpha[32, 32] > 0.95 and alpha[24, 24] > 0.95
    assert alpha[:12].max() < 0.02 and alpha[:, 52:].max() < 0.02
    assert np.allclose(lifted[24, 24, :3], [0.1, 0.15, 0.5], atol=0.02)


def test_ellipse_and_largest_blob_isolate_a_seal() -> None:
    """A seal keeps only its largest blob inside the limiting ellipse."""
    region = np.full((80, 80, 3), 0.8, dtype=np.float32)
    rows, columns = np.mgrid[0:80, 0:80]
    region[(rows - 40) ** 2 + (columns - 40) ** 2 < 20**2] = [0.3, 0.1, 0.05]
    region[2:8, 2:8] = [0.3, 0.1, 0.05]  # stray speck
    region[:, 36:44] = np.minimum(region[:, 36:44], 0.45)  # dark tag crossing the seal
    matte = {**MATTE, "keep_largest": True, "ellipse": [40, 40, 24, 24]}
    alpha = fa_ui_assets.lift(region, matte)[..., 3]
    assert alpha[40, 40] > 0.95
    assert alpha[4, 4] == 0.0
    assert alpha[2, 40] == 0.0 and alpha[77, 40] == 0.0


def test_fit_orient_and_shadow() -> None:
    """Resizing keeps proportions, rotation and flip apply, the shadow pads the cut."""
    rgba = np.zeros((40, 80, 4), dtype=np.float32)
    rgba[..., 0] = 1.0
    rgba[..., 3] = 1.0
    assert fa_ui_assets.fit(rgba, 20).shape == (10, 20, 4)
    assert fa_ui_assets.fit(rgba, 400).shape == (40, 80, 4)
    region = np.zeros((2, 3, 3), dtype=np.float32)
    region[0, 0] = 1.0
    assert fa_ui_assets.orient(region, {"rotate": 90}).shape == (3, 2, 3)
    assert fa_ui_assets.orient(region, {"flip_h": True})[0, 2, 0] == 1.0
    shadowed = fa_ui_assets.drop_shadow(
        rgba, {"offset_px": [2, 3], "blur_px": 2.0, "opacity": 0.5}
    )
    assert shadowed.shape[0] > 40 and shadowed.shape[1] > 80
    pad = (shadowed.shape[0] - 40) // 2
    assert 0.0 < shadowed[pad + 41, pad + 40, 3] < 0.6
    assert np.allclose(shadowed[pad + 20, pad + 40], [1.0, 0.0, 0.0, 1.0], atol=1e-3)


def test_tint_recolours_by_luminance() -> None:
    """A wax tint keeps the relief (luminance order) and takes the tint's hue."""
    rgb = np.array([[[0.2, 0.2, 0.2], [0.5, 0.5, 0.5]]], dtype=np.float32)
    graded = fa_ui_assets.grade(rgb, {"tint": [150, 30, 20]})
    assert graded[0, 1, 0] > graded[0, 0, 0]
    assert graded[0, 0, 0] > graded[0, 0, 1] > graded[0, 0, 2] - 0.05


def test_display_names_existing_cuts() -> None:
    """The ornaments and seals the game is told to use exist; an empty spray means none."""
    catalogue = _catalogue()
    display = catalogue["display"]
    if display["title_spray"]:
        assert display["title_spray"] in {cut["id"] for cut in catalogue["ornaments"]}
    seals = {cut["id"] for cut in catalogue["seals"]}
    assert set(display["seals"].values()) <= seals


def test_catalogue_without_initials_or_spray_is_valid(tmp_path: Path) -> None:
    """Initials and the title spray are optional: an empty catalogue of them validates and builds."""
    catalogue = _catalogue()
    catalogue["initials"] = []
    catalogue["ornaments"] = []
    catalogue["display"]["title_spray"] = ""
    schema = json.loads(
        (DATA / "schemas" / "ui_fa_assets.schema.json").read_text(encoding="utf-8")
    )
    assert not list(Draft202012Validator(schema).iter_errors(catalogue))
    text = fa_ui_assets.sources_markdown(catalogue)
    assert "initials/" not in text and "ornaments/" not in text
    assert "seals/" in text


def test_initial_and_ornament_entries_still_validate() -> None:
    """The shelved kinds stay usable: a framed initial and a matte ornament match the schema."""
    catalogue = _catalogue()
    source = next(iter(catalogue["sources"]))
    catalogue["initials"] = [
        {
            "id": "d_example",
            "letter": "D",
            "source": source,
            "box": [0, 0, 64, 64],
            "size": 128,
            "mode": "framed",
        }
    ]
    catalogue["ornaments"] = [
        {
            "id": "spray_example",
            "source": source,
            "box": [0, 0, 64, 32],
            "size": 384,
            "matte": MATTE,
        }
    ]
    catalogue["display"]["title_spray"] = "spray_example"
    schema = json.loads(
        (DATA / "schemas" / "ui_fa_assets.schema.json").read_text(encoding="utf-8")
    )
    assert not list(Draft202012Validator(schema).iter_errors(catalogue))
    region = np.full((64, 64, 3), 0.5, dtype=np.float32)
    assert fa_ui_assets.cut_initial(region, catalogue["initials"][0]).shape == (
        64,
        64,
        4,
    )


def test_rimmed_plate_is_a_seamless_nine_slice() -> None:
    """A rimmed plate keeps one whole period of the material inside a darker-edged rim."""
    columns = np.linspace(0.0, 1.0, 64, dtype=np.float32)
    material = np.dstack([np.tile(columns, (64, 1))] * 3)
    metal = np.full((64, 64, 3), 0.8, dtype=np.float32)
    rim = {"source": "metal", "width_px": 4, "outline": 0.5, "groove": 0.5}
    plate = fa_ui_assets.cut_material(material, {"size": 40, "rim": rim}, metal)
    assert plate.shape == (40, 40, 4)
    assert np.allclose(plate[20, 1, :3], 0.8, atol=0.02)
    assert np.allclose(plate[20, 0, :3], 0.4, atol=0.02)
    assert np.allclose(plate[20, 3, :3], 0.4, atol=0.02)
    centre = plate[20, 4:36, 0]
    assert centre[0] < 0.1 and centre[-1] > 0.9
    plain = fa_ui_assets.cut_material(material, {"size": 32})
    assert plain.shape == (32, 32, 4) and plain[..., 3].min() == 1.0
