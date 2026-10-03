extends SceneTree

## Captures du lot RJ-c (ADR 0175, bannières possesseur / occupant) : la France prend la cité de
## Cornouailles (anglaise) ; vue moyenne (écu anglais + petit écu français, liserés de position)
## puis parchemin (fanions). Fenêtre obligatoire (pas de --headless).
## Usage : godot --path game --script res://tests/rj_banner_shot.gd -- <dossier de sortie>

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const VIEW := Vector2i(1280, 800)
const PROVINCE := "prov_cornwall"
const VIEWS := [["rj-banners-near", 260.0], ["rj-banners-parchment", 1700.0]]


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
	map.sim.call("debug_capture_place", PROVINCE)
	for capture in map.sim.call("get_pending_captures"):
		map.sim.call("choose_capture_outcome", int(capture.get("id", -1)), "occupy")
	map.refresh_all()
	for view: Array in VIEWS:
		var centroid: Vector2 = map.map_data.centroid_of_id(PROVINCE)
		var ground := Vector3(centroid.x, map.map_data.surface_world_at(centroid.x, centroid.y), centroid.y)
		map.camera_rig.look_at_point(ground, float(view[1]))
		map.camera_rig.snap()
		for i in 40:
			await process_frame
		var image := root.get_texture().get_image()
		var path := folder.path_join(str(view[0]) + ".png")
		print("rj_banner_shot: %s → %s" % [path, error_string(image.save_png(path))])
	quit(0)
