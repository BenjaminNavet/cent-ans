"""Free TRELLIS image->3D on the Hugging Face Space ``trellis-community/TRELLIS`` (ADR 0210).

Best-of-N: one glb per seed, the caller keeps the best. Single view by default; several
images are sent together in multi-image mode (only worth it when the views really differ)::

    uv run --with gradio_client python tools/experiments/trellis_hf.py OUT_DIR NAME \
        IMAGE [IMAGE ...] [--seeds 1337,1338,1339] [--algo stochastic|multidiffusion]

Writes ``OUT_DIR/NAME__s<seed>.glb``. Needs ``hf auth login`` (ZeroGPU daily quota, roughly
3-8 objects a day). Inputs: cut-out PNG (transparent background), object filling the frame,
1024x1024 is ideal.

Pitfall: the Space's API path calls ``generate_and_extract_glb`` with
``preprocess_image=False`` and reads the multi-image mode from the UI tab, so every view goes
through ``/preprocess_images`` (alpha bbox crop x1.2, 518x518) and ``/lambda_1`` selects the
multi-image tab.
"""

import argparse
import shutil
import time
from pathlib import Path

from gradio_client import Client, handle_file

SPACE = "trellis-community/TRELLIS"


def main() -> None:
    """Generate one glb per seed from the given views."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out_dir")
    parser.add_argument("name")
    parser.add_argument("images", nargs="+")
    parser.add_argument("--seeds", default="1337,1338,1339")
    parser.add_argument(
        "--algo", default="stochastic", choices=["stochastic", "multidiffusion"]
    )
    args = parser.parse_args()
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    client = Client(SPACE, verbose=False)
    client.predict(api_name="/start_session")
    multi = len(args.images) > 1
    if multi:
        client.predict(api_name="/lambda_1")
    gallery = client.predict(
        images=[{"image": handle_file(p), "caption": None} for p in args.images],
        api_name="/preprocess_images",
    )
    prepped = [{"image": handle_file(g["image"]), "caption": None} for g in gallery]
    for seed in (int(s) for s in args.seeds.split(",")):
        started = time.time()
        _, _, glb = client.predict(
            image=handle_file(gallery[0]["image"]),
            multiimages=prepped if multi else [],
            seed=seed,
            multiimage_algo=args.algo,
            mesh_simplify=0.95,
            texture_size=1024,
            api_name="/generate_and_extract_glb",
        )
        target = out_dir / f"{args.name}__s{seed}.glb"
        shutil.copy(glb, target)
        print(f"{target} {round(time.time() - started)} s", flush=True)


if __name__ == "__main__":
    main()
