extends SceneTree

## Captures du lot JR3 (fenêtre obligatoire, pas de --headless ; 1280 px de large) dans
## `docs/img/jr/` (dossier ignoré par git) : campagne croisée, panneau de faction ouvert sur la
## section « Ferveur », repère « Ferveur » dans la barre du haut.
##  - `jr-ferveur.png` : état de départ (bouton « Prêcher le passage » disponible) ;
##  - `jr-ferveur-passage.png` : passage prêché (contingent attendu, bouton en recharge).
## Les réglages vont sur le fichier de test (`Settings.use_test_file`).
## Usage : godot --path game --script res://tests/jr_crusade_shot.gd [-- <dossier de sortie>]

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const OUT_DIR := "../docs/img/jr/"
const VIEW := Vector2i(1280, 800)
const WIDTH := 1280


func _init() -> void:
	await process_frame
	root.size = VIEW
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "feudal_tutorial/done", true, false)
		settings.call("set_value", "interface/season_report", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_crusaders"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	map.camera_rig.edge_pan_enabled = false
	for i in 30:
		await process_frame
	if map.sim.has_method("set_chronicle_enabled"):
		map.sim.call("set_chronicle_enabled", false)
	map.ui.faction_panel_requested.emit()
	for i in 40:
		await process_frame
	_save("jr-ferveur.png")
	map.ui.faction_panel.crusade_section.request_preach()
	for i in 40:
		await process_frame
	_save("jr-ferveur-passage.png")
	quit(0)


func _save(file: String) -> void:
	var args := OS.get_cmdline_user_args()
	var folder: String = args[0] if not args.is_empty() else ProjectSettings.globalize_path("res://" + OUT_DIR)
	DirAccess.make_dir_recursive_absolute(folder)
	var image := root.get_viewport().get_texture().get_image()
	if image.get_width() != WIDTH:
		image.resize(WIDTH, int(image.get_height() * float(WIDTH) / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var path := folder.path_join(file)
	print("jr_crusade_shot: %s → %s" % [path, error_string(image.save_png(path))])
