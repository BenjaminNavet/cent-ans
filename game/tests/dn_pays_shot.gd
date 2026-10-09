extends SceneTree

## DN-PAYS : captures de la campagne vivante hors champs, sans interface. Chaque vue vise l'instance
## de la règle donnée la plus proche d'un point (lon, lat).
## Usage : tools/godot_bg.sh --path game --resolution 800x500 \
##   --script res://tests/dn_pays_shot.gd -- --out=<dossier> [--only=bocage] [--distances=6,20]
## Les PNG sont écrits hors dépôt (jamais commités).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
# nom, règle, lon, lat
const VIEWS := [
	["bocage", "bocage_enclos", -1.4, 48.0],
	["salines", "salines_atlantique", -2.45, 47.3],
	["moulins", "moulins_polder", 4.3, 52.0],
	["route", "charrettes_bord_de_route", 2.5, 48.5],
	["caravane", "caravanes_chameaux", 6.0, 34.0],
]


func _init() -> void:
	var out := CmdArgs.value("--out", "user://pays_shots")
	var distances := CmdArgs.value("--distances", "6,20").split(",")
	var only := CmdArgs.value("--only", "")
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
	for _i in 5:
		await process_frame
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	rig.floor_distance = 0.0
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var data: MapData = map.get("map_data")
	var layer_node: CountrysideLayer = map.life.countryside
	layer_node.force_active = true
	DirAccess.make_dir_recursive_absolute(out)
	for view in VIEWS:
		if only != "" and not String(view[0]).contains(only):
			continue
		var px := FaunaLayer.lonlat_to_px(float(view[2]), float(view[3]), data)
		var best := px
		var best_d := 1e9
		for cy in range(floori(px.y / 96.0) - 1, floori(px.y / 96.0) + 2):
			for cx in range(floori(px.x / 96.0) - 1, floori(px.x / 96.0) + 2):
				for inst: Dictionary in layer_node.cell_instances(Vector2i(cx, cy)):
					var d := (inst["pos"] as Vector2).distance_to(px)
					if inst["rule"] == view[1] and d < best_d:
						best_d = d
						best = inst["pos"]
		for dist_text in distances:
			rig.look_at_point(Vector3(best.x, data.surface_world_at(best.x, best.y), best.y), float(dist_text))
			rig.snap()
			for _i in 150:
				await process_frame
			await RenderingServer.frame_post_draw
			var path := out.path_join("%s_d%s.png" % [view[0], dist_text])
			root.get_texture().get_image().save_png(path)
			print("dn_pays_shot: ", path, " px=", best, " stats=", layer_node.stats)
	map.queue_free()
	await process_frame
	quit(0)
