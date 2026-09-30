extends SceneTree

## Lot SS : captures du sol de campagne à plusieurs hauteurs (avant/après la pyramide de couleur).
## Usage : godot --path game --resolution 1600x900 --script res://tests/ss_shot.gd --
##   --out=<dossier> [--prefix=avant-] [--at=<x>,<z>] [--distances=900,300,90]
## Écrit `<prefix>sol-<distance>.png` (960 px de large) par distance ; HUD masqué.

const PARIS := Vector2(2213.2, 3203.9)
const WIDTH := 960
const SETTLE_FRAMES := 150


func _init() -> void:
	var out_dir := "user://ss"
	var prefix := ""
	var at := PARIS
	var distances: Array[float] = [900.0, 300.0, 90.0]
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--prefix="):
			prefix = arg.trim_prefix("--prefix=")
		elif arg.begins_with("--at="):
			var xz := arg.trim_prefix("--at=").split(",")
			at = Vector2(float(xz[0]), float(xz[1]))
		elif arg.begins_with("--distances="):
			distances.clear()
			for d in arg.trim_prefix("--distances=").split(","):
				distances.append(float(d))
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "tutorial/enabled", false, false)
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("SS shot: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.get("map_data")
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var ground := Vector3(at.x, data.surface_world_at(at.x, at.y), at.y)
	for distance in distances:
		rig.look_at_point(ground, maxf(distance, rig.min_distance_at(ground)))
		rig.snap()
		for i in SETTLE_FRAMES:
			await process_frame
		_shot(out_dir, "%ssol-%d" % [prefix, int(distance)])
	quit(0)


func _shot(out_dir: String, name: String) -> void:
	var image := root.get_viewport().get_texture().get_image()
	if image.get_width() > WIDTH:
		image.resize(WIDTH, roundi(image.get_height() * float(WIDTH) / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var path := out_dir.path_join("%s.png" % name)
	image.save_png(path)
	print("SS shot %s" % path)
