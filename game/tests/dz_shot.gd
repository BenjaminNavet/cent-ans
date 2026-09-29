extends SceneTree

## Captures du lot DZ (frontières diplomatiques) : mode Diplomatie vu de la France, puis vu de
## l'Angleterre (clic sur la Guyenne). Fenêtre obligatoire (pas de --headless).
## Usage : godot --path game --script res://tests/dz_shot.gd -- <dossier de sortie>

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const VIEW := Vector2i(1600, 900)


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
	var modes: Node = map.map_modes
	modes.set_mode("diplomacy")
	await _shot(folder.path_join("dz-france.png"))
	map._on_province_selected(map.map_data.index_of_id("prov_guyenne"))
	await _shot(folder.path_join("dz-england.png"))
	quit(0)


func _shot(path: String) -> void:
	for i in 12:
		await process_frame
	var image := root.get_texture().get_image()
	print("dz_shot: %s → %s" % [path, error_string(image.save_png(path))])
