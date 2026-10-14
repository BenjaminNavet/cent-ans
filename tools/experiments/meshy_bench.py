"""MX: Meshy bench on fal -- multi-view generation and retexture against the shipped TRELLIS 1.

``uv run --project tools --with fal-client python tools/experiments/meshy_bench.py run``
then ``... meshy_bench.py sheets`` (Blender headless, one comparison sheet per object).

``run`` calls ``meshy/v7.1/multi-image-to-3d`` (``gen``) on our cut views and
``meshy/v6-lite/retexture`` (``retex``) on the shipped lod0 glb, keeping its UVs. A finished
output (glb present) is skipped. Every paid call is appended to ``RAW_DIR/costs.json`` and a call
that would push the total over ``CAP_USD`` is refused.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import urllib.request
from pathlib import Path

try:
    import fal_client
except ImportError:  # sheets only
    fal_client = None

REPO = Path(__file__).resolve().parents[2]
DN_RAW = Path.home() / "dev/cent-ans-raw/dn"
RAW_DIR = Path.home() / "dev/cent-ans-raw/meshy"
MODELS = REPO / "game/assets/models/dn"
CAP_USD = 8.0
PRICES = {"gen": 1.20, "retex": 0.30}
GEN_ENDPOINT = "meshy/v7.1/multi-image-to-3d"
RETEX_ENDPOINT = "meshy/v6-lite/retexture"
PROMPT_MAX = 600  # Meshy text_style_prompt limit

# object id -> (model sub-folder, run gen)
BENCH = {
    "city_west": ("buildings", True),
    "castle_west": ("buildings", True),
    "siege_tower": ("siege", True),
    "siege_trebuchet_counterweight": ("siege", True),
    "bombard": ("siege", False),
    "abbey_west": ("buildings", False),
    "animal_horse_draught": ("fauna", False),
}


def spent() -> float:
    """Total already logged in ``costs.json``."""
    path = RAW_DIR / "costs.json"
    return sum(e["usd"] for e in json.loads(path.read_text())) if path.exists() else 0.0


def log_cost(object_id: str, kind: str) -> None:
    """Append one paid call to ``costs.json``."""
    path = RAW_DIR / "costs.json"
    entries = json.loads(path.read_text()) if path.exists() else []
    entries.append({"id": object_id, "kind": kind, "usd": PRICES[kind]})
    path.write_text(json.dumps(entries, indent=1))


def views(object_id: str) -> list[Path]:
    """Front cut, then back and side cuts (multi-view folder first), at most 4."""
    src = DN_RAW / object_id
    found = [src / "cut/s1337.png"]
    for name in ("back_cut_s1337.png", "side_cut_s1337.png"):
        for folder in (src / "mv/views", src / "views"):
            if (folder / name).exists():
                found.append(folder / name)
                break
    return [p for p in found if p.exists()][:4]


def style_prompt(object_id: str) -> str:
    """The DN image prompt, cut to Meshy's limit on a word boundary."""
    text = (DN_RAW / object_id / "prompt.txt").read_text().strip()
    return text if len(text) <= PROMPT_MAX else text[:PROMPT_MAX].rsplit(" ", 1)[0]


def call(object_id: str, kind: str, endpoint: str, arguments: dict) -> None:
    """Run one paid call under the cap, save the raw response and the glb."""
    out_dir = RAW_DIR / object_id
    target = out_dir / f"{kind}.glb"
    if target.exists():
        print(f"skip {object_id} {kind} (done)")
        return
    if spent() + PRICES[kind] > CAP_USD + 1e-9:
        raise SystemExit(
            f"cap {CAP_USD} $ reached ({spent():.2f} $), {object_id} {kind} refused"
        )
    out_dir.mkdir(parents=True, exist_ok=True)
    print(
        f"call {object_id} {kind} ({PRICES[kind]:.2f} $, spent {spent():.2f} $)",
        flush=True,
    )
    log_cost(
        object_id, kind
    )  # logged before the call: a failure after submission may still bill
    result = fal_client.subscribe(endpoint, arguments=arguments, with_logs=False)
    (out_dir / f"result_{kind}.json").write_text(json.dumps(result, indent=1))
    urllib.request.urlretrieve(result["model_glb"]["url"], target)


def run(only: set[str]) -> None:
    """Every bench call not done yet."""
    for object_id, (folder, do_gen) in BENCH.items():
        if only and object_id not in only:
            continue
        cut = views(object_id)
        image_url = fal_client.upload_file(str(cut[0]))
        if do_gen:
            call(
                object_id,
                "gen",
                GEN_ENDPOINT,
                {
                    "image_urls": [image_url]
                    + [fal_client.upload_file(str(p)) for p in cut[1:]],
                    "should_texture": True,
                    "enable_pbr": False,
                    "should_remesh": True,
                    "topology": "triangle",
                    "target_polycount": 30000,
                    "texture_prompt": style_prompt(object_id),
                },
            )
        glb = MODELS / folder / f"{object_id}_lod0.glb"
        call(
            object_id,
            "retex",
            RETEX_ENDPOINT,
            {
                "model_url": fal_client.upload_file(str(glb)),
                "image_style_url": image_url,
                "text_style_prompt": style_prompt(object_id),
                "enable_original_uv": True,
                "enable_pbr": False,
                "texture_resolution": "2k",
            },
        )
    print(f"total {spent():.2f} $")


def sheets(only: set[str]) -> None:
    """One Blender sheet per object: references, T1, T1 + retexture, Meshy generation."""
    out = RAW_DIR / "sheets"
    out.mkdir(parents=True, exist_ok=True)
    for object_id, (folder, _) in BENCH.items():
        if only and object_id not in only:
            continue
        cut = views(object_id)
        cmd = [
            "blender",
            "-b",
            "--factory-startup",
            "-P",
            str(REPO / "tools/blender_scripts/ga3_compare_render.py"),
            "--",
            "--out",
            str(out / f"{object_id}.jpg"),
            "--stats",
            str(out / f"{object_id}_stats.json"),
            "--fit-width",
            "1.8",
            "--ref",
            ":".join(str(p) for p in cut[:2]),
            "Référence",
            "--panel",
            str(MODELS / folder / f"{object_id}_lod0.glb"),
            "T1 livré",
        ]
        for kind, label in (("retex", "T1 + retexture Meshy"), ("gen", "Meshy v7.1")):
            if (RAW_DIR / object_id / f"{kind}.glb").exists():
                cmd += ["--panel", str(RAW_DIR / object_id / f"{kind}.glb"), label]
        subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL)
        print(f"sheet {out / f'{object_id}.jpg'}")


def main() -> None:
    """CLI entry point."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("action", choices=["run", "sheets"])
    parser.add_argument("--only", default="", help="comma-separated object ids")
    args = parser.parse_args()
    only = {s for s in args.only.split(",") if s}
    if args.action == "run":
        if fal_client is None:
            raise SystemExit("fal-client missing: uv run --with fal-client")
        run(only)
    else:
        sheets(only)


if __name__ == "__main__":
    main()
