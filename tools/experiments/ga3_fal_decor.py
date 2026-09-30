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
"""

import argparse
import json
import urllib.request
from pathlib import Path

import fal_client

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


def main() -> None:
    """Run the cached stages."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out_dir")
    parser.add_argument("--prompt-file", required=True)
    parser.add_argument("--seed", type=int, default=1337)
    parser.add_argument("--models", default="trellis,trellis-2")
    args = parser.parse_args()
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    source = out_dir / "src.png"
    if not source.exists():
        prompt = Path(args.prompt_file).read_text().strip()
        result = run(
            "fal-ai/flux-2",
            {
                "prompt": prompt,
                "image_size": {"width": 1024, "height": 1024},
                "seed": args.seed,
                "output_format": "png",
            },
            out_dir,
            "flux2",
        )
        urllib.request.urlretrieve(first_url(result, ("images",)), source)
        print(f"GA3 image -> {source}")

    cut = out_dir / "cut.png"
    if not cut.exists():
        result = run(
            "fal-ai/bria/background/remove",
            {"image_url": fal_client.upload_file(str(source))},
            out_dir,
            "bria",
        )
        urllib.request.urlretrieve(first_url(result, ("image", "images")), cut)
        print(f"GA3 cut-out -> {cut}")

    cut_url = None
    for model in filter(None, args.models.split(",")):
        glb = out_dir / f"{model}.glb"
        if glb.exists():
            continue
        cut_url = cut_url or fal_client.upload_file(str(cut))
        extra = TRELLIS2_ARGS if model == "trellis-2" else TRELLIS_ARGS
        result = run(
            f"fal-ai/{model}",
            {"image_url": cut_url, "seed": args.seed, **extra},
            out_dir,
            model,
        )
        urllib.request.urlretrieve(
            first_url(result, ("model_glb", "model_mesh", "model_file")), glb
        )
        print(f"GA3 {model} -> {glb}")


if __name__ == "__main__":
    main()
