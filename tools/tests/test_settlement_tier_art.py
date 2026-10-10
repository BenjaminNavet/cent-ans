"""Light tests of the CO-D settlement tier prompts and paths (no model run)."""

from cent_ans_tools import settlement_tier_art as art


def test_prompts_cover_every_kind_and_tier():
    """Five kinds, six stages each, and the style rules out text."""
    prompts = art.load_prompts()
    for kind in art.KINDS:
        assert len(prompts["kinds"][kind]["tiers"]) == art.TIER_COUNT
        assert "No text" in art.tier_prompt(kind, 1, prompts)
    assert len(art.jobs(art.KINDS, False, prompts)) == 30


def test_tier_paths_follow_convention():
    """Paths are settlement_tiers/<kind>_<n>.jpg."""
    assert art.tier_path("city", 3).as_posix().endswith("settlement_tiers/city_3.jpg")


def test_stages_of_a_kind_share_one_seed():
    """Same seed keeps the composition across stages."""
    seeds = {seed for label, _, seed in art.jobs(("town",), False, art.load_prompts())}
    assert len(seeds) == 1


def test_building_prompts_use_data_and_exist_on_disk():
    """The eight missing buildings have a scene and a data entry."""
    prompts = art.load_prompts()
    assert len(prompts["buildings"]) == 8
    prompt = art.building_prompt("bld_town_hall", prompts)
    assert "Hôtel de ville" in prompt
