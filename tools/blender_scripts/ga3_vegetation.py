"""GA3-S5 probe: realistic campaign-map vegetation from fal.ai (textures vs image-to-3D).

Two routes are compared on the oak, a grass tuft, a bush and a rock:

* route A (textures): ``fal-ai/flux-2`` image of an oak leaf cluster / grass tuft on white,
  ``fal-ai/bria/background/remove`` for the alpha, then the same normalisation as
  ``game/assets/textures/vegetation/build_leaf_cards.py`` (colour divided by its linear mean,
  times 0.5) so the atlas is a drop-in replacement of ``campaign_leaf_cards.png``;
* route B (3D): ``flux-2`` image of the whole object on grey, background removed,
  ``fal-ai/trellis`` (0.02 $), then decimation to the campaign budgets (~250 and ~90 triangles
  for the oak) and an albedo bake onto a small texture.

Steps (raw files in ``~/dev/cent-ans-raw/ga3/s5/``):

    uv run --with fal-client python ga3_vegetation.py fal            # images, alpha, TRELLIS
    uv run --with pillow --with numpy python ga3_vegetation.py atlas  # candidate atlases
    blender --background --python ga3_vegetation.py -- decimate       # route B LODs + bake
    blender --background --python ga3_vegetation.py -- sheet          # comparison renders

Outputs go to ``game/assets/models/vegetation/ga3/`` (candidates only, nothing is wired).
"""

import json
import math
import sys
import urllib.request
from pathlib import Path

RAW = Path.home() / "dev/cent-ans-raw/ga3/s5"
REPO = Path(__file__).resolve().parents[2]
MAIN_REPO = REPO.parent / "game_project"  # FC5 leaf cards / mid trees live on main
OUT = REPO / "game/assets/models/vegetation/ga3"

WHITE = "isolated on a plain pure white seamless studio background, no shadow, no ground, soft even diffuse lighting, photorealistic botanical scan, sharp focus, high detail"
GREY = "isolated on a plain uniform neutral mid-grey studio background, no ground, no grass, no shadow, soft diffuse overcast lighting, photorealistic, sharp focus"

# name -> (prompt, background, route)
IMAGES = {
    "oak_leaves": (
        "Top-down view of a dense rounded cluster of fresh summer pedunculate oak (Quercus robur) leaves "
        "on thin brown twigs, deeply lobed dark green leaves overlapping, the cluster fills most of the frame, "
        "a few gaps between leaves at the edge, " + WHITE,
        "white",
        "A",
    ),
    "grass_tuft": (
        "Side view of a single tuft of wild meadow grass, thin green and straw-yellow blades fanning out from one base, "
        "a few seed heads, the tuft fills the frame, " + WHITE,
        "white",
        "A",
    ),
    "oak_tree": (
        "Full view of a single mature pedunculate oak tree standing alone, short thick gnarled trunk, "
        "broad irregular domed crown of dense summer foliage with visible leaf clumps and sky gaps, "
        "whole tree visible from trunk base to crown top, eye-level three-quarter view, " + GREY,
        "grey",
        "B",
    ),
    "bush": (
        "A single rounded wild hawthorn bush, dense small green leaves, a few thin woody stems at the base, "
        "whole bush visible, eye-level view, " + GREY,
        "grey",
        "B",
    ),
    "rock": (
        "A single weathered grey limestone boulder with lichen and small moss patches, roughly rounded, "
        "whole rock visible, three-quarter view from slightly above, " + GREY,
        "grey",
        "B",
    ),
}
TRELLIS_ARGS = {"texture_size": 1024, "mesh_simplify": 0.95, "seed": 1337}


# ---------------------------------------------------------------- fal step (plain Python)
def download(url: str, path: Path) -> None:
    """Fetch ``url`` into ``path``."""
    path.parent.mkdir(parents=True, exist_ok=True)
    urllib.request.urlretrieve(url, path)


def fal_step(only: list[str]) -> None:
    """Generate the source images, remove their background and run TRELLIS on route B."""
    import fal_client

    RAW.mkdir(parents=True, exist_ok=True)
    log = {}
    for name, (prompt, _bg, route) in IMAGES.items():
        if only and name not in only:
            continue
        image = fal_client.subscribe(
            "fal-ai/flux-2",
            arguments={"prompt": prompt, "image_size": "square_hd", "num_images": 1, "seed": 1337, "output_format": "png"},
        )
        src_url = image["images"][0]["url"]
        download(src_url, RAW / f"{name}_src.png")
        cut = fal_client.subscribe("fal-ai/bria/background/remove", arguments={"image_url": src_url})
        cut_url = cut["image"]["url"]
        download(cut_url, RAW / f"{name}_cut.png")
        entry = {"prompt": prompt, "src": src_url, "cut": cut_url}
        if route == "B":
            mesh = fal_client.subscribe("fal-ai/trellis", arguments={"image_url": cut_url, **TRELLIS_ARGS})
            glb_url = mesh["model_mesh"]["url"]
            download(glb_url, RAW / f"{name}_trellis.glb")
            entry["glb"] = glb_url
        log[name] = entry
        print("FAL", name, "ok")
    old = json.loads((RAW / "fal_log.json").read_text()) if (RAW / "fal_log.json").exists() else {}
    old.update(log)
    (RAW / "fal_log.json").write_text(json.dumps(old, indent=1))


# ---------------------------------------------------------------- atlas step (Pillow + numpy)
def atlas_step() -> None:
    """Build candidate atlases with the FC5 normalisation (drop-in for ``campaign_leaf_cards.png``)."""
    import numpy as np
    from PIL import Image

    sys.path.insert(0, str(MAIN_REPO / "game/assets/textures/vegetation"))
    from build_leaf_cards import bleed, linear_to_srgb, srgb_to_linear

    def normalized(path: Path, side: int, crop_to_alpha: bool = True) -> np.ndarray:
        image = Image.open(path).convert("RGBA")
        if crop_to_alpha:
            image = image.crop(image.getchannel("A").point(lambda a: 255 if a > 128 else 0).getbbox())
            w, h = image.size
            square = Image.new("RGBA", (max(w, h),) * 2, (0, 0, 0, 0))
            square.paste(image, ((max(w, h) - w) // 2, max(w, h) - h))  # keep tuft bases at the bottom
            image = square
        arr = np.asarray(image.resize((side, side), Image.LANCZOS)).astype(np.float32) / 255.0
        lin = srgb_to_linear(arr[..., :3])
        opaque = arr[..., 3] > 0.5
        mean = lin[opaque].mean(axis=0)
        print(f"{path.name}: mean linear {mean.round(3)}, coverage {opaque.mean():.2f}")
        return np.concatenate([linear_to_srgb(bleed(lin / mean * 0.5, opaque)), arr[..., 3:4]], axis=-1)

    OUT.mkdir(parents=True, exist_ok=True)
    current = np.asarray(Image.open(MAIN_REPO / "game/assets/textures/vegetation/campaign_leaf_cards.png")).astype(np.float32) / 255.0
    leaves = normalized(RAW / "oak_leaves_cut.png", 512)
    atlas = np.concatenate([leaves, current[:, 512:]], axis=1)  # fir half unchanged
    Image.fromarray(np.round(atlas * 255).astype(np.uint8), "RGBA").save(OUT / "ga3_leaf_cards.png")
    tuft = normalized(RAW / "grass_tuft_cut.png", 256)
    # the grass tuft of FC5 is not normalised (raw colours): keep the real colours for that one
    raw = Image.open(RAW / "grass_tuft_cut.png").convert("RGBA")
    raw = raw.crop(raw.getchannel("A").point(lambda a: 255 if a > 128 else 0).getbbox()).resize((256, 256), Image.LANCZOS)
    arr = np.asarray(raw).astype(np.float32) / 255.0
    arr[..., :3] = bleed(arr[..., :3], arr[..., 3] > 0.5)
    Image.fromarray(np.round(arr * 255).astype(np.uint8), "RGBA").save(OUT / "ga3_grass_tuft.png")
    del tuft
    print("OK atlas")


# ---------------------------------------------------------------- Blender steps
def blender_main(argv: list[str]) -> None:
    """Dispatch the Blender sub-steps (imported lazily: ``bpy`` only exists inside Blender)."""
    import ga3_vegetation_blender as vb  # noqa: PLC0415

    {"decimate": vb.decimate_step, "sheet": vb.sheet_step}[argv[0]](argv[1:])


if __name__ == "__main__":
    if "--" in sys.argv:
        sys.path.insert(0, str(Path(__file__).parent))
        blender_main(sys.argv[sys.argv.index("--") + 1 :])
    elif len(sys.argv) > 1 and sys.argv[1] == "fal":
        fal_step(sys.argv[2:])
    elif len(sys.argv) > 1 and sys.argv[1] == "atlas":
        atlas_step()
    else:
        print(__doc__)
