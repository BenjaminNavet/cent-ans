"""MX: drive the Meshy subscription waves (image -> 3D) within the 10-task queue limit.

``python3 tools/experiments/meshy_waves.py run [--wave N ...] [--min-balance 20]``
``python3 tools/experiments/meshy_waves.py status``

Each object is one task: figurines go to ``multi-image-to-3d`` (front/side/back, meshy-7.1,
a-pose), decor to ``image-to-3d`` (front view only, meshy-6-lite). The task ledger
``RAW/tasks/ledger.json`` makes the run resumable: a submitted object is never resubmitted, a
finished one is downloaded to ``RAW/<id>/meshy.glb``. The API key is read from ``MESHY_API_KEY``
or from the ``meshy`` MCP server entry of ``~/.claude.json``; it never touches the repository.
"""

from __future__ import annotations

import argparse
import base64
import json
import os
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RAW = Path.home() / "dev/cent-ans-raw/meshy"
GA3 = Path.home() / "dev/cent-ans-raw/ga3"
DN = Path.home() / "dev/cent-ans-raw/dn"
LEDGER = RAW / "tasks/ledger.json"
API = "https://api.meshy.ai/openapi"
QUEUE_MAX = 10
COST = {"figure": 30, "decor": 15}

FIGURES_L3 = {
    "crossbowman",
    "knight",
    "longbowman",
    "man_at_arms",
    "militia",
    "sergeant",
}
WAVES = {
    0: "castle_byz castle_west city_west siege_tower",  # bench models, already paid
    1: "coutilier crossbowman gascon_crossbowman gendarme knight longbowman man_at_arms militia "
    "plain_crossbowman retinue routier schiltron sergeant standard_bearer urban_militia welsh_spearman",
    2: "castle_balt castle_byz_serbia castle_empire_rhine castle_hungpol castle_iber castle_isl "
    "castle_isl_anatolia castle_isl_andalus castle_isl_mamluk castle_ital_rocca castle_med castle_nord "
    "castle_rus castle_steppe",
    3: "city_balt city_byz city_hansa city_hungpol city_iber_mudejar city_isl city_isl_andalus "
    "city_isl_mamluk city_ital_commune city_ital_maritime city_med city_nord city_rus city_steppe",
    4: "siege_tower_belfry_hides siege_ram_roofed siege_ballista_crossbow_great siege_springald_bolt "
    "siege_bombard_great_1450 siege_bombard_hooped_1380 siege_cannon_early_1340 "
    "siege_gun_bronze_wheeled_1450 siege_ribauldequin_organ siege_gun_battery_mantlet "
    "siege_powder_barrels_cart wagon_ammunition_bombard siege_handgonne_pair",
    5: "abbey_balt abbey_byz abbey_byz_serbia abbey_iber abbey_isl_anatolia abbey_isl_maghreb abbey_ital "
    "abbey_nord abbey_rus abbey_steppe abbey_ural_shrine abbey_west church_stave church_wood_polish "
    "fort_fortified_church",
    6: "fort_barbican_gatehouse fort_castle_keep_square fort_shell_keep_motte fort_palisade_corner_tower "
    "fort_portcullis_gate fort_drawbridge_lowered ruin_castle fort_stone_curtain_ruined",
    7: "bridge_gate bridge_stone bridge_stone_arch bridge_wood econ_town_gate_brick_hanse "
    "econ_town_gate_horseshoe_bab econ_town_gate_mediterranean econ_town_gate_timber_rus "
    "econ_town_gate_twin_towers round_tower watchtower",
    # Campaign map trial (13/10): 5 trees (one per climate), 3 rocks, 4 crops as a whole field
    # and as a close-up of the crop, to compare both on the campaign map.
    8: "tree_oak_pedunculate tree_spruce tree_birch tree_pine_stone tree_juniper "
    "rock_granite_tor_moor rock_cliff_face_limestone rock_sandstone_outcrop "
    "field_wheat_ripe crop_wheat_ripe field_barley_green crop_barley_ripe "
    "field_vineyard_rows crop_vine_stakes_row field_olive_grove tree_olive",
    # Campaign towns (13/10): every `town` family of dn_campaign_models.json, still TRELLIS 1.
    9: "town_west town_med town_isl town_byz town_rus town_steppe town_hansa town_empire_south "
    "town_hungpol town_nord town_balt town_ital_towers town_iber town_byz_serbia town_ural "
    "town_isl_andalus town_isl_mamluk town_isl_anatolia town_circassian town_volga_bulgar "
    "town_port_harbour",
}


def api_key() -> str:
    """The Meshy key from the environment or the MCP server config."""
    if key := os.environ.get("MESHY_API_KEY"):
        return key
    config = json.loads((Path.home() / ".claude.json").read_text())
    return config["mcpServers"]["meshy"]["env"]["MESHY_API_KEY"]


def request(method: str, path: str, body: dict | None = None) -> dict:
    """One authenticated JSON call to the Meshy API."""
    req = urllib.request.Request(
        f"{API}{path}",
        method=method,
        data=json.dumps(body).encode() if body is not None else None,
        headers={
            "Authorization": f"Bearer {api_key()}",
            "Content-Type": "application/json",
        },
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        return json.loads(resp.read())


def data_uri(path: Path) -> str:
    """A PNG as a base64 data URI (Meshy accepts them in place of URLs)."""
    return "data:image/png;base64," + base64.b64encode(path.read_bytes()).decode()


def figure_views(object_id: str) -> list[Path]:
    """Front, side and back of a battle figurine (the shipped GA3 sheet cut)."""
    folder = GA3 / ("l3" if object_id in FIGURES_L3 else "l4") / object_id
    return [folder / f"{view}.png" for view in ("front", "side", "back")]


def decor_view(object_id: str) -> Path:
    """The front view of a decor object: the ``__r3d`` variant when it exists."""
    for folder in (DN / f"{object_id}__r3d", DN / object_id):
        if (cut := folder / "cut/s1337.png").exists():
            return cut
    raise FileNotFoundError(object_id)


def submit(object_id: str, wave: int) -> dict:
    """Create the task of one object; return its ledger entry."""
    if wave == 1:
        task_type = "multi-image-to-3d"
        body = {
            "image_urls": [data_uri(p) for p in figure_views(object_id)],
            "ai_model": "meshy-7.1",
            "pose_mode": "a-pose",
        }
        source = str(figure_views(object_id)[0].parent)
    else:
        task_type = "image-to-3d"
        source_path = decor_view(object_id)
        body = {"image_url": data_uri(source_path), "ai_model": "meshy-6-lite"}
        source = str(source_path)
    body |= {"should_texture": True, "target_formats": ["glb"]}
    task_id = request("POST", f"/v1/{task_type}", body)["result"]
    return {
        "wave": wave,
        "task_id": task_id,
        "task_type": task_type,
        "source": source,
        "model": body["ai_model"],
        "status": "PENDING",
    }


def load() -> dict:
    """The task ledger (object id -> entry)."""
    return json.loads(LEDGER.read_text()) if LEDGER.exists() else {}


def save(ledger: dict) -> None:
    """Write the task ledger."""
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    LEDGER.write_text(json.dumps(ledger, indent=1))


def refresh(object_id: str, entry: dict) -> None:
    """Poll one unfinished task and download its glb when it succeeds."""
    task = request("GET", f"/v1/{entry['task_type']}/{entry['task_id']}")
    entry["status"] = task["status"]
    if task["status"] == "SUCCEEDED":
        out = RAW / object_id / "meshy.glb"
        out.parent.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(task["model_urls"]["glb"], out)
        entry["glb"] = str(out)
        entry["credits"] = task.get("consumed_credits")
        print(f"done {object_id} -> {out}")
    elif task["status"] in ("FAILED", "CANCELED"):
        entry["error"] = task.get("task_error")
        print(f"FAILED {object_id}: {entry['error']}")


def balance() -> int:
    """Current credit balance."""
    return request("GET", "/v1/balance")["balance"]


def run(waves: list[int], min_balance: int) -> None:
    """Submit every object of the waves in order, keeping the queue full, until done or out of credits."""
    ledger = load()
    todo = [(w, o) for w in waves for o in WAVES[w].split() if o not in ledger]
    while True:
        open_ids = [
            o for o, e in ledger.items() if e["status"] in ("PENDING", "IN_PROGRESS")
        ]
        for object_id in open_ids:
            refresh(object_id, ledger[object_id])
        save(ledger)
        open_count = sum(
            e["status"] in ("PENDING", "IN_PROGRESS") for e in ledger.values()
        )
        credits = balance()
        while todo and open_count < QUEUE_MAX:
            wave, object_id = todo[0]
            cost = COST["figure" if wave == 1 else "decor"]
            if credits - cost < min_balance:
                print(f"stop: balance {credits}, next {object_id} costs {cost}")
                todo = []
                break
            try:
                ledger[object_id] = submit(object_id, wave)
            except urllib.error.HTTPError as err:  # queue full: retry on the next poll
                print(f"submit {object_id}: HTTP {err.code} {err.read()[:200]!r}")
                break
            todo.pop(0)
            open_count += 1
            credits -= cost
            print(f"submitted {object_id} (wave {wave})")
            save(ledger)
        if not todo and open_count == 0:
            print(f"all done, balance {balance()}")
            return
        time.sleep(30)


def status() -> None:
    """Print one line per ledger entry and the balance."""
    for object_id, entry in load().items():
        print(f"{entry['wave']} {entry['status']:<11} {object_id}")
    print(f"balance {balance()}")


def shipped_t1(object_id: str, wave: int) -> Path | None:
    """The TRELLIS 1 model currently shipped for an object (raw glb for figurines)."""
    if wave == 1:
        return figure_views(object_id)[0].parent / "multi_front_back.glb"
    found = sorted((REPO / "game/assets/models/dn").glob(f"*/{object_id}_lod0.glb"))
    return found[0] if found else None


def sheets(waves: list[int], per_sheet: int) -> None:
    """Per object a Blender strip (reference, shipped T1, Meshy), then one montage per wave."""
    from PIL import Image

    out_dir = RAW / "sheets/final"
    out_dir.mkdir(parents=True, exist_ok=True)
    ledger = load()
    for wave in waves:
        strips = []
        for object_id in WAVES[wave].split():
            entry = ledger.get(object_id, {})
            if not entry.get("glb"):
                continue
            strip = out_dir / f"{object_id}.jpg"
            if not strip.exists():
                if wave == 1:
                    views = figure_views(object_id)
                    ref = f"{views[0]}:{views[2]}"
                else:
                    ref = f"{entry['source']}:{entry['source']}"
                cmd = [
                    "blender",
                    "-b",
                    "--factory-startup",
                    "-P",
                    str(REPO / "tools/blender_scripts/ga3_compare_render.py"),
                    "--",
                    "--out",
                    str(strip),
                    "--stats",
                    str(out_dir / f"{object_id}_stats.json"),
                    "--ref",
                    ref,
                    object_id,
                ]
                if wave != 1:
                    cmd += ["--fit-width", "1.8"]
                if (t1 := shipped_t1(object_id, wave)) and t1.exists():
                    cmd += ["--panel", str(t1), "T1 actuel"]
                cmd += ["--panel", entry["glb"], entry["model"]]
                subprocess.run(
                    cmd,
                    check=True,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
            strips.append(Image.open(strip))
        for start in range(0, len(strips), per_sheet):
            group = strips[start : start + per_sheet]
            cols = 2
            rows = (len(group) + cols - 1) // cols
            w = max(s.width for s in group)
            h = max(s.height for s in group)
            board = Image.new("RGB", (w * cols, h * rows), (40, 40, 44))
            for k, s in enumerate(group):
                board.paste(s, ((k % cols) * w, (k // cols) * h))
            scale = min(1.0, 2400 / board.width)
            board = board.resize((int(board.width * scale), int(board.height * scale)))
            path = out_dir / f"wave{wave}_{start // per_sheet + 1}.jpg"
            board.save(path, quality=85)
            print(f"sheet {path}")


def ingest(waves: list[int]) -> None:
    """Put the finished models in game: GA3 bake for figurines, ``dn-ingest`` for decor.

    Decor keeps the class and size of its current ``dn_manifest.json`` entry; an object already
    ingested from Meshy (``model_3d`` starts with ``meshy``) is skipped.
    """
    ledger = load()
    manifest = json.loads((REPO / "data/art/dn_manifest.json").read_text())
    entries = manifest["assets"]
    classes = json.loads((REPO / "data/art/dn_ingest_classes.json").read_text())
    classes = classes.get("classes", classes)
    failed = []
    for wave in waves:
        for object_id in WAVES[wave].split():
            entry = ledger.get(object_id, {})
            if not entry.get("glb"):
                continue
            if wave == 1:
                cmd = [
                    "blender",
                    "-b",
                    "--factory-startup",
                    "--python",
                    str(REPO / "tools/blender_scripts/ga3_figures.py"),
                    "--",
                    object_id,
                    "--glb",
                    entry["glb"],
                ]
            else:
                current = entries.get(object_id)
                if current is None:
                    print(f"skip {object_id}: not in dn_manifest.json")
                    continue
                if str(current.get("model_3d", "")).startswith("meshy"):
                    continue
                axis = classes[current["class"]]["scale_axis"]
                cmd = [
                    "uv",
                    "run",
                    "--project",
                    "tools",
                    "cent-ans",
                    "dn-ingest",
                    entry["glb"],
                    "--id",
                    object_id,
                    "--class",
                    current["class"],
                    f"--{axis}",
                    str(current["dimensions_m"][axis]),
                    "--model-3d",
                    entry["model"],
                    "--source-image",
                    entry["source"],
                ]
            result = subprocess.run(cmd, cwd=REPO, capture_output=True, text=True)
            status = "ok" if result.returncode == 0 else f"FAILED ({result.returncode})"
            print(f"ingest {object_id}: {status}")
            if result.returncode != 0:
                failed.append(object_id)
                print(result.stdout[-1500:], result.stderr[-1500:])
    print(f"failed: {failed}" if failed else "all ingested")


def main() -> None:
    """Command line entry point."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("run", "status", "sheets", "ingest"))
    parser.add_argument("--wave", type=int, action="append")
    parser.add_argument("--min-balance", type=int, default=20)
    parser.add_argument("--per-sheet", type=int, default=8)
    args = parser.parse_args()
    if args.command == "run":
        run(args.wave or sorted(WAVES), args.min_balance)
    elif args.command == "ingest":
        ingest(args.wave or sorted(WAVES))
    elif args.command == "sheets":
        sheets(args.wave or sorted(WAVES), args.per_sheet)
    else:
        status()


if __name__ == "__main__":
    main()
