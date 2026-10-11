"""T2: regenerate the DN buildings/props/siege models with TRELLIS.2 run locally (ADR 0342).

Two sub-commands:

``generate`` loads TRELLIS.2 once and turns every source image of the scope into
``~/dev/cent-ans-raw/dn/<id>/3d/trellis2__s42.glb``. It must run with the trellis-mac
interpreter and re-executes itself under ``tools/gpu_lock.sh``. Existing glbs are skipped
(resume), failures go to ``~/dev/cent-ans-raw/t2/failures.jsonl``, no ``.obj`` is kept::

    ~/dev/cent-ans-raw/trellis-mac/.venv/bin/python tools/experiments/t2_batch.py generate [--ids a,b]

``ingest`` runs ``cent-ans dn-ingest`` on each new glb with the class, dimension and yaw of the
current manifest entry, then ``fix_inverted_faces.py`` on the three LODs::

    python3 tools/experiments/t2_batch.py ingest [--ids a,b] [--manifest PATH]
"""

from __future__ import annotations

import argparse
import gc
import json
import os
import shutil
import subprocess
import sys
import time
import traceback
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RAW = Path.home() / "dev/cent-ans-raw"
TRELLIS_MAC = RAW / "trellis-mac"
STATE_DIR = RAW / "t2"
SEED = 42
SCOPE_DIRS = ("buildings", "props", "siege")
KEEP_MODELS = ("fal-ai/trellis-2",)  # already TRELLIS.2 (their source images are gone)


def catalog_classes() -> dict[str, str]:
    """Ingest class of every DN catalog entry, the source of truth over the manifest."""
    classes: dict[str, str] = {}
    for path in sorted((REPO / "data/art").glob("dn_catalog_*.json")):
        document = json.loads(path.read_text())
        entries = (
            document if isinstance(document, list) else document.get("entries", [])
        )
        for entry in entries:
            asset_class = (entry.get("ingest") or {}).get("class")
            if asset_class:
                classes[entry["id"]] = asset_class
    return classes


def with_catalog_class(entry: dict, asset_class: str, classes: dict) -> dict:
    """Entry re-targeted to a class changed in the catalog: budget, texture and output folder."""
    if asset_class == entry["class"]:
        return entry
    category = classes[asset_class]["category"]
    files = [f"dn/{category}/{Path(name).name}" for name in entry["files"]]
    return {
        **entry,
        "class": asset_class,
        "tex": classes[asset_class]["tex"],
        "files": files,
        "stale_files": [name for name in entry["files"] if name not in files],
    }


def scope(ids: set[str]) -> list[dict]:
    """Manifest entries to regenerate, optionally restricted to ``ids``."""
    assets = json.loads((REPO / "data/art/dn_manifest.json").read_text())["assets"]
    classes = json.loads((REPO / "data/art/dn_ingest_classes.json").read_text())[
        "classes"
    ]
    catalog = catalog_classes()
    return [
        with_catalog_class(entry, catalog.get(entry["id"], entry["class"]), classes)
        for entry in assets.values()
        if entry["files"][0].split("/")[1] in SCOPE_DIRS
        and entry.get("model_3d") not in KEEP_MODELS
        and (not ids or entry["id"] in ids)
    ]


def log_event(name: str, record: dict) -> None:
    """Append one JSON line to ``STATE_DIR/<name>.jsonl``."""
    STATE_DIR.mkdir(parents=True, exist_ok=True)
    record = {"ts": time.strftime("%Y-%m-%dT%H:%M:%S"), **record}
    with (STATE_DIR / f"{name}.jsonl").open("a") as handle:
        handle.write(json.dumps(record) + "\n")


def failure_counts() -> dict[str, int]:
    """Number of recorded generation failures per asset id."""
    counts: dict[str, int] = {}
    path = STATE_DIR / "failures.jsonl"
    if path.exists():
        for line in path.read_text().splitlines():
            asset_id = json.loads(line)["id"]
            counts[asset_id] = counts.get(asset_id, 0) + 1
    return counts


def raw_glb(asset_id: str, tag: str = "") -> Path:
    """Where the TRELLIS.2 raw glb of ``asset_id`` is written (``tag``: variant from another image)."""
    suffix = f"__{tag}" if tag else ""
    return RAW / "dn" / asset_id / "3d" / f"trellis2__s{SEED}{suffix}.glb"


def image_overrides(pairs: list[str]) -> dict[str, str]:
    """``ID=PATH`` pairs of ``--image`` as a dict."""
    return dict(pair.split("=", 1) for pair in pairs)


def generate(args: argparse.Namespace) -> None:
    """Generate the missing raw glbs with one loaded pipeline."""
    if os.environ.get("T2_GPU_LOCKED") != "1":
        os.environ["T2_GPU_LOCKED"] = "1"
        lock = str(REPO / "tools/gpu_lock.sh")
        os.execv(lock, [lock, sys.executable, *sys.argv])
    failures = failure_counts()
    overrides = image_overrides(args.image)
    todo = [
        entry
        for entry in scope(set(filter(None, args.ids.split(","))))
        if not raw_glb(entry["id"], args.tag).exists()
        and failures.get(entry["id"], 0) < args.max_attempts
    ]
    print(f"{len(todo)} to generate", flush=True)
    if not todo:
        return
    sys.path.insert(0, str(TRELLIS_MAC))
    import generate as trellis_mac  # noqa: F401  (sets the MPS/attention backends and paths)
    import torch
    from PIL import Image
    from trellis2.pipelines.trellis2_image_to_3d import Trellis2ImageTo3DPipeline

    classes = json.loads((REPO / "data/art/dn_ingest_classes.json").read_text())[
        "classes"
    ]
    started = time.time()
    pipeline = Trellis2ImageTo3DPipeline.from_pretrained("microsoft/TRELLIS.2-4B")
    pipeline.to(torch.device("mps"))
    print(f"pipeline loaded in {time.time() - started:.0f}s", flush=True)
    for index, entry in enumerate(todo, 1):
        free_gb = shutil.disk_usage(RAW).free / 1e9
        if free_gb < args.min_free_gb:
            print(f"stop: {free_gb:.1f} GB free < {args.min_free_gb}", flush=True)
            break
        asset_id = entry["id"]
        target = raw_glb(asset_id, args.tag)
        target.parent.mkdir(parents=True, exist_ok=True)
        partial = target.with_name(target.stem + ".partial")
        started = time.time()
        try:
            image = Image.open(overrides.get(asset_id, entry["source_image"]))
            mesh = pipeline.run(image, seed=SEED, pipeline_type="512")
            mesh = mesh[0] if isinstance(mesh, list) else mesh
            spec = classes[entry["class"]]
            triangles = bake_glb(
                mesh, partial, spec["lod_tris"][0], min(spec["tex"], 2048)
            )
            partial.with_suffix(".glb").rename(target)
            seconds = round(time.time() - started)
            log_event("done", {"id": asset_id, "s": seconds, "triangles": triangles})
            print(
                f"[{index}/{len(todo)}] {asset_id} {seconds}s {triangles} tris {free_gb:.0f} GB free",
                flush=True,
            )
        except Exception as error:  # noqa: BLE001  (one bad asset must not stop the night)
            log_event(
                "failures",
                {
                    "id": asset_id,
                    "error": repr(error)[:500],
                    "trace": traceback.format_exc()[-1500:],
                },
            )
            print(
                f"[{index}/{len(todo)}] {asset_id} FAILED {error!r}"[:300], flush=True
            )
        finally:
            for leftover in target.parent.glob(f"{partial.name}*"):
                leftover.unlink()
            gc.collect()
            torch.mps.empty_cache()
    print(f"batch over: {time.strftime('%H:%M:%S')}", flush=True)


def bake_glb(mesh, partial: Path, faces_target: int, texture_size: int) -> int:
    """Bake the PBR texture of a TRELLIS.2 mesh into ``<partial>.glb`` (Metal, KDTree fallback).

    Same steps as ``trellis-mac/generate.py`` without the ``.obj``, except that TRELLIS.2 itself
    decimates to the class LOD0 budget before unwrapping and baking: its full-resolution mesh
    (~200 k faces, thousands of UV charts) stalls the collapse decimation of ``dn-ingest``,
    whose weld fallback then shreds the model.
    """
    import fast_simplification
    import torch

    vertices = mesh.vertices.cpu().numpy()
    faces = mesh.faces.cpu().numpy()
    if len(vertices) == 0 or len(faces) == 0:
        raise RuntimeError("empty mesh (macOS GPU watchdog)")
    target_faces = min(200_000, len(faces))
    if len(faces) > target_faces:
        vertices, faces = fast_simplification.simplify(
            vertices, faces, 1.0 - target_faces / len(faces)
        )
    glb_path = partial.with_suffix(".glb")
    try:
        import o_voxel.postprocess

        if not getattr(o_voxel.postprocess, "_HAS_DR", False):
            raise RuntimeError("no Metal rasteriser")
        if not getattr(o_voxel.postprocess, "_HAS_FLEX_GEMM", False):
            o_voxel.postprocess._grid_sample_3d = grid_sample_3d_fix
        glb = o_voxel.postprocess.to_glb(
            vertices=torch.from_numpy(vertices).float(),
            faces=torch.from_numpy(faces.astype("int32")),
            attr_volume=mesh.attrs.cpu(),
            coords=mesh.coords.cpu(),
            attr_layout=mesh.layout,
            voxel_size=mesh.voxel_size,
            aabb=[[-0.5, -0.5, -0.5], [0.5, 0.5, 0.5]],
            decimation_target=faces_target,
            texture_size=texture_size,
            verbose=False,
        )
        glb.export(glb_path)
    except (ImportError, AttributeError, RuntimeError) as error:
        print(f"  Metal bake unavailable ({error}), KDTree fallback", flush=True)
        from backends.texture_baker import (
            bake_texture,
            export_glb_with_texture,
            uv_unwrap,
        )

        if len(faces) > faces_target:
            vertices, faces = fast_simplification.simplify(
                vertices, faces, 1.0 - faces_target / len(faces)
            )
        new_vertices, new_faces, uvs, _ = uv_unwrap(vertices, faces)
        base_color, metallic_roughness, _ = bake_texture(
            new_vertices, new_faces, uvs,
            mesh.coords.cpu().float().numpy(), mesh.attrs.cpu().float().numpy(),
            mesh.origin.cpu().float().numpy(), mesh.voxel_size, texture_size=texture_size,
        )  # fmt: skip
        export_glb_with_texture(
            new_vertices, new_faces, uvs, base_color, metallic_roughness, str(glb_path)
        )
    return faces_target


def grid_sample_3d_fix(feats, coords, shape, grid, mode="trilinear"):
    """O_voxel's grid-sample fallback with the [M, C] layout the bake expects (from generate.py)."""
    import torch
    import torch.nn.functional as functional

    batch, channels, depth, height, width = shape[:5]
    dense = torch.zeros(
        batch, channels, depth, height, width, dtype=feats.dtype, device=feats.device
    )
    dense[
        coords[:, 0].long(),
        :,
        coords[:, 1].long(),
        coords[:, 2].long(),
        coords[:, 3].long(),
    ] = feats
    grid_norm = torch.stack(
        [
            grid[..., 2] / (width - 1) * 2 - 1,
            grid[..., 1] / (height - 1) * 2 - 1,
            grid[..., 0] / (depth - 1) * 2 - 1,
        ],
        dim=-1,
    ).reshape(batch, 1, 1, -1, 3)
    sampled = functional.grid_sample(
        dense, grid_norm, mode="bilinear", align_corners=True, padding_mode="border"
    )
    count = grid.shape[1]
    return (
        sampled.reshape(batch, channels, count)
        .permute(0, 2, 1)
        .reshape(batch * count, channels)
    )


def ingest(args: argparse.Namespace) -> None:
    """Ingest the raw glbs with the current manifest parameters, then fix inverted faces."""
    classes = json.loads((REPO / "data/art/dn_ingest_classes.json").read_text())[
        "classes"
    ]
    entries = [
        e
        for e in scope(set(filter(None, args.ids.split(","))))
        if raw_glb(e["id"], args.tag).exists()
    ]
    out_dir = Path(args.out_dir) if args.out_dir else REPO / "game/assets/models"

    def run(entry: dict) -> tuple[str, str]:
        asset_id = entry["id"]
        overrides = image_overrides(args.image)
        axis = classes[entry["class"]]["scale_axis"]
        command = [
            "uv", "run", "--project", str(REPO / "tools"), "cent-ans", "dn-ingest",
            str(raw_glb(asset_id, args.tag)), "--id", asset_id, "--class", entry["class"],
            f"--{axis}", str(entry["dimensions_m"][axis]), "--yaw", str(entry.get("yaw_deg", 0.0) + args.yaw_offset),
            "--lods", str(len(entry["files"])), "--tex", str(entry["tex"]),
            "--source-image", overrides.get(asset_id, entry["source_image"]), "--model-3d", "trellis2-local", "--cost-usd", "0",
            "--manifest", args.manifest,
        ]  # fmt: skip
        if args.out_dir:
            command += ["--out-dir", str(out_dir / "dn")]
        done = subprocess.run(command, capture_output=True, text=True)
        if done.returncode not in (
            0,
            2,
        ):  # 2 = triangle budget flag, asset still written
            log_event(
                "ingest_failures",
                {"id": asset_id, "rc": done.returncode, "err": done.stderr[-800:]},
            )
            return asset_id, f"ingest rc {done.returncode}"
        for stale in entry.get("stale_files", []):
            (out_dir / stale).unlink(missing_ok=True)  # moved to the new class folder
        lods = [str(out_dir / f) for f in entry["files"]]
        fixed = subprocess.run(
            ["uv", "run", "--project", str(REPO / "tools"), "--with", "numpy", "--with", "pillow",
             "--with", "trimesh", "--with", "scipy", "python",
             str(REPO / "tools/experiments/fix_inverted_faces.py"), *lods],
            capture_output=True, text=True,
        )  # fmt: skip
        status = "ok" if fixed.returncode == 0 else f"fix rc {fixed.returncode}"
        log_event(
            "ingested",
            {"id": asset_id, "status": status, "fix": fixed.stdout.strip()[-300:]},
        )
        return asset_id, status

    with ThreadPoolExecutor(args.j) as pool:
        for asset_id, status in pool.map(run, entries):
            print(asset_id, status, flush=True)


def main() -> None:
    """Parse the sub-command."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    gen = sub.add_parser("generate")
    gen.add_argument("--ids", default="")
    gen.add_argument("--min-free-gb", type=float, default=8.0)
    gen.add_argument("--max-attempts", type=int, default=2)
    ing = sub.add_parser("ingest")
    ing.add_argument("--ids", default="")
    ing.add_argument("--manifest", default=str(REPO / "data/art/dn_manifest.json"))
    ing.add_argument("-j", type=int, default=3)
    ing.add_argument(
        "--out-dir", default="", help="models root (default game/assets/models)"
    )
    ing.add_argument(
        "--yaw-offset", type=float, default=0.0, help="added to the manifest yaw"
    )
    for sub_parser in (gen, ing):
        sub_parser.add_argument(
            "--tag", default="", help="raw variant name (other source image)"
        )
        sub_parser.add_argument(
            "--image", action="append", default=[], help="ID=PATH source override"
        )
    args = parser.parse_args()
    {"generate": generate, "ingest": ingest}[args.command](args)


if __name__ == "__main__":
    main()
