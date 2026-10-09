"""Bake the merged skinned-figure manifest used by ``BattleSkinned`` (lot SC bt6).

The game used to assemble four sources at load time. This tool does it offline and writes
one data file, ``game/assets/models/battle_skinned/manifest_merged.json``:

1. ``battle_skinned/manifest.json`` (coarse Quaternius kit, still read with ``--coarse-figures``);
2. ``battle_fine/manifest.json`` (fine rigs renamed ``fine_*``, fine figures);
3. clip layers on the fine ``human`` rig, in order: ``battle_fine/melee`` (NT14 default melee),
   then the ``default`` clips of ``battle_fine/fa3_anim`` (FA3); each layer's frames are
   appended to the rig texture (``mocap_textures``), clip ``start`` offsets are shifted;
4. ``battle_ga3/manifest.json`` (generated figures: LODs, albedo, head height).

Usage (repository root)::

    uv run --project tools python tools/cent_ans_tools/bake_skinned_manifest.py
"""

from __future__ import annotations

import copy
import json
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MODELS = ROOT / "game" / "assets" / "models"
RES = "res://assets/models/"
FINE_RIG_PREFIX = "fine_"
OUT = MODELS / "battle_skinned" / "manifest_merged.json"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _texture_frames(path: Path) -> int:
    """Frame count of a ``CAB1`` bone texture (header only), 0 if unreadable."""
    if not path.is_file() or path.stat().st_size < 12:
        return 0
    with path.open("rb") as handle:
        header = handle.read(12)
    if header[:4] != b"CAB1":
        return 0
    return struct.unpack("<II", header[4:12])[1]


def _res_to_path(res_path: str, base: Path) -> Path:
    return (
        MODELS / res_path.removeprefix(RES)
        if res_path.startswith("res://")
        else base / res_path
    )


def _merge_fine(base: dict) -> None:
    fine = _load(MODELS / "battle_fine" / "manifest.json")
    rigs = base.setdefault("rigs", {})
    for rig_name, rig in fine.get("rigs", {}).items():
        entry = copy.deepcopy(rig)
        entry["texture"] = f"{RES}battle_fine/{entry.get('texture', '')}"
        rigs[FINE_RIG_PREFIX + rig_name] = entry
    figures = base.setdefault("figures", {})
    for fig_name, fig in fine.get("figures", {}).items():
        entry = copy.deepcopy(fig)
        entry["lods"] = [f"{RES}battle_fine/{f}" for f in entry.get("lods", [])]
        entry["rig"] = FINE_RIG_PREFIX + str(entry.get("rig", ""))
        figures[fig_name] = entry


def _merge_clip_layer(rigs: dict, layer_dir: str, only_default: bool) -> None:
    """Repoint clips of a layer to frames appended after the rig texture (and earlier layers)."""
    directory = MODELS / "battle_fine" / layer_dir
    trial = _load(directory / "manifest.json")
    entry = rigs.get(FINE_RIG_PREFIX + str(trial.get("rig", "human")))
    if entry is None:
        return
    if entry.get("bones", []) != trial.get("bones", []):
        raise SystemExit(f"bones of layer {layer_dir} differ from the rig")
    base_frames = _texture_frames(
        _res_to_path(entry["texture"], MODELS / "battle_skinned")
    )
    if base_frames <= 0:
        raise SystemExit("unreadable rig texture")
    layers = entry.get("mocap_textures", [])
    for layer in layers:
        base_frames += _texture_frames(_res_to_path(layer, MODELS / "battle_skinned"))
    clips = entry.get("clips", {})
    substituted = []
    for clip_name, clip in trial.get("clips", {}).items():
        if clip_name not in clips:
            continue
        clip = dict(clip)
        if only_default and not clip.get("default", False):
            continue
        clip.pop("default", None)
        clip["start"] = base_frames + int(clip["start"])
        clips[clip_name] = clip
        substituted.append(clip_name)
    if not substituted:
        return
    entry["clips"] = clips
    texture = f"{RES}battle_fine/{layer_dir}/{trial.get('texture', '')}"
    entry["mocap_texture"] = texture
    layers.append(texture)
    entry["mocap_textures"] = layers
    entry["mocap_clips"] = substituted
    entry["mocap_trial_dir"] = f"{RES}battle_fine/{layer_dir}/"


def _merge_ga3(figures: dict) -> None:
    generated = _load(MODELS / "battle_ga3" / "manifest.json").get("figures", {})
    for fig_name, gen in generated.items():
        if fig_name not in figures:
            continue
        if not (MODELS / "battle_ga3" / str(gen.get("ga3_albedo", ""))).is_file():
            print(f"warning: GA3 albedo missing for {fig_name}")
            continue
        entry = figures[fig_name]
        entry["lods"] = [f"{RES}battle_ga3/{f}" for f in gen.get("lods", [])]
        entry["tris"] = gen.get("tris", [])
        entry["variants"] = int(gen.get("variants", 1))
        entry["ga3_albedo"] = f"{RES}battle_ga3/{gen['ga3_albedo']}"
        entry["ga3_lum"] = gen.get("ga3_lum", [0.2, 0.2])
        entry["ga3_head_y"] = float(gen.get("head_y", 99.0))
        if not gen.get("fine_horse", False):
            entry.pop("atlas_layer", None)


def bake() -> dict:
    """Return the merged manifest (fine kit, NT14 melee + FA3 default layers, GA3 figures)."""
    merged = _load(MODELS / "battle_skinned" / "manifest.json")
    _merge_fine(merged)
    _merge_clip_layer(merged["rigs"], "melee", only_default=False)
    _merge_clip_layer(merged["rigs"], "fa3_anim", only_default=True)
    _merge_ga3(merged["figures"])
    return merged


def main() -> None:
    """Write the merged manifest."""
    OUT.write_text(
        json.dumps(bake(), indent=1, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
