extends SceneTree

## Captures du lot ZG8 (relief exagéré façon Total War) aux mêmes cadrages avant/après.
## Fenêtre réelle (pas headless) :
##   godot --path game --script res://tests/zg8_relief_shots.gd -- --out=<dossier> --prefix=after
##   godot --path game --script res://tests/zg8_relief_shots.gd -- --out=<dossier> --prefix=before --no-relief-exaggeration
## JPEG ≤ 960 px de large. `--only=<nom>` : une seule vue.

## [nom, point carte (px 4096), distance caméra (unités)]
const VIEWS := [
	["pyrenees", Vector2(1855, 2826), 40.0],
	["alpes", Vector2(2652, 2404), 45.0],
	["massif_central", Vector2(2233, 2405), 30.0],
	["galles", Vector2(1694, 1188), 22.0],
	["falaises_normandes", Vector2(2018, 1772), 6.0],
	["coteaux_seine", Vector2(2127, 1853), 8.0],
	["paris", Vector2(2214, 1924), 10.0],
	["pyrenees_pres", Vector2(1855, 2826), 3.0],
	["france", Vector2(2100, 2100), 900.0],
]


func _init() -> void:
	var out_dir := "user://zg8"
	var prefix := "shot"
	var only := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.substr(6)
		elif arg.begins_with("--prefix="):
			prefix = arg.substr(9)
		elif arg.begins_with("--only="):
			only = arg.substr(7)
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("zg8 shots: campaign map failed to load")
		quit(1)
		return
	var vegetation: Node = map.get_node_or_null("Vegetation")
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.map_data
	var terrain: TerrainBuilder = map.get("terrain")
	var ui: CanvasItem = map.get("ui") as CanvasItem
	if ui != null:
		ui.visible = false  # interface masquée : le relief seul
	print("ZG8 relief gain (strategic) %.2f, floor %s" % [MapData.relief_gain(), MapData.has_relief_floor()])
	for view in VIEWS:
		if only != "" and view[0] != only:
			continue
		var point: Vector2 = view[1]
		rig.look_at_point(Vector3(point.x, data.surface_world_at(point.x, point.y), point.y), view[2])
		rig.snap()
		for i in 30:
			await process_frame
		var guard := 0
		while guard < 1200:
			guard += 1
			var busy := false
			if vegetation != null and vegetation.has_method("pending_jobs") and vegetation.pending_jobs() > 0:
				busy = true
			if terrain != null and (not terrain.fine_ready() or terrain.pending_rescales() > 0):
				busy = true
			if ReliefLandcover.pending():
				busy = true
			if not busy:
				break
			await process_frame
		for i in 40:
			await process_frame
		var image := root.get_viewport().get_texture().get_image()
		if image.get_width() > 960:
			image.resize(960, roundi(image.get_height() * 960.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
		var path := out_dir.path_join("%s_%s.jpg" % [prefix, view[0]])
		image.save_jpg(path, 0.82)
		print("ZG8 shot %s (scale ×%.2f, gain %.2f)" % [path, MapData.vertical_exaggeration(), MapData.relief_gain()])
	map.queue_free()
	await process_frame
	quit(0)
