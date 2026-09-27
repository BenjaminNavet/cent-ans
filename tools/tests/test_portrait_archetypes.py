"""Tests for the DA2 living portraits bank (data, prompts, planning, reference image)."""

import base64
import io
import json
from pathlib import Path

import httpx
import pytest
from jsonschema import Draft202012Validator
from PIL import Image

from cent_ans_tools import openrouter, portrait_archetypes, portraits

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _config() -> dict:
    return portrait_archetypes.load_config()


def test_config_matches_schema() -> None:
    """archetypes.json validates against its schema."""
    schema = json.loads(
        (DATA / "schemas" / "portrait_archetypes.schema.json").read_text("utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_config()))
    assert not errors, [error.message for error in errors]


def test_references_are_consistent() -> None:
    """Cells, fallbacks, factions and aged variants point at known ids."""
    config = _config()
    ranks = {rank["id"] for rank in config["ranks"]}
    cultures = {culture["id"] for culture in config["cultures"]} | {"any"}
    for cell in config["cells"]:
        assert cell["rank"] in ranks
        assert set(cell["cultures"]) <= cultures
        for sex in cell["sexes"]:
            rank = next(r for r in config["ranks"] if r["id"] == cell["rank"])
            assert sex in rank["prompt"], (cell["rank"], sex)
            assert cell["faces"] <= len(config["faces"][sex])
    for source, target in config["fallbacks"]["rank"].items():
        assert source in ranks and target in ranks
    assert set(config["rank_rules"]["data_roles"].values()) <= ranks
    factions = {path.stem for path in (DATA / "factions").glob("*.json")}
    covered = [f for c in config["cultures"] for f in c["factions"]]
    assert len(covered) == len(set(covered)), "a faction belongs to one culture only"
    assert set(covered) == factions, set(covered) ^ factions
    for variant in config["aged_variants"]:
        character = variant["character"]
        assert (DATA / "characters" / f"{character}.json").is_file(), character
        assert (
            ROOT / "game" / "assets" / "portraits" / f"{character}.png"
        ).is_file(), character
    keys = [f"{v['character']}_{v['band']}" for v in config["aged_variants"]]
    assert len(keys) == len(set(keys))


def test_bank_size_and_unique_keys() -> None:
    """About 100-220 archetypes (lot spec + crowded-cell faces), all distinct."""
    specs = portrait_archetypes.iter_archetypes(_config())
    assert 100 <= len(specs) <= 220
    assert len({spec.key for spec in specs}) == len(specs)


def test_band_for_age() -> None:
    """Age bands follow max_age."""
    config = _config()
    assert portrait_archetypes.band_for_age(config, 8) == "child"
    assert portrait_archetypes.band_for_age(config, 16) == "young"
    assert portrait_archetypes.band_for_age(config, 49) == "adult"
    assert portrait_archetypes.band_for_age(config, 80) == "old"


def test_archetype_prompt_is_data_driven_and_ends_with_style() -> None:
    """Prompt carries rank, age, face, dress, no arms, then the shared STYLE block."""
    config = _config()
    spec = portrait_archetypes.ArchetypeSpec("knight", "male", "old", "england", 1)
    prompt = portrait_archetypes.build_archetype_prompt(config, spec)
    assert "knight" in prompt
    assert config["faces"]["male"][1] in prompt
    assert "grey or white" in prompt
    assert "English" in prompt
    assert "No coat of arms" in prompt
    assert prompt.endswith(portraits.STYLE)


def test_plan_skips_existing_and_attaches_reference(tmp_path: Path) -> None:
    """Idempotent plan; aged variants carry the 1337 portrait as reference."""
    config = _config()
    first = portrait_archetypes.plan(config, assets_dir=tmp_path)
    assert len(first) == len(portrait_archetypes.iter_archetypes(config)) + len(
        config["aged_variants"]
    )
    done = first[0]
    done.out_path.parent.mkdir(parents=True, exist_ok=True)
    done.out_path.write_bytes(b"x")
    again = portrait_archetypes.plan(config, assets_dir=tmp_path)
    assert done.character_id not in {job.character_id for job in again}
    real = portrait_archetypes.plan(
        config, archetypes=False, only=["chr_edward_iii_adult"]
    )
    if real:  # absent once generated
        assert real[0].reference is not None and real[0].reference.is_file()
        assert "same person as the reference portrait" in real[0].prompt


def test_to_archetype_jpg_is_square_512() -> None:
    """Output is a 512x512 JPEG."""
    buffer = io.BytesIO()
    Image.new("RGB", (600, 900), (10, 20, 30)).save(buffer, "PNG")
    out = portrait_archetypes.to_archetype_jpg(buffer.getvalue())
    image = Image.open(io.BytesIO(out))
    assert image.format == "JPEG" and image.size == (512, 512)


def test_request_image_sends_reference(monkeypatch: pytest.MonkeyPatch) -> None:
    """Reference pictures go as image_url parts next to the text prompt."""
    monkeypatch.setenv("OPENROUTER_API_KEY", "test-key")
    buffer = io.BytesIO()
    Image.new("RGB", (8, 8)).save(buffer, "PNG")
    png = buffer.getvalue()
    seen: list[dict] = []

    def handler(request: httpx.Request) -> httpx.Response:
        seen.append(json.loads(request.read()))
        url = "data:image/png;base64," + base64.b64encode(png).decode()
        return httpx.Response(
            200,
            json={
                "choices": [{"message": {"images": [{"image_url": {"url": url}}]}}],
                "usage": {"cost": 0.01},
            },
        )

    with httpx.Client(transport=httpx.MockTransport(handler)) as client:
        openrouter.request_image("vendor/imager", "hello", client, images=[png])
    content = seen[0]["messages"][0]["content"]
    assert content[0] == {"type": "text", "text": "hello"}
    assert content[1]["image_url"]["url"].startswith("data:image/png;base64,")
