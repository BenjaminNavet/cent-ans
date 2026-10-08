"""Non-interactive, resumable batch of the image -> 3D procedure (docs/pipeline-assets-3d.md).

Catalogue JSON: a list of ``{"id", "kind": "decor"|"figure", "prompt", "seeds": N,
"backend3d": "fal"|"sf3d"|"hf"|"both"}`` (``both`` = fal + sf3d; ``hf`` = free TRELLIS Space,
best effort, a failure is logged and skipped). Run::

    uv run --with rembg --with onnxruntime --with fal-client --with pillow --with numpy \
        python tools/experiments/dn_batch.py CATALOG.json [--select-best] [--until STAGE] \
        [--only ID,ID] [--fal-workers 4]

Stages, each skipped when its output exists: ``image`` (Z-Image Turbo, mflux) -> ``cut``
(rembg, object framed ~90 % of a 1024 square) -> ``select`` (contact sheet + chosen seed) ->
``3d`` (fal TRELLIS, 0.02 $ per call, in parallel; SF3D local) -> ``sheet`` (Blender, headless)
-> ``gallery`` (rebuild ``~/dev/cent-ans-raw/galerie-3d``). Outputs live outside the repo in
``~/dev/cent-ans-raw/dn/<id>/``.

Charter control (bible 14.2, D5): an image whose cut-out is too saturated/dark never goes to 3D
(status in ``dn/charter.jsonl``, flag on the contact sheet).

Chosen image: ``chosen.json`` if present (write ``{"seed": N}`` to override by eye), else with
``--select-best`` the heuristic winner, else the first seed. ``--until select`` stops after the
contact sheet so the orchestrator can pick by eye.

A global lock file serialises the local generators (mflux, SF3D): two batch processes never
run two of them at once. fal calls are not locked. Every fal call is appended to
``dn/fal_spend.jsonl`` (for ``docs/budget.md``); step durations to ``dn/timings.jsonl``.
No Qwen here (too slow), no windowed Godot.
"""

from __future__ import annotations

import argparse
import contextlib
import fcntl
import json
import os
import subprocess
import threading
import time
import urllib.request
from concurrent.futures import Future, ThreadPoolExecutor
from pathlib import Path

RAW = Path.home() / "dev" / "cent-ans-raw"
DN = RAW / "dn"
GALLERY = RAW / "galerie-3d"
SF3D_DIR = RAW / "sf3d" / "stable-fast-3d"
SF3D_PYTHON = RAW / "sf3d" / ".venv" / "bin" / "python"
LOCK_PATH = DN / "gpu.lock"
REPO = Path(__file__).resolve().parents[2]
DECOR_STYLE = REPO / "data" / "art" / "ga3_decor.json"
MFLUX_MODEL = Path.home() / "models" / "mflux" / "z-image-turbo-q8"
FAL_ENDPOINT = "fal-ai/trellis"
FAL_COST_USD = 0.02
STAGES = ("image", "cut", "select", "3d", "sheet", "gallery")
SIZE = 1024
FILL = 0.9
DEFAULT_REGION = "Rural medieval France circa 1340, humble,"
FIGURE_PREFIX = "Photorealistic reference sheet of a single"
# Bible 14.8 (Qwen figure prompt, text only here).
FIGURE_SUFFIX = (
    "14th-century (1337-1453) European soldier, historically accurate period armour and clothing, "
    "plausible real-world proportions (1.75 m, natural stance, no heroic exaggeration), hand-made "
    "wool, linen, quilted gambeson, boiled leather, riveted mail and hand-forged steel with visible "
    "wear, mud at the boots, scratched and dulled metal, matte finish. Muted earthy palette for all "
    "clothing and gear except the livery: surcoat/jupon in pure saturated green (recolour mask), "
    "hose in pure saturated blue (recolour mask). Arms raised almost horizontal, palms open and "
    "empty, nothing in the hands or in front of the body, mounted figures with hands off the "
    "reins. Full body front view, three-quarter front, feet included, centred, filling 90 percent "
    "of a square frame, plain pure white background, no ground, no shadow, soft even diffuse "
    "light, no text, no heraldic emblem, no modern elements, no fantasy, no weapons."
)
# Bible 14.2 colour check (D5): HSV on the cut-out, livery pixels (pure green/blue) excluded.
MAX_SAT_MEAN, MAX_SAT_P95 = 0.40, 0.90
VALUE_RANGE = (0.25, 0.60)
LIVERY_SAT, LIVERY_VALUE = 0.75, 0.30

_append_lock = threading.Lock()


# --- plumbing -------------------------------------------------------------------------------


@contextlib.contextmanager
def local_generator_lock():
    """Block until this process holds the global lock for local generators (mflux, SF3D)."""
    DN.mkdir(parents=True, exist_ok=True)
    with LOCK_PATH.open("w") as handle:
        started = time.time()
        fcntl.flock(handle, fcntl.LOCK_EX)
        waited = time.time() - started
        if waited > 1:
            print(f"lock acquired after {waited:.0f} s", flush=True)
        try:
            yield
        finally:
            fcntl.flock(handle, fcntl.LOCK_UN)


def append_jsonl(path: Path, record: dict) -> None:
    """Thread-safe single-line append."""
    with _append_lock, path.open("a") as handle:
        handle.write(json.dumps(record, ensure_ascii=False) + "\n")


CATALOG_NAME = ""
SPEND_CAP_USD = float(os.environ.get("DN_FAL_CAP_USD", "29.5"))


def check_cap() -> None:
    """Refuse a fal call when the cumulative spend of ``fal_spend.jsonl`` reached the cap."""
    path = DN / "fal_spend.jsonl"
    if path.exists():
        total = sum(json.loads(line)["usd"] for line in path.read_text().splitlines() if line)
        if total >= SPEND_CAP_USD:
            raise RuntimeError(f"fal cap {SPEND_CAP_USD} $ reached ({total:.2f} $)")


def gen_event(entry_id: str, section: str, record: dict) -> None:
    """Append ``record`` to ``<id>/generation.json[section]`` (provenance, merged not replaced)."""
    path = DN / entry_id / "generation.json"
    record = {"ts": time.strftime("%Y-%m-%dT%H:%M:%S"), **record}
    with _append_lock:
        data = json.loads(path.read_text()) if path.exists() else {"id": entry_id}
        data.pop("reconstructed", None)
        data.setdefault(section, []).append(record)
        path.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")


def gen_base(entry: dict) -> None:
    """Static provenance fields and ``prompt.txt`` (fal path writes them like the local one)."""
    out_dir = DN / entry["id"]
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "prompt.txt").write_text(full_prompt(entry))
    path = out_dir / "generation.json"
    with _append_lock:
        data = json.loads(path.read_text()) if path.exists() else {}
        data.update(
            {
                "id": entry["id"], "catalogue": CATALOG_NAME, "target": entry.get("target"),
                "region": entry.get("region"), "catalogue_prompt": entry["prompt"],
                "full_prompt": full_prompt(entry), "ingest": entry.get("ingest"),
            }
        )  # fmt: skip
        path.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")


@contextlib.contextmanager
def timed(entry_id: str, step: str, **extra):
    """Record the wall-clock time of a step in ``dn/timings.jsonl``."""
    started = time.time()
    try:
        yield
    finally:
        seconds = round(time.time() - started, 1)
        append_jsonl(
            DN / "timings.jsonl", {"id": entry_id, "step": step, "s": seconds, **extra}
        )
        print(f"[{entry_id}] {step} {extra or ''} {seconds} s", flush=True)


def full_prompt(entry: dict) -> str:
    """Decor: charter prefix + object + suffix from ``ga3_decor.json``; figure: white-sheet suffix.

    ``image`` entries (illustrations) carry their whole prompt.
    """
    if entry["kind"] == "image":
        return entry["prompt"]
    if entry["kind"] == "decor":
        style = json.loads(DECOR_STYLE.read_text())
        suffix = style["style_suffix"]
        if entry.get("region"):
            suffix = (
                entry["region"].strip()
                + " "
                + suffix.removeprefix(DEFAULT_REGION).lstrip()
            )
        return f"{style['style_prefix']} {entry['prompt']} {suffix}"
    if entry["prompt"].startswith(
        FIGURE_PREFIX
    ):  # the catalogue holds the whole prompt
        return entry["prompt"]
    return f"{FIGURE_PREFIX} {entry['prompt']} {FIGURE_SUFFIX}"


def backends(entry: dict) -> set[str]:
    """The 3D backends of an entry (``both`` = fal + sf3d)."""
    value = entry.get("backend3d", "both")
    return {"fal", "sf3d"} if value == "both" else set(value.split("+"))


# --- stage image / cut ----------------------------------------------------------------------


def stage_image(entry: dict, out_dir: Path) -> None:
    """Z-Image Turbo, one image per seed, under the local lock."""
    prompt_file = out_dir / "prompt.txt"
    prompt_file.write_text(full_prompt(entry))
    (out_dir / "img").mkdir(exist_ok=True)
    for seed in seed_list(entry):
        target = out_dir / "img" / f"s{seed}.png"
        if target.exists():
            continue
        command = [
            "mflux-generate-z-image-turbo", "--model", str(MFLUX_MODEL),
            "--base-model", "z-image-turbo", "--prompt-file", str(prompt_file),
            "--steps", "9", "--seed", str(seed), "--width", str(SIZE), "--height", str(SIZE),
            "--output", str(target),
        ]  # fmt: skip
        with local_generator_lock(), timed(entry["id"], "zimage", seed=seed):
            subprocess.run(command, check=True, capture_output=True)


FAL_IMAGE_ENDPOINT = "fal-ai/z-image/turbo"
FAL_IMAGE_COST_USD = 0.005


def image_size(entry: dict) -> dict:
    """fal ``image_size``: the ingest size of an ``image`` entry (multiple of 16), else 1024 square."""
    if entry["kind"] == "image":
        width, height = entry["ingest"]["size"]
        return {"width": -(-width // 16) * 16, "height": -(-height // 16) * 16}
    return {"width": SIZE, "height": SIZE}


def fal_image(entry: dict, seed: int, target: Path) -> None:
    """One Z-Image Turbo call on fal (same prompt, seed, 1024 square), PNG at ``target``."""
    import fal_client

    if target.exists():
        return
    try:
        check_cap()
        with timed(entry["id"], "zimage-fal", seed=seed):
            result = fal_client.subscribe(
                FAL_IMAGE_ENDPOINT,
                arguments={
                    "prompt": full_prompt(entry),
                    "image_size": image_size(entry),
                    "seed": seed,
                    "enable_prompt_expansion": False,
                    "output_format": "png",
                    "enable_safety_checker": False,
                    "num_images": 1,
                },
            )
            partial = target.with_suffix(".part")
            urllib.request.urlretrieve(result["images"][0]["url"], partial)  # noqa: S310
            partial.rename(target)
    except Exception as error:  # noqa: BLE001
        append_jsonl(
            DN / "failures.jsonl",
            {"id": entry["id"], "step": "zimage-fal", "error": str(error)[:400]},
        )
        return
    gen_event(
        entry["id"], "images",
        {
            "seed": seed, "endpoint": FAL_IMAGE_ENDPOINT, "file": f"img/s{seed}.png",
            "arguments": {
                "image_size": image_size(entry), "seed": seed, "enable_prompt_expansion": False,
                "output_format": "png",
            },
            "usd": FAL_IMAGE_COST_USD,
        },
    )  # fmt: skip
    append_jsonl(
        DN / "fal_spend.jsonl",
        {
            "ts": time.strftime("%Y-%m-%dT%H:%M:%S"), "id": entry["id"], "category": "image",
            "endpoint": FAL_IMAGE_ENDPOINT, "seed": seed, "usd": FAL_IMAGE_COST_USD,
        },
    )  # fmt: skip


def image_output_path(entry: dict) -> Path:
    """Repo path of an ``image`` entry (``ingest.out`` is ``res://...`` relative to ``game/``)."""
    out = entry["ingest"]["out"]
    return REPO / "game" / out.removeprefix("res://")


def finalise_image(entry: dict, source: Path) -> Path:
    """Fit ``source`` to ``ingest.size`` (cover + centre crop), write it as PNG or JPG per extension."""
    from PIL import Image

    width, height = entry["ingest"]["size"]
    image = Image.open(source).convert("RGB")
    scale = max(width / image.width, height / image.height)
    resized = image.resize(
        (max(width, round(image.width * scale)), max(height, round(image.height * scale))),
        Image.LANCZOS,
    )
    left, top = (resized.width - width) // 2, (resized.height - height) // 2
    fitted = resized.crop((left, top, left + width, top + height))
    target = image_output_path(entry)
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.suffix.lower() in (".jpg", ".jpeg"):
        fitted.save(target, quality=90)
    else:
        fitted.save(target)
    return target


def run_image_entries(entries: list[dict], workers: int) -> None:
    """``image`` entries: one fal image at the ingest size, no cut-out, no 3D."""
    jobs = []
    with ThreadPoolExecutor(max_workers=workers) as pool:
        for entry in entries:
            out_dir = DN / entry["id"]
            (out_dir / "img").mkdir(parents=True, exist_ok=True)
            gen_base(entry)
            seed = seed_list(entry)[0]
            target = out_dir / "img" / f"s{seed}.png"
            jobs.append((entry, target, pool.submit(fal_image, entry, seed, target)))
        for entry, target, job in jobs:
            job.result()
            if target.exists():
                finalise_image(entry, target)
            else:
                print(f"[{entry['id']}] image missing (cap or failure)", flush=True)


def prefetch_images(entries: list[dict], workers: int) -> None:
    """Generate every missing image on fal in parallel, without the GPU lock."""
    jobs = []
    with ThreadPoolExecutor(max_workers=workers) as pool:
        for entry in entries:
            out_dir = DN / entry["id"]
            (out_dir / "img").mkdir(parents=True, exist_ok=True)
            for seed in seed_list(entry):
                target = out_dir / "img" / f"s{seed}.png"
                if not target.exists():
                    jobs.append(pool.submit(fal_image, entry, seed, target))
        for job in jobs:
            job.result()


def seed_list(entry: dict) -> list[int]:
    """Deterministic seeds 1337, 1338, ... (``seeds`` of the catalogue = how many)."""
    return [1337 + index for index in range(int(entry.get("seeds", 1)))]


_rembg_session = None


def rembg_cut(image):
    """RGBA cut-out of ``image`` (rembg isnet, session shared by the process)."""
    global _rembg_session
    from rembg import new_session, remove

    if _rembg_session is None:
        _rembg_session = new_session("isnet-general-use")
    return remove(image, session=_rembg_session).convert("RGBA")


def frame_square(cut):
    """Crop to the alpha box and centre it in a 1024 square, longest side = 90 %."""
    from PIL import Image

    box = cut.getchannel("A").point(lambda v: 255 if v > 40 else 0).getbbox()
    if box is None:
        return cut.resize((SIZE, SIZE))
    crop = cut.crop(box)
    scale = SIZE * FILL / max(crop.size)
    crop = crop.resize(
        (max(1, round(crop.width * scale)), max(1, round(crop.height * scale)))
    )
    canvas = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    canvas.paste(crop, ((SIZE - crop.width) // 2, (SIZE - crop.height) // 2))
    return canvas


def stage_cut(entry: dict, out_dir: Path) -> None:
    """Rembg for every image: ``cut_raw/`` (as cut) and ``cut/`` (framed for the 3D models)."""
    from PIL import Image

    (out_dir / "cut_raw").mkdir(exist_ok=True)
    (out_dir / "cut").mkdir(exist_ok=True)
    for seed in seed_list(entry):
        raw, framed = (
            out_dir / "cut_raw" / f"s{seed}.png",
            out_dir / "cut" / f"s{seed}.png",
        )
        if framed.exists() or not (out_dir / "img" / f"s{seed}.png").exists():
            continue
        with timed(entry["id"], "rembg", seed=seed):
            cut = rembg_cut(Image.open(out_dir / "img" / f"s{seed}.png").convert("RGB"))
            cut.save(raw)
            frame_square(cut).save(framed)


# --- stage select ---------------------------------------------------------------------------


def charter_check(raw_cut) -> dict:
    """Bible 14.2 colour control on the cut-out: S mean <= 0.40, S p95 <= 0.90, V mean 0.25-0.60.

    Pure saturated green / blue pixels (livery recolour channels) are left out.
    """
    import numpy as np

    hsv = np.array(raw_cut.convert("RGB").convert("HSV"), dtype=float) / 255.0
    opaque = np.array(raw_cut.getchannel("A")) > 40
    hue, sat, val = hsv[..., 0] * 360, hsv[..., 1], hsv[..., 2]
    pure = (
        (sat >= LIVERY_SAT)
        & (val >= LIVERY_VALUE)
        & (((hue >= 90) & (hue <= 160)) | ((hue >= 200) & (hue <= 260)))
    )
    kept = opaque & ~pure
    if not kept.any():
        return {"empty": True}
    s_mean, s_p95 = float(sat[kept].mean()), float(np.percentile(sat[kept], 95))
    v_mean = float(val[kept].mean())
    return {
        "s_mean": round(s_mean, 3),
        "s_p95": round(s_p95, 3), "v_mean": round(v_mean, 3),
        "livery_px": int((opaque & pure).sum()),
    }  # fmt: skip


CHARTER_MODE = "strict"  # strict = block (bible 14.2) | warn = flag only | off


def charter_reason(info: dict) -> str:
    """Why a scored image is out of charter ('' = fine); thresholds from the bible, 14.2."""
    if info.get("empty"):
        return "empty"
    reasons = []
    if info["s_mean"] > MAX_SAT_MEAN:
        reasons.append("S mean")
    if info["s_p95"] > MAX_SAT_P95:
        reasons.append("S p95")
    if not VALUE_RANGE[0] <= info["v_mean"] <= VALUE_RANGE[1]:
        reasons.append("V mean")
    return ",".join(reasons)


def charter_ok(info: dict) -> bool:
    """Whether an image may go to 3D under the current ``CHARTER_MODE``."""
    return CHARTER_MODE != "strict" or not charter_reason(info)


def score_cut(raw_cut) -> dict:
    """Heuristic quality of a raw cut-out: centred, whole (not cut by the border), well filled.

    Score in [0, 1]: 0 if the object touches the border; else the mean of centring (alpha
    centroid near the middle), fill (alpha box covers 35-85 % of the frame, peak at 60 %) and
    solidity (alpha area over box area, a sparse mask means a ragged cut).
    """
    import numpy as np

    alpha = np.array(raw_cut.getchannel("A")) > 40
    height, width = alpha.shape
    if not alpha.any():
        return {"score": 0.0, "touches_border": True}
    rows, cols = np.where(alpha.any(axis=1))[0], np.where(alpha.any(axis=0))[0]
    margin = round(0.01 * max(height, width))
    touches = bool(
        rows[0] <= margin or cols[0] <= margin
        or rows[-1] >= height - 1 - margin or cols[-1] >= width - 1 - margin
    )  # fmt: skip
    ys, xs = np.nonzero(alpha)
    offset = max(abs(xs.mean() / width - 0.5), abs(ys.mean() / height - 0.5))
    centring = max(0.0, 1 - 4 * offset)
    box_fraction = ((rows[-1] - rows[0] + 1) * (cols[-1] - cols[0] + 1)) / (
        height * width
    )
    fill = max(0.0, 1 - abs(box_fraction - 0.6) / 0.4)
    solidity = float(alpha.sum()) / (
        (rows[-1] - rows[0] + 1) * (cols[-1] - cols[0] + 1)
    )
    score = 0.0 if touches else (centring + fill + min(1.0, solidity / 0.6)) / 3
    return {
        **charter_check(raw_cut),
        "score": round(score, 3), "touches_border": touches, "centring": round(centring, 3),
        "fill": round(fill, 3), "solidity": round(solidity, 3),
    }  # fmt: skip


def stage_select(entry: dict, out_dir: Path, select_best: bool) -> int | None:
    """Contact sheet + scores; returns the chosen seed (chosen.json > best > first)."""
    from PIL import Image, ImageDraw

    seeds = [s for s in seed_list(entry) if (out_dir / "cut_raw" / f"s{s}.png").exists()]
    scores_path = out_dir / "scores.json"
    if scores_path.exists():
        scores = json.loads(scores_path.read_text())
    else:
        scores = {
            str(seed): score_cut(Image.open(out_dir / "cut_raw" / f"s{seed}.png"))
            for seed in seeds
        }
        scores_path.write_text(json.dumps(scores, indent=1))
    contact = out_dir / "contact.png"
    if not contact.exists():
        tile = 384
        sheet = Image.new("RGB", (tile * len(seeds), tile + 24), (110, 110, 112))
        draw = ImageDraw.Draw(sheet)
        for index, seed in enumerate(seeds):
            picture = (
                Image.open(out_dir / "img" / f"s{seed}.png")
                .convert("RGB")
                .resize((tile, tile))
            )
            sheet.paste(picture, (index * tile, 24))
            info = scores[str(seed)]
            flag = (" BORD" if info["touches_border"] else "") + (
                "" if not (r := charter_reason(info)) else f" HORS-CHARTE({r})"
            )
            draw.text(
                (index * tile + 6, 6),
                f"{entry['id']} s{seed} score {info['score']}{flag}",
                fill=(255, 255, 255),
            )
        sheet.save(contact)
    chosen_path = out_dir / "chosen.json"
    allowed = [x for x in seeds if charter_ok(scores[str(x)])]
    if chosen_path.exists():
        seed = int(json.loads(chosen_path.read_text())["seed"])
        if seed not in allowed:
            allowed = []
    elif allowed:
        seed = (
            max(allowed, key=lambda x: scores[str(x)]["score"])
            if select_best
            else allowed[0]
        )
        chosen_path.write_text(
            json.dumps({"seed": seed, "by": "heuristic" if select_best else "first"})
        )
    if not allowed:
        append_jsonl(
            DN / "charter.jsonl",
            {"id": entry["id"], "status": "rejected", "scores": scores},
        )
        print(f"[{entry['id']}] all images out of charter: no 3D", flush=True)
        return None
    append_jsonl(
        DN / "charter.jsonl", {"id": entry["id"], "status": "ok", "seed": seed}
    )
    return seed


def global_contact(entries: list[dict]) -> Path:
    """One PNG stacking every per-object contact sheet, for the orchestrator's eye."""
    from PIL import Image

    sheets = [
        Image.open(DN / e["id"] / "contact.png")
        for e in entries
        if (DN / e["id"] / "contact.png").exists()
    ]
    target = DN / "_contact.png"
    if sheets:
        width = max(s.width for s in sheets)
        board = Image.new("RGB", (width, sum(s.height for s in sheets)), (60, 60, 60))
        y = 0
        for sheet in sheets:
            board.paste(sheet, (0, y))
            y += sheet.height
        board.save(target)
    return target


# --- stage 3d -------------------------------------------------------------------------------


def fal_trellis(entry_id: str, cut: Path, target: Path, seed: int) -> None:
    """One ``fal-ai/trellis`` call (0.02 $), glb saved to ``target``, spend logged."""
    import fal_client

    check_cap()
    with timed(entry_id, "trellis-fal", seed=seed):
        url = fal_client.upload_file(str(cut))
        result = fal_client.subscribe(
            FAL_ENDPOINT,
            arguments={
                "image_url": url,
                "seed": seed,
                "texture_size": 1024,
                "mesh_simplify": 0.95,
            },
        )
        urllib.request.urlretrieve(result["model_mesh"]["url"], target)  # noqa: S310
    gen_event(
        entry_id, "calls_3d",
        {
            "seed": seed, "endpoint": FAL_ENDPOINT, "mode": "single-view",
            "views": [str(cut.relative_to(DN / entry_id))], "file": f"3d/{target.name}",
            "usd": FAL_COST_USD,
        },
    )  # fmt: skip
    append_jsonl(
        DN / "fal_spend.jsonl",
        {
            "ts": time.strftime("%Y-%m-%dT%H:%M:%S"), "id": entry_id,
            "endpoint": FAL_ENDPOINT, "seed": seed, "usd": FAL_COST_USD,
        },
    )  # fmt: skip


def sf3d_run(entry_id: str, cut: Path, target: Path, seed: int) -> None:
    """SF3D local (MPS) under the lock; the glb is copied to ``target``."""
    work = target.parent / f"sf3d_out_s{seed}"
    with local_generator_lock(), timed(entry_id, "sf3d", seed=seed):
        subprocess.run(
            [str(SF3D_PYTHON), "run.py", str(cut), "--output-dir", str(work)],
            cwd=SF3D_DIR, check=True, capture_output=True,
        )  # fmt: skip
    target.write_bytes((work / "0" / "mesh.glb").read_bytes())


def hf_trellis(entry_id: str, cut: Path, target: Path, seed: int) -> None:
    """Free TRELLIS Space (ZeroGPU quota); failure is logged, never fatal."""
    command = [
        "uv", "run", "--with", "gradio_client", "python",
        str(REPO / "tools/experiments/trellis_hf.py"), str(target.parent), f"hf_{target.stem}",
        str(cut), "--seeds", str(seed),
    ]  # fmt: skip
    with timed(entry_id, "trellis-hf", seed=seed):
        done = subprocess.run(command, capture_output=True, text=True)
    produced = target.parent / f"hf_{target.stem}__s{seed}.glb"
    if done.returncode == 0 and produced.exists():
        produced.rename(target)
    else:
        note = (done.stderr or done.stdout)[-300:]
        append_jsonl(
            DN / "failures.jsonl", {"id": entry_id, "step": "hf", "error": note}
        )
        print(f"[{entry_id}] HF failed (skipped): {note.strip()[-120:]}", flush=True)


MULTI_VIEW_CLASSES = {
    "house", "major_building", "bridge", "ship", "cart", "siege_engine", "figure",
    "figure_mounted",
}  # fmt: skip
SIDE_VIEW_CLASSES = {"cart", "siege_engine", "bridge"}
VIEW_PROMPTS = {
    "back": "Show exactly the same object from behind, seen from the back three-quarter "
    "opposite to the original view, same materials, same proportions, same lighting, "
    "same plain mid-grey background, whole object visible and centred, no ground.",
    "side": "Show exactly the same object from its left side, seen straight from the side at "
    "eye level, same materials, same proportions, same lighting, same plain mid-grey "
    "background, whole object visible and centred, no ground.",
}
FAL_EDIT_ENDPOINT = "fal-ai/flux-2/edit"
FAL_EDIT_COST_USD = 0.024
FAL_MULTI_ENDPOINT = "fal-ai/trellis/multi"
_rembg_lock = threading.Lock()


def is_multi_view(entry: dict) -> bool:
    """Oriented objects need several views (single-view TRELLIS leaves a black back)."""
    return entry.get("ingest", {}).get("class") in MULTI_VIEW_CLASSES


def log_spend(entry_id: str, endpoint: str, seed: int, usd: float, category: str) -> None:
    """One line of ``fal_spend.jsonl``."""
    append_jsonl(
        DN / "fal_spend.jsonl",
        {
            "ts": time.strftime("%Y-%m-%dT%H:%M:%S"), "id": entry_id, "category": category,
            "endpoint": endpoint, "seed": seed, "usd": usd,
        },
    )  # fmt: skip


def fal_multi(entry: dict, out_dir: Path, seed: int, target: Path) -> None:
    """Back (and side) views by flux-2/edit, rembg, then ``fal-ai/trellis/multi``."""
    import fal_client
    from PIL import Image

    check_cap()
    entry_id = entry["id"]
    views = out_dir / "views"
    views.mkdir(exist_ok=True)
    names = ["back"] + (["side"] if entry["ingest"]["class"] in SIDE_VIEW_CLASSES else [])
    cuts = [out_dir / "cut" / f"s{seed}.png"]
    source_url = None
    for name in names:
        raw, framed = views / f"{name}_s{seed}.png", views / f"{name}_cut_s{seed}.png"
        if not raw.exists():
            source_url = source_url or fal_client.upload_file(
                str(out_dir / "img" / f"s{seed}.png")
            )
            with timed(entry_id, f"edit-{name}", seed=seed):
                result = fal_client.subscribe(
                    FAL_EDIT_ENDPOINT,
                    arguments={
                        "prompt": VIEW_PROMPTS[name], "image_urls": [source_url],
                        "image_size": {"width": SIZE, "height": SIZE}, "seed": seed,
                        "output_format": "png",
                    },
                )  # fmt: skip
                urllib.request.urlretrieve(result["images"][0]["url"], raw)  # noqa: S310
            log_spend(entry_id, FAL_EDIT_ENDPOINT, seed, FAL_EDIT_COST_USD, "view")
            gen_event(
                entry_id, "views",
                {
                    "seed": seed, "view": name, "endpoint": FAL_EDIT_ENDPOINT,
                    "prompt": VIEW_PROMPTS[name], "source": f"img/s{seed}.png",
                    "file": f"views/{name}_s{seed}.png", "usd": FAL_EDIT_COST_USD,
                },
            )  # fmt: skip
        if not framed.exists():
            with _rembg_lock:
                frame_square(rembg_cut(Image.open(raw).convert("RGB"))).save(framed)
        cuts.append(framed)
    with timed(entry_id, "trellis-multi", seed=seed):
        urls = [fal_client.upload_file(str(path)) for path in cuts]
        result = fal_client.subscribe(
            FAL_MULTI_ENDPOINT,
            arguments={
                "image_urls": urls, "multiimage_algo": "stochastic", "seed": seed,
                "texture_size": 1024, "mesh_simplify": 0.95,
            },
        )  # fmt: skip
        mesh = result.get("model_mesh") or result.get("model_glb")
        urllib.request.urlretrieve(mesh["url"], target)  # noqa: S310
    log_spend(entry_id, FAL_MULTI_ENDPOINT, seed, FAL_COST_USD, "3d")
    gen_event(
        entry_id, "calls_3d",
        {
            "seed": seed, "endpoint": FAL_MULTI_ENDPOINT, "mode": "multi-view",
            "views": [str(path.relative_to(out_dir)) for path in cuts],
            "file": f"3d/fal__s{seed}.glb", "usd": FAL_COST_USD,
        },
    )  # fmt: skip


def fal_job(
    entry_id: str, cut: Path, target: Path, seed: int, entry: dict | None = None
) -> None:
    """Wrapper that logs a fal failure (balance, network) instead of killing the batch."""
    try:
        if entry is not None and is_multi_view(entry):
            fal_multi(entry, cut.parent.parent, seed, target)
        else:
            fal_trellis(entry_id, cut, target, seed)
    except Exception as error:  # noqa: BLE001
        append_jsonl(
            DN / "failures.jsonl",
            {"id": entry_id, "step": "fal", "error": str(error)[:400]},
        )
        print(f"[{entry_id}] fal failed: {str(error)[:200]}", flush=True)


# --- stage sheet / gallery ------------------------------------------------------------------


def stage_sheet(entry: dict, out_dir: Path) -> None:
    """Headless Blender board (fal | sf3d | hf glb side by side, same light)."""
    target = out_dir / "sheet.png"
    glbs = sorted((out_dir / "3d").glob("*.glb"))
    if target.exists() or not glbs:
        return
    command = [
        "blender", "-b", "--factory-startup", "-P",
        str(REPO / "tools/blender_scripts/ga3_compare_render.py"), "--",
        "--out", str(target), "--stats", str(out_dir / "sheet_stats.json"), "--fit-width", "1.8",
    ]  # fmt: skip
    for glb in glbs:
        label = glb.stem.split("__")[0]
        command += [
            "--panel",
            str(glb),
            f"{entry['id']} {label}",
            "--yaw",
            "180" if label == "sf3d" else "0",
        ]
    with timed(entry["id"], "sheet"):
        done = subprocess.run(command, capture_output=True, text=True)
    if done.returncode != 0:
        append_jsonl(
            DN / "failures.jsonl",
            {"id": entry["id"], "step": "sheet", "error": done.stderr[-300:]},
        )


def stage_gallery() -> None:
    """Rebuild the gallery (its ``build.py`` links ``dn/<id>/3d/*.glb`` by convention)."""
    subprocess.run(
        ["python3", "-I", "build.py"], cwd=GALLERY, check=True, capture_output=True
    )


# --- main -----------------------------------------------------------------------------------


def process(
    entry: dict, args, executor: ThreadPoolExecutor, futures: list[Future]
) -> None:
    """Run the stages of one entry; fal jobs go to the executor, the rest runs here."""
    out_dir = DN / entry["id"]
    out_dir.mkdir(parents=True, exist_ok=True)
    if entry["kind"] == "figure":
        (out_dir / "kind_figure").touch()
    until = STAGES.index(args.until)
    if args.image_backend == "fal":
        gen_base(entry)
        if not any((out_dir / "img").glob("s*.png")):
            print(f"[{entry['id']}] no fal image, skipped", flush=True)
            return
    else:
        stage_image(entry, out_dir)
    if until < 1:
        return
    stage_cut(entry, out_dir)
    if until < 2:
        return
    seed = stage_select(entry, out_dir, args.select_best)
    if until < 3 or seed is None:
        return
    cut = out_dir / "cut" / f"s{seed}.png"
    (out_dir / "3d").mkdir(exist_ok=True)
    wanted = backends(entry)
    if "fal" in wanted and not (out_dir / "3d" / f"fal__s{seed}.glb").exists():
        futures.append(
            executor.submit(
                fal_job, entry["id"], cut, out_dir / "3d" / f"fal__s{seed}.glb", seed, entry
            )
        )
    if "hf" in wanted and not (out_dir / "3d" / f"hf__s{seed}.glb").exists():
        hf_trellis(entry["id"], cut, out_dir / "3d" / f"hf__s{seed}.glb", seed)
    if "sf3d" in wanted and not (out_dir / "3d" / f"sf3d__s{seed}.glb").exists():
        try:
            sf3d_run(entry["id"], cut, out_dir / "3d" / f"sf3d__s{seed}.glb", seed)
        except Exception as error:  # noqa: BLE001
            append_jsonl(
                DN / "failures.jsonl",
                {"id": entry["id"], "step": "sf3d", "error": str(error)[:300]},
            )


def main() -> None:
    """CLI entry point."""
    global CHARTER_MODE, MAX_SAT_P95, MAX_SAT_MEAN
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("catalog")
    parser.add_argument(
        "--select-best", action="store_true", help="heuristic image choice"
    )
    parser.add_argument("--until", choices=STAGES, default="gallery")
    parser.add_argument("--only", default="", help="comma-separated ids")
    parser.add_argument(
        "--charter",
        choices=("strict", "warn", "off"),
        default="strict",
        help="bible 14.2 colour control: block out-of-charter images, only flag them, or skip",
    )
    parser.add_argument("--charter-s-p95", type=float, default=MAX_SAT_P95)
    parser.add_argument("--charter-s-mean", type=float, default=MAX_SAT_MEAN)
    parser.add_argument("--fal-workers", type=int, default=4)
    parser.add_argument("--backend3d", default="", help="override every entry (fal|sf3d|hf|both)")
    parser.add_argument("--seeds", type=int, default=0, help="override seeds per entry")
    parser.add_argument("--image-backend", choices=("local", "fal"), default="local")
    parser.add_argument("--image-workers", type=int, default=6)
    parser.add_argument("--kind", default="", help="keep only this kind (decor|figure)")
    args = parser.parse_args()
    CHARTER_MODE, MAX_SAT_P95, MAX_SAT_MEAN = (
        args.charter,
        args.charter_s_p95,
        args.charter_s_mean,
    )
    global CATALOG_NAME
    CATALOG_NAME = Path(args.catalog).name
    entries = json.loads(Path(args.catalog).read_text())
    if args.only:
        entries = [e for e in entries if e["id"] in args.only.split(",")]
    if args.kind:
        entries = [e for e in entries if e["kind"] == args.kind]
    for entry in entries:
        if args.backend3d:
            entry["backend3d"] = args.backend3d
        if args.seeds:
            entry["seeds"] = args.seeds
    if "FAL_KEY" not in os.environ and any("fal" in backends(e) for e in entries):
        print(
            "warning: FAL_KEY not set, fal calls will fail (logged in failures.jsonl)"
        )
    DN.mkdir(parents=True, exist_ok=True)
    image_entries = [e for e in entries if e["kind"] == "image"]
    if image_entries:
        entries = [e for e in entries if e["kind"] != "image"]
        run_image_entries(image_entries, args.image_workers)
        if not entries:
            return
    if args.image_backend == "fal":
        prefetch_images(entries, args.image_workers)
    futures: list[Future] = []
    with ThreadPoolExecutor(max_workers=args.fal_workers) as executor:
        for entry in entries:
            process(entry, args, executor, futures)
        global_contact(entries)
        for future in futures:
            future.result()
    until = STAGES.index(args.until)
    if until >= 4:
        for entry in entries:
            stage_sheet(entry, DN / entry["id"])
    if until >= 5:
        stage_gallery()
    print(f"contact sheet: {DN / '_contact.png'}")


if __name__ == "__main__":
    main()
