"""Ingest a raw TRELLIS/SF3D glb into game-ready LOD glb files (lot DN ingest).

The geometry work runs in Blender (``blender_scripts/dn_ingest.py``); this module reads the
class budgets from ``data/art/dn_ingest_classes.json``, builds the Blender job, checks the
result against the budgets and records it in ``data/art/dn_manifest.json``.
"""

from __future__ import annotations

import json
from pathlib import Path

from cent_ans_tools import blender

REPO = Path(__file__).resolve().parents[2]
CLASSES_PATH = REPO / "data" / "art" / "dn_ingest_classes.json"
MANIFEST_PATH = REPO / "data" / "art" / "dn_manifest.json"
MODELS_DIR = REPO / "game" / "assets" / "models"
OUT_ROOT = MODELS_DIR / "dn"
SCRIPT = blender.SCRIPTS_DIR / "dn_ingest.py"


class IngestError(ValueError):
    """Raised for an unknown class, bad target size or a budget overrun."""


def load_classes(path: Path = CLASSES_PATH) -> dict:
    """Read the class budgets document."""
    return json.loads(path.read_text(encoding="utf-8"))


def build_job(
    *,
    asset_id: str,
    asset_class: str,
    raw: Path,
    classes: dict,
    out_dir: Path | None = None,
    length: float | None = None,
    width: float | None = None,
    height: float | None = None,
    yaw_deg: float = 0.0,
    lods: int = 3,
    tex: int | None = None,
    grade: bool = True,
    gamma: float | None = None,
) -> dict:
    """Validate the request against the class table and return the Blender job document."""
    if asset_class not in classes["classes"]:
        raise IngestError(
            f"classe inconnue {asset_class!r} (connues : {', '.join(classes['classes'])})"
        )
    spec = classes["classes"][asset_class]
    targets = {
        axis: value
        for axis, value in (("length", length), ("width", width), ("height", height))
        if value is not None
    }
    if len(targets) != 1:
        raise IngestError(
            "donner exactement une de --length, --width, --height (mètres)"
        )
    axis, target = next(iter(targets.items()))
    if target <= 0:
        raise IngestError("la taille cible doit être positive")
    if not 1 <= lods <= 3:
        raise IngestError("lods doit valoir 1, 2 ou 3")
    if not asset_id.isidentifier() or asset_id != asset_id.lower():
        raise IngestError("id en snake_case anglais")
    category_dir = (out_dir or OUT_ROOT) / spec["category"]
    return {
        "id": asset_id,
        "raw": str(raw),
        "out_dir": str(category_dir),
        "axis": axis,
        "target_m": target,
        "yaw_deg": yaw_deg,
        "lods": lods,
        "lod_tris": spec["lod_tris"],
        "tex": tex or spec["tex"],
        "grade": grade,
        "gamma": gamma,
        "roughness": spec["roughness"],
        "strip_base": spec["strip_base"],
        "island_min": spec.get("island_min", classes["island_min"]),
        "saturation_cap": classes["grade"]["saturation_cap"],
        "luma_range": [
            classes["grade"]["albedo_mean_min"],
            classes["grade"]["albedo_mean_max"],
        ],
    }


def check_result(job: dict, result: dict, classes: dict) -> list[str]:
    """Return budget problems (empty = fine). A LOD may be under budget, never over."""
    tolerance = classes["lod_ratio_tolerance"]
    problems = []
    for lod, count in enumerate(result["triangles"]):
        cap = job["lod_tris"][lod]
        if count > cap * (1 + tolerance):
            problems.append(f"LOD{lod} : {count} triangles > budget {cap}")
    grade = result.get("grade")
    if grade and grade["after"]["p95_s"] > classes["grade"]["saturation_cap"] + 0.01:
        problems.append(
            f"saturation p95 {grade['after']['p95_s']} au-dessus du plafond"
        )
    return problems


def manifest_entry(
    job: dict,
    result: dict,
    asset_class: str,
    *,
    source_image: str | None = None,
    model_3d: str | None = None,
    cost_usd: float | None = None,
    generation: dict | None = None,
) -> dict:
    """Build the manifest entry for a finished ingest."""
    out_dir = Path(job["out_dir"])
    files = [
        (out_dir / f"{job['id']}_lod{i}.glb").relative_to(MODELS_DIR).as_posix()
        if out_dir.is_relative_to(MODELS_DIR)
        else str(out_dir / f"{job['id']}_lod{i}.glb")
        for i in range(len(result["triangles"]))
    ]
    entry = {
        "id": job["id"],
        "class": asset_class,
        "files": files,
        "dimensions_m": result["dimensions_m"],
        "triangles": result["triangles"],
        "tex": job["tex"],
        "raw": job["raw"],
        "yaw_deg": job["yaw_deg"],
    }
    if result.get("grade"):
        entry["grade"] = result["grade"]
    for key, value in (
        ("source_image", source_image),
        ("model_3d", model_3d),
        ("cost_usd", cost_usd),
        ("generation", generation),
    ):
        if value is not None:
            entry[key] = value
    return entry


def update_manifest(entry: dict, path: Path = MANIFEST_PATH) -> None:
    """Insert or replace ``entry`` in the manifest, keeping keys sorted."""
    document = json.loads(path.read_text(encoding="utf-8"))
    document["assets"][entry["id"]] = entry
    document["assets"] = dict(sorted(document["assets"].items()))
    path.write_text(
        json.dumps(document, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )


def parse_result(output: str) -> dict:
    """Extract the ``DN_RESULT {json}`` line printed by the Blender script."""
    for line in output.splitlines():
        if line.startswith("DN_RESULT "):
            return json.loads(line[len("DN_RESULT ") :])
    raise IngestError("le script Blender n'a pas imprimé DN_RESULT")


def run_ingest(job: dict, classes: dict) -> dict:
    """Run Blender on ``job`` and return the parsed result."""
    output = blender.run_blender_script(SCRIPT, json.dumps(job), timeout=1800)
    return parse_result(output)
