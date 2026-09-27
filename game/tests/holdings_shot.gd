extends SceneTree

## Capture de contrôle du lot HL2 : panneau « Colonies » (touche B) ouvert sur la vraie
## simulation. Une seule lecture d'image par la session principale pour juger la lisibilité.
## Usage (avec affichage ; sans écran, le rendu est vide) :
##   godot --path game --script res://tests/holdings_shot.gd
## Écrit `docs/img/holdings.png` (640 px de large).

const WIDTH := 640
const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _init() -> void:
	await process_frame
	var out_dir := ProjectSettings.globalize_path("res://").path_join("../docs/img").simplify_path()
	DirAccess.make_dir_recursive_absolute(out_dir)
	root.size = Vector2i(1280, 800)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 10:
		await process_frame
	if not map.load_ok or map.sim == null:
		push_error("holdings_shot: campaign map with the real simulation failed to start")
		quit(1)
		return
	var holdings: Node = map.holdings_ctl
	if holdings == null or not holdings.available():
		push_error("holdings_shot: holdings controller missing (run core/build.sh)")
		quit(1)
		return
	map.camera_rig.edge_pan_enabled = false
	holdings.toggle()
	await _settle()
	_save(out_dir.path_join("holdings.png"))
	quit(0)


func _settle() -> void:
	for i in 40:
		await process_frame


func _save(path: String) -> void:
	var image := root.get_texture().get_image()
	if image == null or image.is_empty():
		push_error("holdings_shot: empty frame (no display?)")
		return
	image.resize(WIDTH, int(round(float(image.get_height()) * WIDTH / image.get_width())), Image.INTERPOLATE_LANCZOS)
	image.save_png(path)
	print("holdings_shot: wrote %s" % path)
