extends SceneTree

## Lot FA5 : captures de contrôle des fenêtres enluminées à 1280×720 (province, cour, faction,
## techniques, diplomatie, chronique) et d'une planche de titres à lettrine, écrites dans `<out>/<vue>.png`. `--no-fa` rend l'habillage
## d'avant FA5 (comparaison avant/après). Carte de campagne réelle (France, graine 1337).
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1280x720 --script res://tests/fa5_ui_shot.gd -- --out=<dossier> [--no-fa] [--only=court,tech]

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

const LETTRINE_TITLES := [
	"Diplomatie", "Cour — France", "Techniques", "Île-de-France", "Paris", "Essex",
	"Dauphiné", "Chronique", "Angleterre", "Évreux", "Bourgogne", "Poitou",
]

## Vues sans carte de campagne.
const STANDALONE := ["lettrines"]

var _out := ""
var _only := PackedStringArray()


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--only="):
			_only = arg.trim_prefix("--only=").split(",", false)
		elif arg == "--no-fa":
			FaUi.enabled = false
	_run.call_deferred()
	create_timer(240.0).timeout.connect(func() -> void:
		print("fa5_ui_shot: timeout")
		quit(1))


func _wants(view: String) -> bool:
	return _only.is_empty() or _only.has(view)


func _run() -> void:
	await process_frame
	if _out.is_empty():
		push_error("fa5_ui_shot: --out=<dir> required (work shots stay outside the repository)")
		quit(1)
		return
	DirAccess.make_dir_recursive_absolute(_out)
	if _wants("lettrines"):
		await _lettrine_sheet()
	if not _only.is_empty() and Array(_only).all(func(view: String) -> bool: return STANDALONE.has(view)):
		quit(0)
		return
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
	current_scene = map
	for _i in 90:
		await process_frame

	if _wants("province"):
		map.call("_stage_screenshot_province")
		await _shot("province")
		map.ui.hide_province()
	if _wants("court"):
		map.call("_on_court_panel_requested")
		await _shot("court")
		map.ui.court_panel.hide()
	if _wants("faction"):
		map.call("_show_faction_panel", map.player_faction)
		await _shot("faction")
		map.ui.faction_panel.hide()
	if _wants("tech"):
		map.call("_on_tech_panel_requested")
		await _shot("tech")
		map.ui.tech_panel.hide()
	if _wants("diplomacy"):
		var controller: Node = map.diplomacy
		if controller != null and controller.call("available"):
			controller.call("open_panel")
			await _shot("diplomacy")
			controller.panel.hide()
	if _wants("chronicle"):
		map.chronicle.call("stage_screenshot")
		await _shot("chronicle")
	print("FA5_SHOT done")
	quit(0)


## Planche de titres à lettrine sur vélin (une ligne par titre, tailles Title et Heading).
func _lettrine_sheet() -> void:
	var page := PanelContainer.new()
	page.theme = load("res://scenes/ui/parchment_theme.tres") as Theme
	page.add_theme_stylebox_override("panel", HudStyle.illuminated_box())
	page.position = Vector2(20, 20)
	var grid := GridContainer.new()
	grid.columns = 3
	grid.add_theme_constant_override("h_separation", 40)
	grid.add_theme_constant_override("v_separation", 14)
	page.add_child(grid)
	for title in LETTRINE_TITLES:
		var label := Label.new()
		label.text = title
		UiType.apply(label, UiType.TITLE)
		label.add_theme_color_override("font_color", HudStyle.INK)
		grid.add_child(label)
		Lettrine.attach(label)
	root.add_child(page)
	await _shot("lettrines")
	page.queue_free()


func _shot(view: String) -> void:
	var until := Time.get_ticks_msec() + 1200
	while Time.get_ticks_msec() < until:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_viewport().get_texture().get_image()
	var path := _out.path_join(view + ".png")
	print("FA5_SHOT %s (%s)" % [path, error_string(image.save_png(path))])
