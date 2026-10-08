"""``--local`` on every ``assets`` image command (ADR 0190, lot A).

Each command is run through ``CliRunner`` with the mflux subprocess faked: no network, the
aspect ratio reaches mflux as ``--width/--height``, and the ledger gets no row. Output
paths are redirected to ``tmp_path``: nothing is written under ``game/assets``.
"""

from pathlib import Path

import httpx
import pytest
import yaml
from PIL import Image
from typer.testing import CliRunner

from cent_ans_tools import (
    art_plates,
    budget,
    cli,
    codex_art,
    entity_icons,
    event_art,
    ground_materials,
    horizon_panoramas,
    ink_icons,
    local_art,
    portrait_archetypes,
    portraits,
    ui_ornaments,
)
from cent_ans_tools.portraits import PortraitJob

runner = CliRunner()
REAL_BUDGET = budget.DEFAULT_BUDGET_PATH


@pytest.fixture(autouse=True)
def offline_and_free(monkeypatch):
    """Forbid network and ledger writes for every test of this file."""

    def no_network(*args, **kwargs):
        raise AssertionError("network call in --local mode")

    def no_row(*args, **kwargs):
        raise AssertionError("ledger row in --local mode")

    monkeypatch.setattr(httpx.Client, "send", no_network)
    monkeypatch.setattr(httpx, "get", no_network)
    monkeypatch.setattr(budget, "add_entry", no_row)
    monkeypatch.setattr(budget.BudgetLedger, "add_entry", no_row)
    before = REAL_BUDGET.read_text(encoding="utf-8")
    yield
    assert REAL_BUDGET.read_text(encoding="utf-8") == before


def _sizes(commands: list[list[str]]) -> set[tuple[int, int]]:
    return {
        (
            int(command[command.index("--width") + 1]),
            int(command[command.index("--height") + 1]),
        )
        for command in commands
    }


def _jobs(tmp_path: Path, count: int = 1) -> list[PortraitJob]:
    return [
        PortraitJob(f"job_{i}", "A scene", tmp_path / f"job_{i}.jpg")
        for i in range(count)
    ]


def _invoke(args: list[str]):
    result = runner.invoke(cli.app, ["assets", *args])
    assert result.exit_code == 0, result.output
    return result


# (command, module attribute to stub with the jobs, expected mflux size)
BATCH_COMMANDS = [
    ("event-art", event_art, "plan", "16:9"),
    ("codex-art", codex_art, "plan", "16:9"),
    ("portraits", portraits, "plan", "3:4"),
    ("portrait-archetypes", portrait_archetypes, "plan", "3:4"),
]


@pytest.mark.parametrize(("command", "module", "attribute", "aspect"), BATCH_COMMANDS)
def test_batch_commands_render_locally(
    command, module, attribute, aspect, fake_mflux, tmp_path, monkeypatch
):
    """Portrait/event/codex batches use the local model at their final crop's aspect."""
    jobs = _jobs(tmp_path)
    monkeypatch.setattr(module, attribute, lambda *a, **k: jobs)
    _invoke([command, "--local"])
    assert _sizes(fake_mflux) == {local_art.ASPECT_SIZES[aspect]}
    assert jobs[0].out_path.exists()


def test_art_plates_use_one_aspect_per_family(fake_mflux, tmp_path, monkeypatch):
    """Loading/ending plates are 16:9, the wide vignette 21:9."""
    jobs = _jobs(tmp_path, 3)
    families = dict(
        zip((j.character_id for j in jobs), art_plates.FORMATS, strict=True)
    )
    monkeypatch.setattr(art_plates, "build_commons", lambda *a, **k: [])
    monkeypatch.setattr(art_plates, "generation_jobs", lambda *a, **k: jobs)
    monkeypatch.setattr(
        art_plates, "family_of", lambda job, *a, **k: families[job.character_id]
    )
    _invoke(["art-plates", "--generate", "--local"])
    assert _sizes(fake_mflux) == {
        local_art.ASPECT_SIZES["16:9"],
        local_art.ASPECT_SIZES["21:9"],
    }
    assert all(job.out_path.exists() for job in jobs)


def test_entity_icons_render_square(fake_mflux, tmp_path, monkeypatch):
    """Entity miniatures are centre-cropped squares: 1:1."""
    jobs = _jobs(tmp_path)
    monkeypatch.setattr(entity_icons, "plan", lambda *a, **k: jobs)
    monkeypatch.setattr(
        entity_icons,
        "build",
        lambda *a, **k: {"derived": [], "generated": [], "missing": []},
    )
    monkeypatch.setattr(entity_icons, "contact_sheet", lambda *a, **k: tmp_path / "s")
    _invoke(["entity-icons", "--local"])
    assert _sizes(fake_mflux) == {local_art.ASPECT_SIZES["1:1"]}


def test_ink_icons_render_square(fake_mflux, tmp_path, monkeypatch):
    """Icons and medallions are square: 1:1, for both kinds."""
    jobs = _jobs(tmp_path)
    monkeypatch.setattr(ink_icons, "plan", lambda *a, **k: jobs)
    monkeypatch.setattr(
        ink_icons,
        "build",
        lambda *a, **k: {"icons": [], "medallions": [], "missing": []},
    )
    monkeypatch.setattr(ink_icons, "contact_sheet", lambda *a, **k: tmp_path / "s")
    _invoke(["ink-icons", "--local"])
    assert _sizes(fake_mflux) == {local_art.ASPECT_SIZES["1:1"]}


def test_dry_run_local_is_free_and_offline(tmp_path, monkeypatch):
    """A local dry run announces 0 $ without any lookup."""
    monkeypatch.setattr(event_art, "plan", lambda *a, **k: _jobs(tmp_path, 2))
    result = _invoke(["event-art", "--local", "--dry-run"])
    assert "0.00 $" in result.output
    assert local_art.MODEL_ID in result.output


def test_horizon_panoramas_are_ultra_wide(fake_mflux, tmp_path):
    """The generator paints 21:9 locally, records nothing and needs no budget room."""
    data = {"prompt_template": "{prompt}", "panoramas": {"hills": {"prompt": "hills"}}}
    costs = horizon_panoramas.generate(
        ["hills"], local_art.MODEL_ID, data=data, raw_dir=tmp_path
    )
    assert costs[0][1] == 0
    assert _sizes(fake_mflux) == {local_art.ASPECT_SIZES["21:9"]}
    assert (tmp_path / "hills.jpg").exists()


def test_horizon_command_passes_the_local_model(monkeypatch):
    """``--local`` swaps the model; processing stays untouched."""
    seen = {}
    monkeypatch.setattr(
        horizon_panoramas,
        "generate",
        lambda ids, model, **k: seen.update(model=model, ids=ids),
    )
    monkeypatch.setattr(
        horizon_panoramas, "process_all", lambda *a, **k: {"panoramas": {}}
    )
    _invoke(["horizon-panoramas", "--generate", "--local", "--ids", "x"])
    assert seen["model"] == local_art.MODEL_ID


def test_materials_render_square_without_ledger_check(fake_mflux, tmp_path):
    """Generated materials are square 1:1 and skip the section/ledger guard."""
    config = tmp_path / "materials.yaml"
    config.write_text(
        yaml.safe_dump(
            {
                "model": "openai/gpt-5-image-mini",
                "size": 64,
                "tile_size": 32,
                "materials": [{"id": "felt", "prompt": "felt"}],
            }
        ),
        encoding="utf-8",
    )
    out = tmp_path / "out"
    _invoke(["materials", "--out", str(out), "--config", str(config), "--local"])
    assert _sizes(fake_mflux) == {local_art.ASPECT_SIZES["1:1"]}
    assert (out / "felt_albedo.png").exists()
    result = _invoke(
        [
            "materials",
            "--out",
            str(out / "b"),
            "--config",
            str(config),
            "--local",
            "--dry-run",
        ]
    )
    assert local_art.MODEL_ID in result.output


def test_ui_ornaments_keep_aspect_and_drop_gemini_size(fake_mflux, tmp_path):
    """Local ornaments send only the aspect ratio and ignore an empty anchor."""
    request = ui_ornaments.OrnamentRequest(
        piece_id="frame",
        variant=0,
        prompt="p",
        references=(b"",),
        image_config={"aspect_ratio": "21:9", "image_size": "2K"},
        seed=1,
    )
    paths = ui_ornaments.generate([request], raw_dir=tmp_path, model=local_art.MODEL_ID)
    assert _sizes(fake_mflux) == {local_art.ASPECT_SIZES["21:9"]}
    assert "--image-path" not in fake_mflux[0]
    assert Image.open(paths[0]).size == local_art.ASPECT_SIZES["21:9"]


def test_ui_ornaments_command_passes_the_local_model(monkeypatch):
    """The CLI hands the local model to the generator."""
    seen = {}
    monkeypatch.setattr(
        ui_ornaments, "generate", lambda requests, **k: seen.update(k) or []
    )
    _invoke(["ui-ornaments", "--local", "--dry-run"])
    assert seen["model"] == local_art.MODEL_ID


def test_ground_materials_render_square(fake_mflux, tmp_path):
    """Ground raw images: local 1:1 (1024 px as the fal call), cost 0, no fal client."""
    document = {
        "model": "fal-ai/flux-2-pro",
        "size": 1024,
        "style_prefix": "Aerial.",
        "style_suffix": "No text.",
        "scale_sentence": "Tile {tile_m} m.",
        "materials": [{"id": "loam", "seed": 7, "prompt": "loam", "tile_m": 4}],
    }
    report = ground_materials.generate(document, raw_dir=tmp_path, local=True)
    assert report["cost"] == 0
    assert _sizes(fake_mflux) == {(1024, 1024)}
    assert (tmp_path / "loam_1.png").exists()
