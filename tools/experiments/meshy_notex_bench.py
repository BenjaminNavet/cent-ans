"""MN: Meshy mesh-only bench (5 credits in meshy-6-lite instead of 15 textured).

``python3 tools/experiments/meshy_notex_bench.py submit`` creates the untextured tasks,
``... fetch`` downloads ``RAW/<id>/notex.glb`` once they succeed. Texturing is done
elsewhere (Blender projection bake, Hunyuan3D Paint); see ``docs/wip/mn-meshy-sans-texture.md``.
"""

from __future__ import annotations

import argparse
import json
import urllib.request

from meshy_waves import RAW, balance, data_uri, decor_view, request

OBJECTS = ("castle_byz", "siege_ram_roofed")
LEDGER = RAW / "tasks/notex_ledger.json"


def load() -> dict:
    """The bench ledger (object id -> task id)."""
    return json.loads(LEDGER.read_text()) if LEDGER.exists() else {}


def submit() -> None:
    """Create one mesh-only task per object, never twice."""
    ledger = load()
    print("balance before", balance())
    for object_id in OBJECTS:
        if object_id in ledger:
            continue
        body = {
            "image_url": data_uri(decor_view(object_id)),
            "ai_model": "meshy-6-lite",
            "should_texture": False,
            "target_formats": ["glb"],
        }
        ledger[object_id] = request("POST", "/v1/image-to-3d", body)["result"]
        LEDGER.write_text(json.dumps(ledger, indent=1))
        print("submitted", object_id, ledger[object_id])


def fetch() -> None:
    """Download the finished meshes."""
    for object_id, task_id in load().items():
        task = request("GET", f"/v1/image-to-3d/{task_id}")
        print(object_id, task["status"], task.get("progress"))
        if task["status"] == "SUCCEEDED":
            urllib.request.urlretrieve(
                task["model_urls"]["glb"], RAW / object_id / "notex.glb"
            )
    print("balance after", balance())


def main() -> None:
    """Command-line entry point."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("submit", "fetch"))
    {"submit": submit, "fetch": fetch}[parser.parse_args().command]()


if __name__ == "__main__":
    main()
