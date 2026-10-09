"""Vegetation cards (kind ``texture``) of ``data/art/dn_catalog_nature_cards.json``: local, free.

Z-Image Turbo (mflux, GPU lock shared with ``dn_batch``) x 3 seeds -> rembg -> RGBA card
cropped to its alpha box and fitted in 512 x 512. Raw outputs in ``~/dev/cent-ans-raw/dn/<id>/``;
the best seed (charter colour score of ``dn_batch``) is written to
``game/assets/textures/vegetation/dn_cards/<id>.png`` and listed in ``data/art/dn_cards_manifest.json``.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import dn_batch as batch  # noqa: E402

CATALOG = batch.REPO / "data" / "art" / "dn_catalog_nature_cards.json"
OUT = batch.REPO / "game" / "assets" / "textures" / "vegetation" / "dn_cards"
MANIFEST = batch.REPO / "data" / "art" / "dn_cards_manifest.json"
STYLE = (
    "Photorealistic matte botanical reference photograph of a single {obj}, medieval Europe "
    "or Mediterranean, natural muted desaturated colours, low saturation, albedo like an "
    "overcast day, whole plant fully visible, centred and filling 85 percent of the frame, on a "
    "plain uniform medium grey studio background, no ground, no soil, no cast shadow, soft even "
    "diffuse light, sharp realistic detail, no text, no people."
)
SEEDS = 3
CARD = 512


def main() -> None:
    """Generate, cut, score and export every card."""
    from PIL import Image

    batch.CHARTER_MODE = "warn"
    cards = json.loads(CATALOG.read_text())["cards"]
    only = set(sys.argv[1].split(",")) if len(sys.argv) > 1 else None
    OUT.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(MANIFEST.read_text()) if MANIFEST.exists() else {}
    for card in cards:
        if only and card["id"] not in only:
            continue
        entry = {
            "id": card["id"], "kind": "decor", "seeds": SEEDS, "prompt": card["prompt"],
            "region": "",
        }  # fmt: skip
        out_dir = batch.DN / card["id"]
        out_dir.mkdir(parents=True, exist_ok=True)
        prompt = STYLE.format(obj=card["prompt"].replace(", cut-out RGBA card", "").replace(", cut-out card", ""))
        (out_dir / "prompt.txt").write_text(prompt)
        (out_dir / "img").mkdir(exist_ok=True)
        (out_dir / "cut_raw").mkdir(exist_ok=True)
        scores = {}
        for seed in batch.seed_list(entry):
            target = out_dir / "img" / f"s{seed}.png"
            if not target.exists():
                command = [
                    "mflux-generate-z-image-turbo", "--model", str(batch.MFLUX_MODEL),
                    "--base-model", "z-image-turbo", "--prompt-file", str(out_dir / "prompt.txt"),
                    "--steps", "9", "--seed", str(seed), "--width", "1024", "--height", "1024",
                    "--output", str(target),
                ]  # fmt: skip
                with batch.local_generator_lock(), batch.timed(card["id"], "zimage", seed=seed):
                    batch.subprocess.run(command, check=True, capture_output=True)
            raw = out_dir / "cut_raw" / f"s{seed}.png"
            if not raw.exists():
                batch.rembg_cut(Image.open(target).convert("RGB")).save(raw)
            scores[seed] = batch.score_cut(Image.open(raw))
        best = max(scores, key=lambda s: (not batch.charter_reason(scores[s]), scores[s]["score"]))
        cut = Image.open(out_dir / "cut_raw" / f"s{best}.png")
        box = cut.getchannel("A").point(lambda v: 255 if v > 40 else 0).getbbox()
        crop = cut.crop(box)
        scale = CARD / max(crop.size)
        crop = crop.resize((max(1, round(crop.width * scale)), max(1, round(crop.height * scale))))
        card_image = Image.new("RGBA", (CARD, CARD), (0, 0, 0, 0))
        card_image.paste(crop, ((CARD - crop.width) // 2, CARD - crop.height))  # base at bottom
        card_image.save(OUT / f"{card['id']}.png", optimize=True)
        reason = batch.charter_reason(scores[best])
        manifest[card["id"]] = {
            "file": f"res://assets/textures/vegetation/dn_cards/{card['id']}.png",
            "seed": best, "scores": scores[best], "charter": reason or "ok",
            "aspect": round(crop.width / crop.height, 3),
        }
        (out_dir / "generation.json").write_text(json.dumps({
            "id": card["id"], "catalogue": CATALOG.name, "kind": "texture",
            "catalogue_prompt": card["prompt"], "full_prompt": prompt,
            "seeds": list(scores), "image_backend": "local-mflux", "backend3d": None,
            "views": [], "chosen_seed": best, "charter": reason or "ok",
        }, indent=1, ensure_ascii=False) + "\n")  # fmt: skip
        MANIFEST.write_text(json.dumps(manifest, indent=1, ensure_ascii=False) + "\n")
        print(card["id"], best, reason or "ok", flush=True)


if __name__ == "__main__":
    main()
