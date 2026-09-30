"""HB5: landscape-scale rock outcrops for the campaign map (ADR 0143).

Steps (raw files in ``~/dev/cent-ans-raw/hb/rocks/``, run from the repository root)::

    uv run --with fal-client --with pyyaml python tools/blender_scripts/hb_rock_outcrops.py fal [ids]
    uv run --with pyyaml python tools/blender_scripts/hb_rock_outcrops.py cleanup [ids]

* ``fal``: per outcrop of ``data/art/rock_outcrops.yaml`` the GA3 chain: one ``fal-ai/flux-2``
  view on grey, one ``fal-ai/bria/background/remove``, one ``fal-ai/trellis`` (0.05 $ per
  outcrop). Skips outcrops already downloaded; ``--retry ID`` regenerates one outcrop with
  seed + 1 (one retry per object at most, kept as ``<id>_s<seed>``).
* ``cleanup``: ``ga3_cleanup.py`` (Blender) on the retained TRELLIS mesh: longest horizontal
  side 1 m (the layer scales to ``size_m``), LOD0 / 1 / 2 = 100 / 50 / 15 % of
  ``cleanup.lod0`` triangles, albedo ``cleanup.tex`` px, graded by ``exposure``.

Outputs ``game/assets/models/rocks/hb/<id>_lod{0,1,2}.glb``, consumed by
``game/scripts/map/rock_outcrops.gd``.
"""

import json
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path

import yaml

RAW = Path.home() / "dev/cent-ans-raw/hb/rocks"
REPO = Path(__file__).resolve().parents[2]
CATALOGUE = REPO / "data/art/rock_outcrops.yaml"
OUT_MODELS = REPO / "game/assets/models/rocks/hb"
TRELLIS_ARGS = {"texture_size": 1024, "mesh_simplify": 0.95}
# Prices (fal pricing API, 2026-09-30, as in ga3_vegetation_l2.py).
PRICE = {
    "fal-ai/flux-2": 0.012,  # per megapixel
    "fal-ai/bria/background/remove": 0.018,
    "fal-ai/trellis": 0.02,
}


def catalogue() -> dict:
    """The outcrop catalogue (YAML, JSON flow syntax)."""
    return yaml.safe_load(CATALOGUE.read_text(encoding="utf-8"))


def _download(url: str, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    urllib.request.urlretrieve(url, path)


def _log() -> tuple[Path, dict]:
    RAW.mkdir(parents=True, exist_ok=True)
    path = RAW / "fal_log.json"
    return path, (json.loads(path.read_text()) if path.exists() else {})


def fal_step(only: list[str], retry: str = "") -> None:
    """Image, background removal and TRELLIS mesh per outcrop."""
    import fal_client

    document = catalogue()
    log_path, log = _log()
    cost = 0.0

    def run(app: str, args: dict, units: float = 1.0) -> dict:
        nonlocal cost
        result = fal_client.subscribe(app, arguments=args)
        cost += PRICE[app] * units
        return result

    for entry in document["outcrops"]:
        name = entry["id"]
        seed = int(entry.get("seed", 1337))
        tag = name
        if retry:
            if name != retry:
                continue
            seed += 1
            tag = f"{name}_s{seed}"
        elif only and name not in only:
            continue
        if (RAW / f"{tag}_trellis.glb").exists():
            continue
        prompt = entry["prompt"] + document["style_suffix"]
        src = run(
            "fal-ai/flux-2",
            {
                "prompt": prompt,
                "image_size": "square_hd",
                "num_images": 1,
                "seed": seed,
                "output_format": "png",
            },
        )["images"][0]["url"]
        _download(src, RAW / f"{tag}_src.png")
        cut = run("fal-ai/bria/background/remove", {"image_url": src})["image"]["url"]
        _download(cut, RAW / f"{tag}_cut.png")
        glb = run("fal-ai/trellis", {"image_url": cut, **TRELLIS_ARGS, "seed": seed})[
            "model_mesh"
        ]["url"]
        _download(glb, RAW / f"{tag}_trellis.glb")
        log[tag] = {"prompt": prompt, "seed": seed, "src": src, "cut": cut, "glb": glb}
        print("FAL", tag, "ok", flush=True)
    log["_cost_usd"] = round(float(log.get("_cost_usd", 0.0)) + cost, 4)
    log_path.write_text(json.dumps(log, indent=1))
    print(f"COST {cost:.3f} $ (total {log['_cost_usd']} $)")


def retained_mesh(name: str) -> Path:
    """The retained raw mesh: the retry (``<id>_s*``) if one exists, else the first run."""
    retries = sorted(RAW.glob(f"{name}_s*_trellis.glb"))
    return retries[-1] if retries else RAW / f"{name}_trellis.glb"


def cleanup_step(only: list[str]) -> None:
    """``ga3_cleanup.py`` on each retained TRELLIS mesh (1 m long, three LODs)."""
    document = catalogue()
    clean = document["cleanup"]
    blender = (
        shutil.which("blender") or "/Applications/Blender.app/Contents/MacOS/Blender"
    )
    OUT_MODELS.mkdir(parents=True, exist_ok=True)
    for entry in document["outcrops"]:
        name = entry["id"]
        if only and name not in only:
            continue
        source = retained_mesh(name)
        if not source.exists():
            print(name, "missing", source)
            continue
        stats = RAW / f"{name}_stats.json"
        cmd = [
            blender, "-b", "--factory-startup", "--python", str(REPO / "tools/blender_scripts/ga3_cleanup.py"), "--",
            str(source), str(OUT_MODELS), name, "--length", str(clean["length"]), "--lod0", str(clean["lod0"]),
            "--tex", str(clean["tex"]), "--normal", "off", "--roughness", "off",
            "--strip-base", str(clean.get("strip_base", 0.0)), "--align",
            "--exposure", str(entry.get("exposure", 1.0)), "--stats", str(stats),
        ]  # fmt: skip
        result = subprocess.run(cmd, capture_output=True, text=True, check=False)
        lines = [
            line
            for line in result.stdout.splitlines()
            if "tri" in line.lower() or "error" in line.lower() or "grade" in line
        ]
        print(name, source.name, result.returncode, *lines[-6:], sep="\n  ")
    print("OK cleanup")


if __name__ == "__main__":
    step = sys.argv[1] if len(sys.argv) > 1 else ""
    args = sys.argv[2:]
    if step == "fal":
        if args[:1] == ["--retry"]:
            fal_step([], retry=args[1])
        else:
            fal_step(args)
    elif step == "cleanup":
        cleanup_step(args)
    else:
        sys.exit("usage: hb_rock_outcrops.py fal [ids | --retry ID] | cleanup [ids]")
