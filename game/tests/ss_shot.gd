extends SceneTree

## Lot SS : captures du sol de campagne à plusieurs hauteurs (avant/après la pyramide de couleur).
## Usage : godot --path game --resolution 1600x900 --script res://tests/ss_shot.gd --
##   --out=<dossier> [--prefix=avant-] [--at=<x>,<z>] [--distances=900,300,90]
##   [--param=<uniforme>=<float>]… [--env=<propriété Environment>=<float>]… (réglage du matériau du terrain) [--stats] (moyenne/écart-type
##   RGB du bas de l'image, sol sans figures, pour comparer sans lire l'image)
## Écrit `<prefix>sol-<distance>.png` (960 px de large) par distance ; HUD masqué.

const PARIS := Vector2(2213.2, 3203.9)
const WIDTH := 960
const SETTLE_FRAMES := 150


func _init() -> void:
	var out_dir := "user://ss"
	var prefix := ""
	var at := PARIS
	var distances: Array[float] = [900.0, 300.0, 90.0]
	var params := {}
	var stats := false
	var env_params := {}
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
		elif arg.begins_with("--param="):
			var kv := arg.trim_prefix("--param=").split("=")
			params[kv[0]] = (kv[1] == "true") if kv[1] in ["true", "false"] else float(kv[1])
		elif arg.begins_with("--env="):
			var ekv := arg.trim_prefix("--env=").split("=")
			env_params[ekv[0]] = float(ekv[1])
		elif arg == "--stats":
			stats = true
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
	var terrain: TerrainBuilder = map.get("terrain")
	var material: ShaderMaterial = terrain.material if terrain != null else null
	for key in params:
		if material != null:
			material.set_shader_parameter(key, params[key])
	var world_env := root.find_children("*", "WorldEnvironment", true, false)
	for key in env_params:
		for node in world_env:
			(node as WorldEnvironment).environment.set(key, env_params[key])
	var ground := Vector3(at.x, data.surface_world_at(at.x, at.y), at.y)
	for distance in distances:
		rig.look_at_point(ground, maxf(distance, rig.min_distance_at(ground)))
		rig.snap()
		for i in SETTLE_FRAMES:
			await process_frame
		if stats:
			_stats("%ssol-%d" % [prefix, int(distance)])
		else:
			_shot(out_dir, "%ssol-%d" % [prefix, int(distance)])
	quit(0)


func _shot(out_dir: String, name: String) -> void:
	var image := root.get_viewport().get_texture().get_image()
	if image.get_width() > WIDTH:
		image.resize(WIDTH, roundi(image.get_height() * float(WIDTH) / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var path := out_dir.path_join("%s.png" % name)
	image.save_png(path)
	print("SS shot %s" % path)


## Moyenne et écart-type RGB (0-255) du tiers inférieur central de l'image (premier plan).
func _stats(name: String) -> void:
	var image := root.get_viewport().get_texture().get_image()
	var w := image.get_width()
	var h := image.get_height()
	var sum := Vector3.ZERO
	var sq := Vector3.ZERO
	var n := 0
	for y in range(h * 2 / 3, h, 4):
		for x in range(w / 4, w * 3 / 4, 4):
			var c := image.get_pixel(x, y)
			var v := Vector3(c.r, c.g, c.b) * 255.0
			sum += v
			sq += v * v
			n += 1
	var mean := sum / n
	var sd := (sq / n - mean * mean).max(Vector3.ZERO)
	print("SS stats %s mean %.0f %.0f %.0f sd %.0f %.0f %.0f" % [name, mean.x, mean.y, mean.z, sqrt(sd.x), sqrt(sd.y), sqrt(sd.z)])
