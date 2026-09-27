"""(Runs inside the Unreal editor.) Import the prototype glTF assets, enable Nanite, print bounds."""

import json

import unreal

HERE = "/Users/jean_hubert/dev/game_project/tools/proto_moteur"
scene = json.load(open(f"{HERE}/scene.json"))
tools = unreal.AssetToolsHelpers.get_asset_tools()
for name in scene["assets"]:
    destination = f"/Game/Proto/{name}"
    if unreal.EditorAssetLibrary.does_directory_exist(destination):
        continue
    task = unreal.AssetImportTask()
    task.filename = f"{HERE}/assets/{name}.glb"
    task.destination_path = destination
    task.automated = True
    task.replace_existing = True
    task.save = True
    tools.import_asset_tasks([task])

for name in scene["assets"]:
    for path in unreal.EditorAssetLibrary.list_assets(f"/Game/Proto/{name}", recursive=True):
        asset = unreal.EditorAssetLibrary.load_asset(path)
        if isinstance(asset, unreal.StaticMesh):
            nanite = asset.get_editor_property("nanite_settings")
            if not nanite.enabled:
                nanite.enabled = True
                asset.set_editor_property("nanite_settings", nanite)
                unreal.EditorAssetLibrary.save_loaded_asset(asset)
            box = asset.get_bounding_box()
            print(f"PROTO mesh {name} {asset.get_name()} min {box.min} max {box.max}")
