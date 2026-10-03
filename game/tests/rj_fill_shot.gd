extends SceneTree

## Captures du lot RJ-d (ADR 0175, lavis par position diplomatique) : France jouée, vue moyenne
## sur la frontière Guyenne anglaise / Poitou (or nous, rouge ennemi, gris les autres), puis
## de près (atténuation). Fenêtre obligatoire (pas de --headless). PNG non commités.
## Usage : godot --path game --script res://tests/rj_fill_shot.gd -- <dossier> [vue…]
##   vues : mid (défaut), near, far, off (même cadrage que mid, lavis coupé : A/B)

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const VIEW := Vector2i(1280, 800)
const VIEWS := {
	"mid": ["prov_poitou", 520.0, true],
	"near": ["prov_poitou", 120.0, true],
	"far": ["prov_berry", 1000.0, true],
	"off": ["prov_poitou", 520.0, false],
}


func _init() -> void:
	await process_frame
	var args := OS.get_cmdline_user_args()
	var folder: String = args[0] if not args.is_empty() else "user://"
	var wanted: Array = Array(args.slice(1)) if args.size() > 1 else ["mid"]
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
	for key: String in wanted:
		if not VIEWS.has(key):
			continue
		var view: Array = VIEWS[key]
		var fill: StanceFill = map.get("stance_fill")
		if fill != null:
			fill.set_enabled(bool(view[2]))
		var centroid: Vector2 = map.map_data.centroid_of_id(str(view[0]))
		var ground := Vector3(centroid.x, map.map_data.surface_world_at(centroid.x, centroid.y), centroid.y)
		map.camera_rig.look_at_point(ground, float(view[1]))
		map.camera_rig.snap()
		for i in 40:
			await process_frame
		var image := root.get_texture().get_image()
		var path := folder.path_join("rj-fill-" + key + ".png")
		print("rj_fill_shot: %s → %s" % [path, error_string(image.save_png(path))])
	quit(0)
