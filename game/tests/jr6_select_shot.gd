extends SceneTree

## Captures du lot JR6 (fenêtre obligatoire, pas de --headless ; 1280 px de large) dans
## `docs/img/jr/` (dossier ignoré par git) : choix de faction, onglet « Défis singuliers » avec
## la carte des Croisés (`jr6-defis.png`), puis l'onglet « Toutes les factions » avec le groupe
## « Sans terre » en tête de liste (`jr6-toutes.png`).
## Usage : godot --path game --script res://tests/jr6_select_shot.gd [-- <dossier de sortie>]

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const OUT_DIR := "../docs/img/jr/"
const VIEW := Vector2i(1600, 1000)
const WIDTH := 1280


func _init() -> void:
	await process_frame
	root.size = VIEW
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	var select := FactionSelect.new()
	select.size = Vector2(VIEW)
	root.add_child(select)
	for i in 10:
		await process_frame
	if select.special_page != null:
		select.start_tabs.current_tab = select.special_page.get_index()
	select.select("fac_crusaders")
	for i in 10:
		await process_frame
	_save("jr6-defis.png")
	select.start_tabs.current_tab = select.map_tab_index()
	for i in 10:
		await process_frame
	_save("jr6-toutes.png")
	quit(0)


func _save(file: String) -> void:
	var args := OS.get_cmdline_user_args()
	var folder: String = args[0] if not args.is_empty() else ProjectSettings.globalize_path("res://" + OUT_DIR)
	DirAccess.make_dir_recursive_absolute(folder)
	var image := root.get_viewport().get_texture().get_image()
	if image.get_width() != WIDTH:
		image.resize(WIDTH, int(image.get_height() * float(WIDTH) / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var path := folder.path_join(file)
	print("jr6_select_shot: %s → %s" % [path, error_string(image.save_png(path))])
