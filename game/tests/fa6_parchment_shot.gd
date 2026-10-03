extends SceneTree

## Lot FA6 : captures de la vue parchemin (caméra > 1200) pour juger les ornements de portulan.
## Usage : godot --path game --resolution 640x720 --script res://tests/fa6_parchment_shot.gd --
##   --out=<dossier> [--prefix=avant-] [--views=<nom>:<x>,<z>,<distance>;…] [--list] [--stats]
##   [--no-fa-parchment] (ancien dessin par code, lu par `ParchmentDecor`)
## `--list` écrit les emplacements des ornements (`ParchmentDecor`) ; `--stats` le nombre d'appels
## de dessin et d'objets de la dernière vue. Une vue `<nom>:rose<i>|ship<i>|monster<i>,<distance>`
## vise un ornement. Écrit `<prefix><nom>.png` à la résolution de la fenêtre ; HUD masqué, couche
## du parchemin gardée.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const SETTLE_FRAMES := 90


func _init() -> void:
	var out_dir := "user://fa6"
	var prefix := ""
	var views := "manche:1900,2950,1400"
	var list := false
	var stats := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--prefix="):
			prefix = arg.trim_prefix("--prefix=")
		elif arg.begins_with("--views="):
			views = arg.trim_prefix("--views=")
		elif arg == "--list":
			list = true
		elif arg == "--stats":
			stats = true
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("FA6 shot: campaign map failed to load")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.get("map_data")
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = (layer as CanvasLayer).name == "Parchment"
	var decor: ParchmentDecor = null
	for node in root.find_children("*", "ParchmentOverlay", true, false):
		decor = (node as ParchmentOverlay).decor
	if list and decor != null:
		print("FA6 map size %s" % [data.size])
		for kind in ["roses", "ships", "monsters"]:
			var points: Array = decor.get(kind)
			for i in points.size():
				print("FA6 %s %d %s" % [kind, i, points[i]])
	for view in views.split(";", false):
		var name := view.get_slice(":", 0)
		var fields := view.get_slice(":", 1).split(",")
		var at := Vector2.ZERO
		var distance := 1400.0
		if fields.size() == 2 and decor != null:
			var target := fields[0]
			for kind in ["rose", "ship", "monster"]:
				if target.begins_with(kind):
					var points: Array = decor.get(kind + "s")
					var p: Vector3 = points[int(target.trim_prefix(kind)) % maxi(points.size(), 1)]
					at = Vector2(p.x, p.y)
			distance = float(fields[1])
		else:
			at = Vector2(float(fields[0]), float(fields[1]))
			distance = float(fields[2])
		var ground := Vector3(at.x, data.surface_world_at(at.x, at.y), at.y)
		rig.look_at_point(ground, distance)
		rig.snap()
		for i in SETTLE_FRAMES:
			await process_frame
		var path := out_dir.path_join("%s%s.png" % [prefix, name])
		root.get_viewport().get_texture().get_image().save_png(path)
		print("FA6 shot %s" % path)
		if stats:
			print("FA6 stats %s%s draw_calls %d objects %d canvas_draw_calls %d" % [
				prefix, name,
				int(Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)),
				int(Performance.get_monitor(Performance.RENDER_TOTAL_OBJECTS_IN_FRAME)),
				int(RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME))])
	quit(0)
