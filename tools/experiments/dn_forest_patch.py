"""DN-forets test: generate whole forest patches (one mesh per block of woodland) instead of trees.

Z-Image Turbo (fal) -> rembg cut -> TRELLIS (fal), outputs outside the repo in
``~/dev/cent-ans-raw/dn/forets/<id>/`` (``img/s<seed>.png``, ``cut/s<seed>.png``, ``3d/s<seed>.glb``).
Spend is appended to ``~/dev/cent-ans-raw/dn/fal_spend.jsonl``. Re-running skips existing outputs.

Usage::

    source <(grep '^export FAL_KEY' ~/.zshrc)
    uv run --with rembg --with onnxruntime --with fal-client --with pillow \
        python tools/experiments/dn_forest_patch.py [--seeds 2] [--local]
"""

import argparse
import json
import subprocess
import time
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path.home() / "dev/cent-ans-raw/dn"
OUT = ROOT / "forets"
IMAGE_ENDPOINT = "fal-ai/z-image/turbo"
TRELLIS_ENDPOINT = "fal-ai/trellis"
REPO = Path(__file__).resolve().parents[2]
MFLUX_MODEL = Path.home() / "models/mflux/z-image-turbo-q8"
SF3D_DIR = Path.home() / "dev/cent-ans-raw/sf3d/stable-fast-3d"

COMMON = (
    "Photorealistic aerial reference photograph of an isolated square block of dense natural forest, "
    "seen from high above at a 45 degree oblique angle, like a cut-out diorama tile of woodland: "
    "dozens of mature trees of varied heights with touching crowns forming one continuous lumpy canopy, "
    "irregular soft natural edge with a few smaller trees at the border, no clearing, no path, "
    "no buildings, no people, no animals, no water. Muted natural desaturated colours, albedo like an "
    "overcast day, soft even diffuse light, no baked directional shadow. The whole forest block fully "
    "visible, centred, filling about 85 percent of the frame, on a plain uniform medium grey studio "
    "background, no ground plane around it, no cast shadow."
)
PATCHES = {
    "forest_broadleaf": "Temperate western European broadleaf forest of pedunculate oak, beech and hornbeam, "
    "summer foliage in olive and dark greens with slight variation between crowns.",
    "forest_conifer": "Mountain conifer forest of silver fir and Norway spruce, tall dark blue-green "
    "pointed crowns packed together, a few larches.",
    "forest_mediterranean": "Mediterranean woodland of holm oak, Aleppo pine and umbrella pine, dark grey-green "
    "rounded crowns, sparse dry ochre undergrowth visible between some trees.",
}


def log_spend(entry_id: str, endpoint: str, seed: int, usd: float) -> None:
    """Append one fal call to the shared spend log."""
    with (ROOT / "fal_spend.jsonl").open("a") as handle:
        handle.write(
            json.dumps(
                {
                    "ts": time.strftime("%Y-%m-%dT%H:%M:%S"),
                    "id": f"forets/{entry_id}",
                    "endpoint": endpoint,
                    "seed": seed,
                    "usd": usd,
                }
            )
            + "\n"
        )


def local_image(entry_id: str, seed: int, target: Path) -> None:
    """Z-Image Turbo in mflux under the shared GPU lock (free)."""
    prompt_file = target.parent / "prompt.txt"
    prompt_file.write_text(f"{COMMON} {PATCHES[entry_id]}")
    command = [
        str(REPO / "tools/gpu_lock.sh"), "mflux-generate-z-image-turbo", "--model", str(MFLUX_MODEL),
        "--base-model", "z-image-turbo", "--prompt-file", str(prompt_file), "--steps", "9",
        "--seed", str(seed), "--width", "1024", "--height", "1024", "--output", str(target),
    ]  # fmt: skip
    subprocess.run(command, check=True, capture_output=True)


def local_3d(cut: Path, target: Path, seed: int) -> None:
    """Free TRELLIS Space first, SF3D (MPS, GPU lock) when the Space fails."""
    done = subprocess.run(
        ["uv", "run", "--with", "gradio_client", "python", str(REPO / "tools/experiments/trellis_hf.py"),
         str(target.parent), f"hf_{target.stem}", str(cut), "--seeds", str(seed)],
        capture_output=True, text=True,
    )  # fmt: skip
    produced = target.parent / f"hf_{target.stem}__s{seed}.glb"
    if done.returncode == 0 and produced.exists():
        produced.rename(target)
        return
    print(f"HF failed for {cut}: {(done.stderr or done.stdout)[-160:]}", flush=True)
    work = target.parent / f"sf3d_s{seed}"
    subprocess.run(
        [str(REPO / "tools/gpu_lock.sh"), str(SF3D_DIR.parent / ".venv/bin/python"), "run.py", str(cut), "--output-dir", str(work)],
        cwd=SF3D_DIR, check=True, capture_output=True,
    )  # fmt: skip
    target.with_name(f"sf3d_{target.name}").write_bytes(
        (work / "0" / "mesh.glb").read_bytes()
    )


def generate(entry_id: str, seed: int, local: bool = False) -> str:
    """Image, cut and 3D for one patch and seed; returns a status line."""
    from PIL import Image
    from rembg import remove

    work = OUT / entry_id
    for sub in ("img", "cut", "3d"):
        (work / sub).mkdir(parents=True, exist_ok=True)
    image_path = work / "img" / f"s{seed}.png"
    cut_path = work / "cut" / f"s{seed}.png"
    glb_path = work / "3d" / f"s{seed}.glb"
    if not image_path.exists() and local:
        local_image(entry_id, seed, image_path)
    if not image_path.exists():
        import fal_client

        result = fal_client.subscribe(
            IMAGE_ENDPOINT,
            arguments={
                "prompt": f"{COMMON} {PATCHES[entry_id]}",
                "image_size": {"width": 1024, "height": 1024},
                "seed": seed,
                "enable_prompt_expansion": False,
                "output_format": "png",
                "enable_safety_checker": False,
                "num_images": 1,
            },
        )
        urllib.request.urlretrieve(result["images"][0]["url"], image_path)  # noqa: S310
        log_spend(entry_id, IMAGE_ENDPOINT, seed, 0.005)
    if not cut_path.exists():
        remove(Image.open(image_path)).save(cut_path)
    if (
        local
        and not glb_path.exists()
        and not glb_path.with_name(f"sf3d_{glb_path.name}").exists()
    ):
        local_3d(cut_path, glb_path, seed)
    if not local and not glb_path.exists():
        import fal_client

        url = fal_client.upload_file(str(cut_path))
        result = fal_client.subscribe(
            TRELLIS_ENDPOINT,
            arguments={
                "image_url": url,
                "seed": seed,
                "texture_size": 1024,
                "mesh_simplify": 0.95,
            },
        )
        urllib.request.urlretrieve(result["model_mesh"]["url"], glb_path)  # noqa: S310
        log_spend(entry_id, TRELLIS_ENDPOINT, seed, 0.02)
    return f"{entry_id} s{seed}: {glb_path}"


def main() -> None:
    """Generate every patch for the requested number of seeds."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--seeds", type=int, default=2)
    parser.add_argument("--only", default="")
    parser.add_argument(
        "--local", action="store_true", help="free chain: mflux, TRELLIS HF, SF3D"
    )
    args = parser.parse_args()
    jobs = [
        (entry_id, seed)
        for entry_id in PATCHES
        for seed in range(1, args.seeds + 1)
        if not args.only or entry_id in args.only.split(",")
    ]
    with ThreadPoolExecutor(max_workers=4) as pool:
        for line in pool.map(lambda job: generate(*job, local=args.local), jobs):
            print(line, flush=True)


if __name__ == "__main__":
    main()
