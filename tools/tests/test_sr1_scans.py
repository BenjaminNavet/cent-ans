"""SR1: ambientCG CC0 scans in the fine figure material pipeline (no network)."""

import io
import json
import re
import zipfile
from pathlib import Path

import numpy as np
import pytest
import yaml
from PIL import Image

from cent_ans_tools import material_gen

ROOT = Path(__file__).resolve().parents[2]
SHADER_PATH = ROOT / "game" / "shaders" / "battle_soldier_skinned.gdshader"


def _shader_tile_sizes() -> list[float]:
    text = SHADER_PATH.read_text(encoding="utf-8")
    match = re.search(
        r"const float GA1_TILE_SIZE\[(\d+)\]\s*=\s*float\[\]\(([^)]*)\);", text
    )
    assert match, "GA1_TILE_SIZE introuvable dans le shader des figurines"
    values = [float(value) for value in match.group(2).split(",")]
    assert len(values) == int(match.group(1))
    return values


def test_shader_tile_sizes_match_yaml():
    """The shader projects each layer with the tile side of its yaml entry."""
    entries = material_gen.load_materials()["materials"]
    assert _shader_tile_sizes() == pytest.approx([float(e["tile_m"]) for e in entries])


def test_scanned_layers_are_the_sr1_assets():
    """SR1 table: 8 ambientCG scans in layer order, 4 generated layers kept."""
    entries = material_gen.load_materials()["materials"]
    sources = {entry["id"]: material_gen.scan_asset_id(entry) for entry in entries}
    assert sources == {
        "wool": "Fabric045",
        "linen": "Fabric061",
        "fustian": "Fabric066",
        "gambeson": "Fabric048",
        "mail": "Chainmail002",
        "leather": "Leather033A",
        "plate": "Metal055A",
        "wood": "Wood049",
        "skin": None,
        "hair": None,
        "coat_light": None,
        "coat_dark": None,
    }
    for entry in entries:
        if material_gen.scan_asset_id(entry):
            assert float(entry["tile_m"]) <= float(entry["scan_m"])


def _fake_scan(size: int = 128) -> dict[str, np.ndarray]:
    """Scan with a periodic relief/normal and a non-periodic albedo ramp (hard edges)."""
    phase = np.linspace(0, 8 * np.pi, size, endpoint=False)
    wave = np.sin(phase)[np.newaxis, :] * np.ones((size, 1))
    height = np.rint(127.5 + 100 * wave).astype(np.uint8)
    normal = np.dstack(
        [
            np.rint(127.5 + 60 * np.cos(phase))[np.newaxis, :] * np.ones((size, 1)),
            np.full((size, size), 128.0),
            np.full((size, size), 230.0),
        ]
    ).astype(np.uint8)
    return {
        "albedo": np.dstack(
            [np.linspace(0, 255, size)[np.newaxis, :] * np.ones((size, 1))] * 3
        ).astype(np.uint8),
        "normal": normal,
        "roughness": np.full((size, size), 128, dtype=np.uint8),
        "height": height,
    }


def _unit_lengths(normal: np.ndarray) -> np.ndarray:
    vectors = normal.astype(np.float64) / 127.5 - 1.0
    return np.linalg.norm(vectors, axis=-1)


def test_scan_tile_whole_scan_keeps_scale_and_units():
    """tile_m = scan_m: whole scan resized, unit normals, roughness bias applied."""
    tiles = material_gen.scan_tile(
        _fake_scan(), scan_m=0.2, tile_m=0.2, tile_size=64, roughness_bias=0.2
    )
    assert tiles["albedo"].shape == (64, 64, 3)
    assert tiles["normal"].shape == (64, 64, 3)
    assert tiles["height"].shape == tiles["roughness"].shape == (64, 64)
    assert np.abs(_unit_lengths(tiles["normal"]) - 1).max() < 0.02
    assert abs(float(tiles["roughness"].mean()) - (128 + 0.2 * 255)) < 2
    # Four sine periods across the scan stay four periods across the tile.
    spectrum = np.abs(np.fft.rfft(tiles["height"][0].astype(np.float64) - 128))
    assert int(np.argmax(spectrum)) == 4


def test_scan_tile_crop_is_tileable_and_rejects_oversize():
    """A crop (tile_m < scan_m) of a non-periodic ramp is blended tileable."""
    tiles = material_gen.scan_tile(
        _fake_scan(), scan_m=0.2, tile_m=0.13, tile_size=64, blend_width=16
    )
    image = tiles["albedo"].astype(np.int16)
    assert np.abs(image[:, 0] - image[:, -1]).max() < 12
    assert np.abs(_unit_lengths(tiles["normal"]) - 1).max() < 0.02
    with pytest.raises(ValueError):
        material_gen.scan_tile(_fake_scan(), scan_m=0.1, tile_m=0.2, tile_size=64)


def test_normal_strength_scales_the_slope():
    """normal_strength steepens the scan normal (X spread grows)."""
    soft = material_gen.scan_tile(_fake_scan(), scan_m=1, tile_m=1, tile_size=64)
    steep = material_gen.scan_tile(
        _fake_scan(), scan_m=1, tile_m=1, tile_size=64, normal_strength=2.0
    )
    assert np.ptp(steep["normal"][..., 0]) > np.ptp(soft["normal"][..., 0])


def _zip_of_maps(asset_id: str) -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        for name in material_gen.SCAN_MAPS + ("Metalness",):
            image = io.BytesIO()
            Image.new("RGB", (32, 32), (120, 130, 140)).save(image, "JPEG")
            archive.writestr(f"{asset_id}_1K-JPG_{name}.jpg", image.getvalue())
    return buffer.getvalue()


def _metadata(asset_id: str) -> dict:
    downloads = [
        {"attribute": attribute, "fullDownloadPath": f"https://x/{attribute}.zip"}
        for attribute in ("2K-JPG", "1K-JPG", "1K-PNG")
    ]
    return {
        "foundAssets": [
            {
                "assetId": asset_id,
                "downloadFolders": {
                    "default": {
                        "downloadFiletypeCategories": {"zip": {"downloads": downloads}}
                    }
                },
            }
        ]
    }


def test_fetch_ambientcg_downloads_once(tmp_path):
    """The 1K-JPG zip is fetched once, maps extracted, cache reused afterwards."""
    calls: list[str] = []

    def http_get(url: str) -> bytes:
        calls.append(url)
        if "full_json" in url:
            return json.dumps(_metadata("Fabric999")).encode()
        assert url == "https://x/1K-JPG.zip"
        return _zip_of_maps("Fabric999")

    paths = material_gen.fetch_ambientcg("Fabric999", tmp_path, http_get=http_get)
    assert set(paths) == set(material_gen.SCAN_MAPS)
    assert all(path.exists() for path in paths.values())
    assert len(calls) == 2 and "id=Fabric999&include=downloadData" in calls[0]
    material_gen.fetch_ambientcg("Fabric999", tmp_path, http_get=None)  # no network
    assert len(calls) == 2


def test_ambientcg_download_url_requires_the_resolution():
    """An answer without a 1K-JPG archive is refused."""
    metadata = _metadata("Wood001")
    assert material_gen.ambientcg_download_url(metadata, "wood001").endswith(
        "1K-JPG.zip"
    )
    with pytest.raises(KeyError):
        material_gen.ambientcg_download_url({"foundAssets": []}, "Wood001")


def test_scan_entries_and_kept_layers(tmp_path):
    """A scanned entry is processed from the cache; a tile-less layer is kept verbatim."""
    cache = tmp_path / "cache"
    material_gen.fetch_ambientcg(
        "Fabric999",
        cache,
        http_get=lambda url: (
            json.dumps(_metadata("Fabric999")).encode()
            if "full_json" in url
            else _zip_of_maps("Fabric999")
        ),
    )
    materials = tmp_path / "materials.yaml"
    materials.write_text(
        yaml.safe_dump(
            {
                "model": "test/model",
                "size": 64,
                "tile_size": 32,
                "materials": [
                    {
                        "id": "wool",
                        "source": "ambientcg:Fabric999",
                        "scan_m": 0.1,
                        "tile_m": 0.1,
                    },
                    {"id": "skin", "prompt": "x" * 30, "tile_m": 0.06},
                ],
            }
        ),
        encoding="utf-8",
    )
    tiles = tmp_path / "tiles"
    material_gen.process_scan("wool", tiles, materials_path=materials, cache_dir=cache)
    out = tmp_path / "out"
    out.mkdir()
    size = material_gen.ALBEDO_LAYER_SIZE
    kept_detail = np.full((64, 32, 4), 77, dtype=np.uint8)
    kept_albedo = np.full((2 * size, size, 3), 99, dtype=np.uint8)
    Image.fromarray(kept_detail, "RGBA").save(out / material_gen.DETAIL_ARRAY)
    Image.fromarray(kept_albedo, "RGB").save(out / material_gen.ALBEDO_ARRAY)
    paths = material_gen.build_fine_arrays(tiles, out, materials_path=materials)
    detail = np.asarray(Image.open(paths["detail"]))
    albedo = np.asarray(Image.open(paths["albedo"]))
    assert detail.shape == (64, 32, 4) and albedo.shape == (2 * size, size, 3)
    assert (detail[32:] == 77).all() and (albedo[size:] == 99).all()
    assert not (detail[:32] == 77).all()
    with pytest.raises(FileNotFoundError):
        material_gen.build_fine_arrays(
            tiles, tmp_path / "empty", materials_path=materials
        )
