extends SceneTree

## Lot VN, suite : panneaux de la carte de campagne à 1280×720 (échelle d'interface 0,9 : vue
## logique 1422×800 ; 1280×720 si l'échelle est 1,0). Chaque mise en scène reprend celle de
## `--stage=<x>` de la carte et vérifie les rectangles dans les deux vues.
## Usage : godot --headless --path game --script res://tests/vn_ui_720_b_test.gd  (≈ 1 min)

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const SIZES := [Vector2i(1422, 800), Vector2i(1280, 720)]

var _failures := 0
var _viewport: SubViewport
var _map: Node3D
var _ui: Node


func _init() -> void:
	_viewport = SubViewport.new()
	_viewport.size = SIZES[0]
	_viewport.render_target_update_mode = SubViewport.UPDATE_DISABLED
	root.add_child(_viewport)
	await process_frame
	await _budget_table_synthetic()
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
	_map = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	_viewport.add_child(_map)
	for i in 30:
		await process_frame
	_ui = _map.ui
	await _skills()
	await _objectives()
	await _settlement()
	await _agents()
	await _budget()
	await _chronicle()
	print("vn_ui_720_b_test: %s" % ("OK" if _failures == 0 else "FAIL (%d)" % _failures))
	quit(0 if _failures == 0 else 1)


func _check(ok: bool, what: String) -> void:
	if not ok:
		_failures += 1
		printerr("vn_ui_720_b_test: FAIL %s" % what)


func _resize(size: Vector2i) -> void:
	_viewport.size = size
	for i in 12:
		await process_frame


func _inside(inner: Rect2, outer: Rect2) -> bool:
	return inner.position.x >= outer.position.x - 1.0 and inner.position.y >= outer.position.y - 1.0 \
		and inner.end.x <= outer.end.x + 1.0 and inner.end.y <= outer.end.y + 1.0


## Tableau du budget : avec des montants à sept chiffres et un cadre de 404 px, « Écart » reste dedans.
func _budget_table_synthetic() -> void:
	var host := Control.new()
	host.custom_minimum_size = Vector2(404, 0)
	host.size = Vector2(404, 300)
	_viewport.add_child(host)
	var table := BudgetTable.new()
	table.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	host.add_child(table)
	table.show_budget({
		"net_change": -1234567, "net_income": 12345678, "net_income_last_turn": 13580245,
		"budget_lines": [
			{"key": "receipts", "charge": false, "projected": 12345678, "last": 13580245, "delta": -1234567},
			{"key": "buildings", "charge": true, "projected": -4186000, "last": -4186000, "delta": -1234567},
		]})
	table.size = Vector2(404, 0)
	for i in 6:
		await process_frame
	var grid := table.get_child(0) as Control
	_check(grid.get_combined_minimum_size().x <= 404.0, "budget grid needs %s px, more than the 404 of the faction panel" % grid.get_combined_minimum_size().x)
	host.queue_free()
	await process_frame


## 6. Fiche personnage (`--stage=skills`) : ouverte, dans l'écran, pas démesurée.
func _skills() -> void:
	_map._stage_screenshot_skills()
	for size: Vector2i in SIZES:
		await _resize(size)
		var sheet: Control = _ui.character_sheet
		_check(sheet.visible, "character sheet is not open at %s" % size)
		var screen := Rect2(Vector2.ZERO, Vector2(size))
		_check(_inside(sheet.get_global_rect(), screen), "character sheet %s outside the screen %s" % [sheet.get_global_rect(), screen])
	_ui.hide_character()
	await process_frame


## 5. Objectifs : le panneau masque le journal (il recouvrait « Déplier »), qui revient à sa fermeture.
func _objectives() -> void:
	_map._focus_capital()
	_map.victory.open_panel()
	for size: Vector2i in SIZES:
		await _resize(size)
		_check(not _ui.event_log.visible, "event log still shown under the objectives panel at %s" % size)
	_map.victory.panel.hide()
	await process_frame
	_check(_ui.event_log.visible, "event log does not come back after the objectives panel closes")


## 2. Fiche de ville : dans la zone `SIDE_PANEL`, au-dessus de la mini-carte, garnison lisible.
func _settlement() -> void:
	_map.settlements_ctl.stage_screenshot("settlement")
	for size: Vector2i in SIZES:
		await _resize(size)
		var side := UiZones.rect(UiZones.Zone.SIDE_PANEL)
		var mini: Control = _map.minimap_ctl.get("minimap")
		var panel := _ui.find_child("SettlementPanel", true, false) as Control
		_check(panel != null and panel.is_visible_in_tree(), "settlement panel not shown at %s" % size)
		if panel == null:
			continue
		# A6-U11 : un seul défilement, celui de l'enveloppe de la zone ; le panneau peut être plus
		# haut que la zone, mais l'enveloppe, elle, reste dans la zone et au-dessus de la minicarte.
		var scroll := panel.get_parent() as ScrollContainer
		_check(scroll != null, "settlement panel is not inside the side-zone scroll at %s" % size)
		var rect := (scroll if scroll != null else panel).get_global_rect()
		_check(rect.end.y <= side.end.y + 1.0, "settlement panel bottom %s below its zone %s at %s" % [rect.end.y, side.end.y, size])
		_check(rect.end.y <= mini.get_global_rect().position.y, "settlement panel %s runs under the minimap at %s" % [rect, size])
		_check(panel.find_children("*", "ScrollContainer", true, false).is_empty(), "nested scroll inside the settlement panel at %s" % size)
		for check: CheckBox in panel.find_children("*", "CheckBox", true, false):
			_check(not check.clip_text and check.size.y >= check.get_minimum_size().y, "settlement garrison row '%s' is cut at %s" % [check.text, size])
		var tabs := panel.find_child("Tabs", true, false) as Control
		_check(tabs.size.y >= 200.0, "settlement tabs only %s px tall at %s" % [tabs.size.y, size])
	_ui.hide_province()
	await process_frame


## 3. Agents : le journal reste en haut à gauche même avec des avis empilés, sans toucher le bandeau d'agents.
func _agents() -> void:
	_map.agents_ctl.stage_screenshot(false)
	for size: Vector2i in SIZES:
		await _resize(size)
		var toasts := UiZones.rect(UiZones.Zone.TOASTS)
		var log_rect: Rect2 = _ui.event_log.get_global_rect()
		_check(_ui.event_log.is_visible_in_tree(), "event log hidden at %s" % size)
		_check(log_rect.position.y <= toasts.position.y + 1.0, "event log at y=%s, not at the top of its zone (%s) at %s" % [log_rect.position.y, toasts.position.y, size])
		var bar := _ui.find_child("AgentBar", true, false) as Control
		if bar != null and bar.is_visible_in_tree():
			_check(not log_rect.intersects(bar.get_global_rect()), "event log %s overlaps the agent bar %s at %s" % [log_rect, bar.get_global_rect(), size])
	await process_frame


## 4. Budget : tableau dans le panneau de faction ; le cartouche de saison passe au-dessus des panneaux.
func _budget() -> void:
	_map._stage_screenshot_budget()
	for size: Vector2i in SIZES:
		await _resize(size)
		var panel: Control = _ui.faction_panel
		var screen := Rect2(Vector2.ZERO, Vector2(size))
		_check(panel.visible and _inside(panel.get_global_rect(), screen), "faction panel %s outside the screen at %s" % [panel.get_global_rect(), size])
		for table: Control in panel.find_children("*", "BudgetTable", true, false):
			var grid := table.get_child(0) as Control
			_check(grid.get_combined_minimum_size().x <= table.size.x + 0.5, "budget grid needs %s px in a %s px table at %s" % [grid.get_combined_minimum_size().x, table.size.x, size])
			_check(grid.get_global_rect().end.x <= panel.get_global_rect().end.x, "budget column sticks out of the faction panel at %s" % size)
	var banner := _ui.find_child("SeasonBanner", true, false) as Control
	_check(banner != null and banner.z_index > _ui.faction_panel.z_index, "season banner is not drawn above the faction panel")
	_ui.hide_faction()
	await process_frame


## 1. Chronique : la fenêtre (boutons de choix, effets) tient dans la zone `SIDE_PANEL`.
func _chronicle() -> void:
	_map._focus_capital()
	_map.chronicle.stage_screenshot()
	for size: Vector2i in SIZES:
		await _resize(size)
		var window: Control = _map.chronicle.window
		_check(window.is_visible_in_tree(), "chronicle window not open at %s" % size)
		var side := UiZones.rect(UiZones.Zone.SIDE_PANEL)
		for control: Control in [window] + window.find_children("*", "Button", true, false) + window.find_children("*", "Label", true, false):
			if not control.is_visible_in_tree():
				continue
			_check(control.get_global_rect().end.x <= side.end.x + 1.0, "chronicle %s '%s' sticks out at x=%s (zone ends %s) at %s" % [control.get_class(), control.get("text"), control.get_global_rect().end.x, side.end.x, size])
	await process_frame
