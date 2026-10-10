"""Settlement tier and missing building illustrations, free local mflux (CO-D, ADR 0190).

30 tier images ``settlement_tiers/<kind>_<n>.jpg`` (5 kinds x 6 stages of one growing
locality, same viewpoint) and the building images still missing. Prompts live in
``data/art/settlement_tiers.json``; every stage of a kind shares one seed so the
composition stays the same from stage to stage (best-of-N: ``--seed-offset``).

Run: ``uv run --project tools python -m cent_ans_tools.settlement_tier_art --kinds city``
"""

from __future__ import annotations

import json
import zlib
from pathlib import Path

from cent_ans_tools import entry_art, local_art
from cent_ans_tools.portraits import DATA_DIR, REPO_DIR

TIERS_DIR = entry_art.ILLUSTRATIONS_DIR / "settlement_tiers"
PROMPTS_PATH = DATA_DIR / "art" / "settlement_tiers.json"
KINDS = ("city", "town", "castle", "abbey", "village")
TIER_COUNT = 6
CANDIDATES_DIR = REPO_DIR / "tmp" / "co_candidates"


def load_prompts(path: Path = PROMPTS_PATH) -> dict:
    """The prompt data file."""
    return json.loads(path.read_text(encoding="utf-8"))


def tier_path(kind: str, tier: int, out_dir: Path = TIERS_DIR) -> Path:
    """Output path of one tier image."""
    return out_dir / f"{kind}_{tier}.jpg"


def tier_prompt(kind: str, tier: int, prompts: dict) -> str:
    """Prompt of stage ``tier`` (1..6) of ``kind``."""
    entry = prompts["kinds"][kind]
    return "\n".join([entry["subject"], entry["tiers"][tier - 1], prompts["style"]])


def building_prompt(building_id: str, prompts: dict, data_dir: Path = DATA_DIR) -> str:
    """Prompt of a building image: name and description from its data, plus a scene."""
    entry = json.loads(
        (data_dir / "buildings" / f"{building_id}.json").read_text(encoding="utf-8")
    )
    base = entry_art.build_prompt("buildings", entry)
    return f"{prompts['buildings'][building_id]}\n{base}"


def kind_seed(kind: str, offset: int = 0) -> int:
    """One seed per kind (shared by its six stages), shifted for best-of-N."""
    return zlib.crc32(f"co-tiers-{kind}".encode()) + offset


def building_seed(building_id: str, offset: int = 0) -> int:
    """Seed of a building image."""
    return zlib.crc32(building_id.encode()) + offset


def render(prompt: str, seed: int, destination: Path) -> None:
    """Render with mflux (16:9), crop to 640x360 and write the JPEG."""
    png = local_art.render_image(prompt, aspect_ratio="16:9", seed=seed)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(entry_art.convert(png))


def jobs(
    kinds: tuple[str, ...], buildings: bool, prompts: dict
) -> list[tuple[str, str, int]]:
    """(label, prompt, base seed) for the requested kinds and the building list."""
    result = []
    for kind in kinds:
        for tier in range(1, TIER_COUNT + 1):
            result.append(
                (f"{kind}_{tier}", tier_prompt(kind, tier, prompts), kind_seed(kind))
            )
    if buildings:
        for building_id in prompts["buildings"]:
            result.append(
                (
                    building_id,
                    building_prompt(building_id, prompts),
                    building_seed(building_id),
                )
            )
    return result


def main(argv: list[str] | None = None) -> None:
    """CLI: render candidates into ``tmp/co_candidates/s<offset>/`` for later choice."""
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--kinds", nargs="*", default=[], choices=KINDS)
    parser.add_argument("--buildings", action="store_true")
    parser.add_argument("--seed-offsets", nargs="+", type=int, default=[0])
    parser.add_argument("--only", nargs="*", help="labels, e.g. city_3 bld_town_hall")
    parser.add_argument("--out", type=Path, default=CANDIDATES_DIR)
    args = parser.parse_args(argv)
    prompts = load_prompts()
    for offset in args.seed_offsets:
        for label, prompt, seed in jobs(tuple(args.kinds), args.buildings, prompts):
            if args.only and label not in args.only:
                continue
            target = args.out / f"s{offset}" / f"{label}.jpg"
            if target.exists():
                continue
            render(prompt, seed + offset, target)
            print(target, flush=True)


if __name__ == "__main__":
    main()
