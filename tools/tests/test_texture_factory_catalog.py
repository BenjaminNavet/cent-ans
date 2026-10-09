# ruff: noqa: D103
"""Tests du catalogue de textures et de la génération reprenable (TX T1c)."""

from __future__ import annotations

import copy
from pathlib import Path

import pytest
import yaml

from cent_ans_tools.texture_factory import catalog, generate

ENTRY = {
    "id": "grass_a",
    "role": "ground",
    "prompt": "short grass, {tile_m} m patch",
    "seed": 100,
    "tile_m": 4,
    "biomes": [1, 2],
}
DOCUMENT = {
    "family": "ground_campaign",
    "size": 1024,
    "output_size": 2048,
    "style_prefix": "top-down photo,",
    "style_suffix": "seamless.",
    "entries": [ENTRY, {**ENTRY, "id": "mud_b", "seed": 200, "size": 1536}],
}


def write_catalog(tmp_path, document):
    (tmp_path / f"{document['family']}.yaml").write_text(
        yaml.safe_dump(document), encoding="utf-8"
    )


def fake_runner(calls):
    def run(command):
        calls.append(command)
        Path(command[command.index("--output") + 1]).write_bytes(b"PNG")

    return run


def test_load_catalog_accepts_valid(tmp_path):
    write_catalog(tmp_path, DOCUMENT)
    loaded = catalog.load_catalog("ground_campaign", tmp_path)
    assert [e["id"] for e in loaded["entries"]] == ["grass_a", "mud_b"]


def test_load_catalog_accepts_regions_and_checks(tmp_path):
    document = copy.deepcopy(DOCUMENT)
    entry = document["entries"][0]
    del entry["biomes"]
    entry["regions"] = ["normandy"]
    entry["alpha"] = True
    entry["checks"] = {"luminance": [0.2, 0.6], "seam_max": 0.1}
    write_catalog(tmp_path, document)
    catalog.load_catalog("ground_campaign", tmp_path)


@pytest.mark.parametrize(
    "mutate",
    [
        lambda d: d["entries"][0].update(biomes=[15]),
        lambda d: d["entries"][0].update(extra=1),
        lambda d: d["entries"][0].pop("seed"),
        lambda d: d["entries"][0].pop("biomes"),
        lambda d: d.update(size=2048),
        lambda d: d.update(output_size=1024),
        lambda d: d.pop("style_prefix"),
        lambda d: d["entries"][0].update(checks={"luminance": [1]}),
        lambda d: d["entries"].append(copy.deepcopy(d["entries"][0])),
    ],
)
def test_load_catalog_rejects_invalid(tmp_path, mutate):
    document = copy.deepcopy(DOCUMENT)
    mutate(document)
    write_catalog(tmp_path, document)
    with pytest.raises(catalog.CatalogError):
        catalog.load_catalog("ground_campaign", tmp_path)


def test_load_catalog_missing_and_wrong_family(tmp_path):
    with pytest.raises(catalog.CatalogError, match="introuvable"):
        catalog.load_catalog("ground_campaign", tmp_path)
    (tmp_path / "ground_battle.yaml").write_text(
        yaml.safe_dump(DOCUMENT), encoding="utf-8"
    )
    with pytest.raises(catalog.CatalogError, match="attendu"):
        catalog.load_catalog("ground_battle", tmp_path)


def test_select_and_build_prompt():
    assert len(catalog.select(DOCUMENT)) == 2
    assert [e["id"] for e in catalog.select(DOCUMENT, ["mud_b"])] == ["mud_b"]
    with pytest.raises(catalog.CatalogError):
        catalog.select(DOCUMENT, ["nope"])
    assert (
        catalog.build_prompt(DOCUMENT, ENTRY)
        == "top-down photo, short grass, 4 m patch seamless."
    )


def test_generate_caches_and_resumes(tmp_path):
    calls: list = []
    manifest = generate.generate(DOCUMENT, raw_dir=tmp_path, runner=fake_runner(calls))
    assert len(calls) == 2
    assert manifest["grass_a"] == {"attempt": 1, "seed": 100, "status": "ok"}
    assert (tmp_path / "grass_a_1.png").read_bytes() == b"PNG"
    # size override of the second entry reaches the command
    assert calls[1][calls[1].index("--width") + 1] == "1536"
    assert calls[0][calls[0].index("--width") + 1] == "1024"
    generate.generate(DOCUMENT, raw_dir=tmp_path, runner=fake_runner(calls))
    assert len(calls) == 2  # cache: nothing regenerated
    generate.generate(
        DOCUMENT, only=["mud_b"], raw_dir=tmp_path, runner=fake_runner(calls)
    )
    assert len(calls) == 2


def test_generate_failure_is_recorded_and_batch_continues(tmp_path):
    calls: list = []
    good = fake_runner(calls)

    def runner(command):
        if "--seed" in command and command[command.index("--seed") + 1] == "100":
            raise RuntimeError("boom")
        good(command)

    manifest = generate.generate(DOCUMENT, raw_dir=tmp_path, runner=runner)
    assert manifest["grass_a"]["status"] == "failed"
    assert manifest["mud_b"]["status"] == "ok"
    assert not (tmp_path / "grass_a_1.png").exists()
    # resume: the failed entry is rendered next time
    manifest = generate.generate(DOCUMENT, raw_dir=tmp_path, runner=good)
    assert manifest["grass_a"]["status"] == "ok"
    assert generate.load_manifest(DOCUMENT, tmp_path) == manifest


def test_retry_uses_next_seeds_then_flags(tmp_path):
    calls: list = []
    runner = fake_runner(calls)
    generate.generate(DOCUMENT, raw_dir=tmp_path, runner=runner)
    manifest = generate.retry_flagged(DOCUMENT, ["grass_a"], tmp_path, runner)
    assert manifest["grass_a"] == {"attempt": 2, "seed": 101, "status": "ok"}
    manifest = generate.retry_flagged(DOCUMENT, ["grass_a"], tmp_path, runner)
    assert manifest["grass_a"] == {"attempt": 3, "seed": 102, "status": "ok"}
    count = len(calls)
    manifest = generate.retry_flagged(DOCUMENT, ["grass_a"], tmp_path, runner)
    assert manifest["grass_a"]["status"] == "flagged"
    assert len(calls) == count
    assert (tmp_path / "grass_a_2.png").is_file()
    assert (tmp_path / "grass_a_3.png").is_file()
    assert manifest["mud_b"]["attempt"] == 1
