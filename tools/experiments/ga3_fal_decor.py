"""GA3 decor pipeline v2 (probe S4): prompt -> image -> cut-out -> image-to-3D on fal.ai.

Run with the fal client (``FAL_KEY`` in the environment)::

    uv run --with fal-client python tools/experiments/ga3_fal_decor.py OUT_DIR \
        --prompt-file PROMPT.txt [--seed 1337] [--models trellis,trellis-2]

Each stage is cached in ``OUT_DIR`` (an existing output is never paid for twice):

1. ``src.png``: ``fal-ai/flux-2`` 1024x1024 (0.012 $/Mpx).
2. ``cut.png``: ``fal-ai/bria/background/remove`` (0.018 $), transparent background, so the
   3D model does not rebuild a ground patch.
3. ``trellis.glb`` (``fal-ai/trellis``, 0.02 $) and/or ``trellis-2.glb``
   (``fal-ai/trellis-2`` at 1024, 0.30 $, PBR textures).

Every raw fal response is kept as ``result_<stage>.json``.

Catalogue mode (lot GA3-L1, ``data/art/ga3_decor.json``) runs the same stages for every object
into ``RAW_DIR/<id>/``::

    uv run --with fal-client python tools/experiments/ga3_fal_decor.py RAW_DIR \
        --catalog data/art/ga3_decor.json [--only church,well] [--attempt 2]

The prompt is ``style_prefix + prompt + style_suffix``. ``model: multi`` (thin parts: cart,
palisade, siege engines) also asks ``fal-ai/flux-2/edit`` for a side and a back view of the
source image, cuts them out and calls ``fal-ai/trellis/multi`` on the three views
(``multi.glb``), besides the single-view ``trellis.glb`` for comparison. ``source: s4`` reuses
a probe's raw folder (no call). ``--attempt 2`` writes to ``<id>_2`` with another seed. The
estimated cost of every paid call is appended to ``RAW_DIR/costs.json``.
"""

import argparse
import json
import urllib.request
from pathlib import Path

import fal_client

# Catalogue prices (USD) used for the cost log: flux-2 1 Mpx, edit (1 Mpx in + out), bria,
# trellis, trellis-2 at 1024, trellis/multi.
PRICES = {
    "flux2": 0.012,
    "edit": 0.024,
    "bria": 0.018,
    "trellis": 0.02,
    "trellis-2": 0.30,
    "multi": 0.02,
}
VIEW_PROMPTS = {
    "side": "Show exactly the same object from its left side, seen straight from the side at "
    "eye level, same materials, same proportions, same lighting, same plain mid-grey "
    "background, whole object visible and centred, no ground.",
    "back": "Show exactly the same object from behind, seen from the back three-quarter "
    "opposite to the original view, same materials, same proportions, same lighting, "
    "same plain mid-grey background, whole object visible and centred, no ground.",
}
COST_LOG: list[dict] = []

TRELLIS_ARGS = {"texture_size": 1024, "mesh_simplify": 0.95}
TRELLIS2_ARGS = {
    "resolution": "1024",
    "texture_size": 2048,
    "decimation_target": 100000,
    "remesh": True,
}


def run(endpoint: str, arguments: dict, out_dir: Path, stage: str) -> dict:
    """Call ``endpoint`` synchronously and keep the raw response."""
    result = fal_client.subscribe(endpoint, arguments=arguments, with_logs=False)
    (out_dir / f"result_{stage}.json").write_text(json.dumps(result, indent=1))
    kind = stage.split("_")[0]
    COST_LOG.append({"dir": out_dir.name, "stage": stage, "usd": PRICES.get(kind, 0.0)})
    return result


def first_url(result: dict, keys: tuple[str, ...]) -> str:
    """URL of the first output file found under ``keys``."""
    for key in keys:
        value = result.get(key)
        if isinstance(value, list) and value:
            value = value[0]
        if isinstance(value, dict) and value.get("url"):
            return value["url"]
    raise KeyError(f"no output among {keys}: {json.dumps(result)[:400]}")


def image_stage(out_dir: Path, prompt: str, seed: int) -> Path:
    """``src.png`` (flux-2) then ``cut.png`` (bria), cached."""
    source = out_dir / "src.png"
    if not source.exists():
        result = run(
            "fal-ai/flux-2",
            {
                "prompt": prompt,
                "image_size": {"width": 1024, "height": 1024},
                "seed": seed,
                "output_format": "png",
            },
            out_dir,
            "flux2",
        )
        urllib.request.urlretrieve(first_url(result, ("images",)), source)
        print(f"GA3 image -> {source}")
    return cut_out(out_dir, source, "cut.png", "bria")


def cut_out(out_dir: Path, source: Path, name: str, stage: str) -> Path:
    """Background removal of ``source`` into ``out_dir/name``, cached."""
    cut = out_dir / name
    if not cut.exists():
        result = run(
            "fal-ai/bria/background/remove",
            {"image_url": fal_client.upload_file(str(source))},
            out_dir,
            stage,
        )
        urllib.request.urlretrieve(first_url(result, ("image", "images")), cut)
        print(f"GA3 cut-out -> {cut}")
    return cut


def views_stage(out_dir: Path, seed: int) -> list[Path]:
    """Side and back views regenerated from ``src.png`` (flux-2 edit), cut out."""
    source_url = None
    cuts = [out_dir / "cut.png"]
    for view, prompt in VIEW_PROMPTS.items():
        image = out_dir / f"view_{view}.png"
        if not image.exists():
            source_url = source_url or fal_client.upload_file(str(out_dir / "src.png"))
            result = run(
                "fal-ai/flux-2/edit",
                {
                    "prompt": prompt,
                    "image_urls": [source_url],
                    "image_size": {"width": 1024, "height": 1024},
                    "seed": seed,
                    "output_format": "png",
                },
                out_dir,
                f"edit_{view}",
            )
            urllib.request.urlretrieve(first_url(result, ("images",)), image)
            print(f"GA3 view {view} -> {image}")
        cuts.append(cut_out(out_dir, image, f"cut_{view}.png", f"bria_{view}"))
    return cuts


def model_stage(out_dir: Path, models: list[str], seed: int) -> None:
    """Image-to-3D calls (``trellis``, ``trellis-2``, ``multi``), cached."""
    cut_url = None
    for model in models:
        glb = out_dir / f"{model}.glb"
        if glb.exists():
            continue
        if model == "multi":
            urls = [fal_client.upload_file(str(p)) for p in views_stage(out_dir, seed)]
            endpoint = "fal-ai/trellis/multi"
            arguments = {
                "image_urls": urls,
                "multiimage_algo": "stochastic",
                "seed": seed,
                **TRELLIS_ARGS,
            }
        else:
            cut_url = cut_url or fal_client.upload_file(str(out_dir / "cut.png"))
            endpoint = f"fal-ai/{model}"
            extra = TRELLIS2_ARGS if model == "trellis-2" else TRELLIS_ARGS
            arguments = {"image_url": cut_url, "seed": seed, **extra}
        result = run(endpoint, arguments, out_dir, model)
        urllib.request.urlretrieve(
            first_url(result, ("model_glb", "model_mesh", "model_file")), glb
        )
        print(f"GA3 {model} -> {glb}")


def catalog_models(entry: dict) -> list[str]:
    """Models to call for a catalogue entry (``multi`` also gets a single-view trellis)."""
    return ["trellis", "multi"] if entry["model"] == "multi" else [entry["model"]]


def run_catalog(raw_dir: Path, catalog: Path, only: set[str], attempt: int) -> None:
    """Run the cached stages for every object of the catalogue."""
    document = json.loads(catalog.read_text(encoding="utf-8"))
    for entry in document["objects"]:
        if only and entry["id"] not in only:
            continue
        if entry.get("source"):
            print(f"GA3 {entry['id']}: reuses probe {entry['source']} (no call)")
            continue
        suffix = "" if attempt <= 1 else f"_{attempt}"
        out_dir = raw_dir / f"{entry['id']}{suffix}"
        out_dir.mkdir(parents=True, exist_ok=True)
        seed = int(entry.get("seed", 1337)) + 101 * (attempt - 1)
        prompt = " ".join(
            (document["style_prefix"], entry["prompt"], document["style_suffix"])
        )
        (out_dir / "prompt.txt").write_text(prompt + "\n")
        image_stage(out_dir, prompt, seed)
        model_stage(out_dir, catalog_models(entry), seed)


def main() -> None:
    """Run the cached stages (one prompt, or the whole catalogue)."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out_dir")
    parser.add_argument("--prompt-file", default="")
    parser.add_argument("--seed", type=int, default=1337)
    parser.add_argument("--models", default="trellis,trellis-2")
    parser.add_argument("--catalog", default="")
    parser.add_argument("--only", default="")
    parser.add_argument("--attempt", type=int, default=1)
    args = parser.parse_args()
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    try:
        if args.catalog:
            only = set(filter(None, args.only.split(",")))
            run_catalog(out_dir, Path(args.catalog), only, args.attempt)
        else:
            if not args.prompt_file:
                parser.error("--prompt-file or --catalog is required")
            prompt = Path(args.prompt_file).read_text().strip()
            image_stage(out_dir, prompt, args.seed)
            model_stage(out_dir, list(filter(None, args.models.split(","))), args.seed)
    finally:
        if COST_LOG:
            log_path = out_dir / "costs.json"
            previous = json.loads(log_path.read_text()) if log_path.exists() else []
            log_path.write_text(json.dumps(previous + COST_LOG, indent=1))
            print(f"GA3 cost of this run: {sum(c['usd'] for c in COST_LOG):.3f} $")


if __name__ == "__main__":
    main()
