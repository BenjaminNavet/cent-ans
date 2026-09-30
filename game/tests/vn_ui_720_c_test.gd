extends SceneTree

## Lot VN 3 : (1) fenêtre Diplomatie dans l'écran, (2) parchemin du tutoriel (sommaire ouvert)
## dans l'écran, (3) écran de résultat de bataille : cartes des régiments entièrement visibles à
## l'ouverture en 1080p, et en 720p. Vues 1422x800 (1280x720 à l'échelle 0,9), 1280x720, 1920x1080.
## Usage : godot --headless --path game --script res://tests/vn_ui_720_c_test.gd  (≈ 1 min)

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const SIZES := [Vector2i(1422, 800), Vector2i(1280, 720), Vector2i(1138, 640)]

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
	await _result_screen()
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
	await _diplomacy()
	await _tutorial()
	print("vn_ui_720_c_test: %s" % ("OK" if _failures == 0 else "FAIL (%d)" % _failures))
	quit(0 if _failures == 0 else 1)


func _check(ok: bool, what: String) -> void:
	if not ok:
		_failures += 1
		printerr("vn_ui_720_c_test: FAIL %s" % what)


func _resize(size: Vector2i) -> void:
	_viewport.size = size
	for i in 12:
		await process_frame


func _inside(inner: Rect2, outer: Rect2) -> bool:
	return inner.position.x >= outer.position.x - 1.0 and inner.position.y >= outer.position.y - 1.0 \
		and inner.end.x <= outer.end.x + 1.0 and inner.end.y <= outer.end.y + 1.0


## 1. Diplomatie : panneau et contrôles visibles dans l'écran, colonne de droite comprise.
func _diplomacy() -> void:
	for stage in ["diplomacy", "diplomacy_treaty", "diplomacy_counter", "diplomacy_map"]:
		_map._focus_capital()
		match stage:
			"diplomacy":
				_map.diplomacy.open_panel("fac_england")
			"diplomacy_treaty":
				_map.diplomacy.open_panel("fac_england")
				_map.diplomacy.panel.stage_example()
			"diplomacy_counter":
				_map.diplomacy.open_panel("fac_aragon")
				_map.diplomacy.panel.stage_counter_example()
			_:
				_map.diplomacy.open_panel("fac_england")
		for size: Vector2i in SIZES:
			await _resize(size)
			var panel: Control = _map.diplomacy.panel
			var screen := Rect2(Vector2.ZERO, Vector2(size))
			_check(panel.is_visible_in_tree(), "%s: diplomacy panel not open at %s" % [stage, size])
			_check(_inside(panel.get_global_rect(), screen), "%s: diplomacy panel %s outside the screen %s" % [stage, panel.get_global_rect(), size])
			_check(panel.get_combined_minimum_size().x <= screen.size.x, "%s: diplomacy panel needs %s px at %s" % [stage, panel.get_combined_minimum_size().x, size])
			var panel_rect := panel.get_global_rect()
			for control: Control in panel.find_children("*", "Control", true, false):
				if not control.is_visible_in_tree() or control is ScrollContainer or control.size.x <= 0.0:
					continue
				if _clipped_by_scroll(control, panel):
					continue
				_check(control.get_global_rect().end.x <= panel_rect.end.x + 1.0, "%s: diplomacy %s '%s' sticks out at x=%s (panel ends %s) at %s" % [stage, control.get_class(), control.get("text"), control.get_global_rect().end.x, panel_rect.end.x, size])
		_map.diplomacy.panel.hide()
		await process_frame


## Vrai si `control` est dans un ScrollContainer (hors contrôle ancêtre `limit`) : son débordement est défilable.
func _clipped_by_scroll(control: Control, limit: Node) -> bool:
	var node := control.get_parent()
	while node != null and node != limit:
		if node is ScrollContainer:
			return true
		node = node.get_parent()
	return false


## 2. Tutoriel : sommaire ouvert, le parchemin reste dans l'écran.
func _tutorial() -> void:
	_map.tutorial.stage_screenshot("tutorial_toc")
	for size: Vector2i in SIZES:
		await _resize(size)
		var overlay: TutorialOverlay = _map.tutorial.overlay
		var panel: Control = overlay.panel
		var screen := Rect2(Vector2.ZERO, Vector2(size))
		_check(panel.is_visible_in_tree(), "tutorial panel not shown at %s" % size)
		_check(overlay.toc_open(), "tutorial toc not open at %s" % size)
		_check(_inside(panel.get_global_rect(), screen), "tutorial panel %s outside the screen %s" % [panel.get_global_rect(), size])
		_check(overlay.objective_label.get_global_rect().end.y <= screen.end.y, "tutorial objective cut at %s" % size)
		_check(overlay.skip_all_button.get_global_rect().end.y <= screen.end.y, "tutorial buttons cut at %s" % size)


## 3. Résultat de bataille : cartes des régiments entièrement dans la zone défilante à l'ouverture.
func _result_screen() -> void:
	var sides := {
		"attacker": {"name": "France", "faction": "fac_france", "color": Color(0.2, 0.3, 0.8)},
		"defender": {"name": "Angleterre", "faction": "fac_england", "color": Color(0.8, 0.2, 0.2)},
	}
	var units: Array = []
	var types := ["men_at_arms", "longbowmen", "spearmen", "knights", "crossbowmen", "militia"]
	var id := 0
	for side in ["attacker", "defender"]:
		for i in 14:
			var initial := 120 + i * 10
			var alive := 0 if i in [2, 5] else initial - 30
			units.append({"id": id, "side": side, "name": "Régiment %d" % i, "type": types[i % types.size()],
				"soldiers": alive, "initial_soldiers": initial, "max_soldiers": initial, "is_general": i == 0,
				"state": "routing" if i == 7 else "fighting", "kills": 10 * i, "present": true,
				"left_field": i == 7})
			id += 1
	var side_result := {"total_losses": 300, "general_captured": true, "no_quarter": true, "routed": true,
		"standards_taken": [{"unit_name": "Archers", "captor": "Lanciers"}, {"general": true}]}
	var outcome := {"winner": "attacker", "duration": 900.0, "end": "broken",
		"attacker": side_result.duplicate(), "defender": side_result.duplicate()}
	for size: Vector2i in [Vector2i(1920, 1080), Vector2i(1280, 720)]:
		_viewport.size = size
		var screen := BattleResultScreen.new()
		_viewport.add_child(screen)
		screen.show_result("Bataille d'essai", "attacker", sides, units, outcome, {})
		for i in 12:
			await process_frame
		var scrolls := screen.panel.find_children("*", "ScrollContainer", true, false)
		var scroll: Control = scrolls[0]
		var view := scroll.get_global_rect()
		_check(_inside(screen.panel.get_global_rect(), Rect2(Vector2.ZERO, Vector2(size))), "result panel outside the screen at %s" % size)
		_check(screen.return_button.get_global_rect().end.y <= size.y, "result return button cut at %s" % size)
		var cards := screen.panel.find_children("*", "RosterCard", true, false)
		_check(cards.size() == 28, "result screen shows %d cards instead of 28 at %s" % [cards.size(), size])
		if size.y >= 1080:
			for card: Control in cards:
				_check(_inside(card.get_global_rect(), view), "result card %s cut by the scroll view %s at %s" % [card.get_global_rect(), view, size])
		screen.queue_free()
		await process_frame
