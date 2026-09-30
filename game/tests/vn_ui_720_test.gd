extends SceneTree

## Lot VN : vérifie à 1280×720 que l'interface tient dans l'écran (menu principal, tutoriel vs
## fenêtre modale, panneau de province vs mini-carte, bandeau de fin de tour).
## Usage : godot --headless --path game --script res://tests/vn_ui_720_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const LOGICAL := Vector2i(1422, 800)

var _failures := 0
var _viewport: SubViewport


func _init() -> void:
	# Fenêtre 1280×720 à l'échelle automatique de l'interface (0,9) : vue logique 1422×800. Le
	# headless garde une fenêtre de 64×64 ; une SubViewport de cette taille la remplace.
	_viewport = SubViewport.new()
	_viewport.size = LOGICAL
	_viewport.render_target_update_mode = SubViewport.UPDATE_DISABLED
	root.add_child(_viewport)
	await process_frame
	await _menu()
	await _tutorial_vs_modal()
	await _map_views()
	print("vn_ui_720_test: %s" % ("OK" if _failures == 0 else "FAIL (%d)" % _failures))
	quit(0 if _failures == 0 else 1)


func _check(ok: bool, what: String) -> void:
	if not ok:
		_failures += 1
		printerr("vn_ui_720_test: FAIL %s" % what)


func _screen() -> Rect2:
	return Rect2(Vector2.ZERO, Vector2(LOGICAL))


## 1. Menu principal : toutes les entrées tiennent, même avec « Continuer » et sa ligne de détail,
## à la hauteur logique de 1280×720 (800) et à des hauteurs plus basses (échelle d'interface 1,0 : 720).
func _menu() -> void:
	for height in [800, 720, 640]:
		await _menu_at(height)


func _menu_at(height: int) -> void:
	_viewport.size = Vector2i(LOGICAL.x, height)
	var menu: Control = (load("res://scenes/start_menu.tscn") as PackedScene).instantiate()
	_viewport.add_child(menu)
	for i in 10:
		await process_frame
	menu.continue_button.show()
	menu._continue_detail.text = "Royaume de France — 1337"
	menu._continue_detail.get_parent().show()
	for i in 4:
		await process_frame
	var screen := Rect2(Vector2.ZERO, Vector2(LOGICAL.x, height))
	var previous_bottom := 0.0
	for button: Button in menu._menu_buttons:
		if not button.visible:
			continue
		var rect := button.get_global_rect()
		_check(screen.encloses(rect), "menu@%d button '%s' %s outside %s" % [height, button.text, rect, screen])
		_check(rect.position.y >= previous_bottom - 0.5, "menu button '%s' overlaps the previous one" % button.text)
		previous_bottom = rect.end.y
	_check(previous_bottom <= screen.end.y, "menu@%d list ends at %s, below the screen" % [height, previous_bottom])
	menu.queue_free()
	_viewport.size = LOGICAL
	await process_frame


## 2. Tutoriel : le parchemin s'efface sous une fenêtre modale (`UiZones.Zone.MODAL`) et revient à
## sa fermeture ; une étape jouée dans une modale (`modal_ok`) le garde.
func _tutorial_vs_modal() -> void:
	var overlay: TutorialOverlay = (load("res://scenes/ui/tutorial.tscn") as PackedScene).instantiate()
	_viewport.add_child(overlay)
	var modal := Panel.new()
	modal.custom_minimum_size = Vector2(600, 400)
	_viewport.add_child(modal)
	UiZones.put(UiZones.Zone.MODAL, modal)
	modal.hide()
	overlay.show_step({"title": "Votre place dans la féodalité", "text": "x", "objective": "y"}, 0, 3)
	for i in 4:
		await process_frame
	_check(overlay.panel.visible, "tutorial panel visible without modal")
	modal.show()
	for i in 4:
		await process_frame
	_check(not overlay.panel.visible, "tutorial panel hidden while a modal window is open")
	modal.hide()
	for i in 4:
		await process_frame
	_check(overlay.panel.visible, "tutorial panel back after the modal closes")
	overlay.show_step({"title": "Les technologies", "text": "x", "objective": "y", "modal_ok": true}, 1, 3)
	modal.show()
	for i in 4:
		await process_frame
	_check(overlay.panel.visible, "modal_ok step keeps its panel over a modal")
	modal.hide()
	overlay.queue_free()
	modal.queue_free()
	await process_frame


## 3 et 4. Carte de campagne (20 s de chargement) : panneau de province et bandeau de fin de tour.
func _map_views() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "interface/season_report", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	_viewport.add_child(map)
	for i in 30:
		await process_frame
	var ui: Node = map.ui
	# 3. Panneau de province : dans la zone SIDE_PANEL, au-dessus de la mini-carte ; garnison lisible.
	map._stage_screenshot_province()
	for i in 30:
		await process_frame
	var panel: Control = ui.province_panel
	var side := UiZones.rect(UiZones.Zone.SIDE_PANEL)
	var mini: Control = map.minimap_ctl.get("minimap")
	var panel_rect := panel.get_global_rect()
	_check(panel.visible, "province panel visible")
	_check(panel_rect.end.y <= side.end.y + 1.0, "province panel bottom %s below its zone %s" % [panel_rect.end.y, side.end.y])
	_check(panel_rect.end.y <= mini.get_global_rect().position.y, "province panel %s runs under the minimap %s" % [panel_rect, mini.get_global_rect()])
	_check(ui.province_panel.garrison_list.get_child_count() > 0, "province panel lists a garrison")
	for row: Button in ui.province_panel.garrison_list.get_children():
		_check(not row.clip_text and row.size.y >= row.get_minimum_size().y, "garrison row '%s' is cut (size %s)" % [row.text, row.size])
		_check(row.get_global_rect().end.x <= panel_rect.end.x, "garrison row '%s' sticks out of the panel" % row.text)
	# 4. Bandeau « Tour des autres factions » : ses textes tiennent dans le bandeau, dans la zone TOASTS.
	ui.show_turn_banner()
	for i in 5:
		await process_frame
	var banner: Control = ui.turn_banner
	var toasts := UiZones.rect(UiZones.Zone.TOASTS)
	var banner_rect := banner.get_global_rect()
	_check(banner.visible, "turn banner visible")
	_check(banner_rect.end.x <= toasts.end.x + 1.0, "turn banner %s wider than its zone %s" % [banner_rect, toasts])
	for label: Label in [ui._turn_banner_title, ui._turn_banner_detail]:
		var rect := label.get_global_rect()
		_check(rect.position.x >= banner_rect.position.x and rect.end.x <= banner_rect.end.x, "banner text '%s' sticks out of the banner" % label.text)
		var lines := label.get_line_count()
		var line_height := label.get_line_height()
		_check(label.size.y + 0.5 >= lines * line_height, "banner text '%s' is cut (%d lines in %s px)" % [label.text, lines, label.size.y])
		_check(label.autowrap_mode != TextServer.AUTOWRAP_OFF or label.get_minimum_size().x <= label.size.x, "banner text '%s' is wider than its label" % label.text)
	map.queue_free()
	await process_frame
