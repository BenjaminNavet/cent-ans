extends SceneTree

## Captures du lot EN (ADR 0155, ennemis lisibles) : mode politique vu de la France, Guyenne
## anglaise (frontière, armées et villes ennemies en rouge) de loin puis de près. Fenêtre
## obligatoire (pas de --headless).
## Usage : godot --path game --script res://tests/en_shot.gd -- <dossier de sortie>

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const VIEW := Vector2i(1280, 800)
const VIEWS := [["en-guyenne-far", "prov_guyenne", 520.0], ["en-guyenne-near", "prov_guyenne", 150.0]]


func _init() -> void:
	await process_frame
	var args := OS.get_cmdline_user_args()
	var folder: String = args[0] if not args.is_empty() else "user://"
	root.size = VIEW
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 30:
		await process_frame
	for view: Array in VIEWS:
		var centroid: Vector2 = map.map_data.centroid_of_id(str(view[1]))
		var ground := Vector3(centroid.x, map.map_data.surface_world_at(centroid.x, centroid.y), centroid.y)
		map.camera_rig.look_at_point(ground, float(view[2]))
		map.camera_rig.snap()
		for i in 40:
			await process_frame
		var image := root.get_texture().get_image()
		var path := folder.path_join(str(view[0]) + ".png")
		print("en_shot: %s → %s" % [path, error_string(image.save_png(path))])
	quit(0)
