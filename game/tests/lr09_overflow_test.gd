extends SceneTree

## Test headless LR-09 : non-débordement à 1280×720 (taille d'interface 1,0 et 1,25) :
##  1. en-tête du panneau de province : au plus 40 % de la hauteur du panneau, onglets >= 200 px ;
##  2. trois avis très longs : la pile reste dans la vue et dans la zone `TOASTS` ;
##  3. panneau des technologies : entièrement dans la vue.
## Usage : godot --headless --path game --script res://tests/lr09_overflow_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const LONG := "Bohême propose un traité. Traité entre la Bohême et la France : paix, commerce et mariage de la princesse avec votre héritier, sous réserve de l'accord des États généraux réunis à Paris pour la circonstance."
const CONFIGS := [[Vector2i(1280, 720), 1.0], [Vector2i(1920, 1080), 1.25], [Vector2i(1280, 720), 1.25]]

var _failures := 0
var map: Node3D


func _init() -> void:
	await process_frame
	await _run()
	print("lr09_overflow_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("lr09_overflow_test: " + message)
	return condition


func _wait(frames: int) -> void:
	for _i in frames:
		await process_frame


func _run() -> void:
	var settings: Node = root.get_node("/root/Settings")
	settings.call("use_test_file")
	settings.call("set_value", "tutorial/enabled", false, false)
	settings.call("set_value", "game/autosave_interval", 0, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	root.size = Vector2i(1920, 1080)
	settings.call("set_value", "interface/ui_size", 1.25, false)
	map = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _wait(10)
	for config in CONFIGS:
		root.size = config[0]
		settings.call("set_value", "interface/ui_size", config[1], false)
		await _wait(6)
		var label := "%s ui %.2f (view %s)" % [config[0], config[1], root.get_visible_rect().size]
		await _test_province_header(label)
		await _test_toasts(label)
		await _test_tech(label)
	map.queue_free()
	await process_frame


func _capital() -> String:
	return str(root.get_node("/root/SimFacade").faction_info(map.player_faction).get("capital", ""))


func _test_province_header(label: String) -> void:
	map.flow.focus_province(_capital())
	await _wait(20)
	var panel: Control = map.ui.province_panel
	if not _check(panel.is_visible_in_tree(), "%s: province panel not open" % label):
		return
	var tabs: TabContainer = panel.tabs
	var header_height := tabs.global_position.y - panel.global_position.y
	print("lr09 header %s: header %.0f px, tabs %.0f px, panel %.0f px" % [label, header_height, tabs.size.y, panel.size.y])
	for child in panel.get_node("VBox").get_children():
		if child is Control and child.visible:
			print("lr09   %s: %.0f px" % [child.name, child.size.y])
	var gauges: Control = panel.get("gauges_row")
	print("lr09   gauges row width %.0f, chips %s" % [gauges.size.x, gauges.get_children().map(func(c): return "%.0f(%s)" % [c.size.x, c.get_child(1).text])])
	_check(header_height <= 0.42 * panel.size.y, "%s: province header %.0f px of %.0f px" % [label, header_height, panel.size.y])
	_check(tabs.size.y >= 200.0, "%s: tabs only %.0f px high" % [label, tabs.size.y])
	panel.hide()
	await _wait(6)


func _test_toasts(label: String) -> void:
	var layout: Node = UiZones.layout()
	layout.clear_toasts()
	for i in 3:
		layout.toast("%d. %s" % [i, LONG], "", 0.0)
	await _wait(12)
	var zone := UiZones.rect(UiZones.Zone.TOASTS)
	var view := root.get_visible_rect()
	for entry in layout.toasts():
		var rect: Rect2 = entry.get_global_rect()
		print("lr09 toast %s: %s zone %s" % [label, rect, zone])
		_check(view.grow(0.5).encloses(rect), "%s: toast %s outside the view %s" % [label, rect, view.size])
		_check(rect.end.x <= zone.end.x + 1.0, "%s: toast %s wider than zone %s" % [label, rect, zone])
		_check(str(entry.tooltip_text).contains("réunis à Paris"), "%s: long toast without full-text tooltip" % label)
	layout.clear_toasts()
	await _wait(4)


func _test_tech(label: String) -> void:
	map.ui.tech_panel_requested.emit()
	await _wait(25)
	var panel: Control = map.ui.tech_panel
	if not _check(panel.is_visible_in_tree(), "%s: tech panel not open" % label):
		return
	var view := root.get_visible_rect()
	for tab in 3:
		panel.tabs.current_tab = tab
		await _wait(6)
		var rect := panel.get_global_rect()
		print("lr09 tech %s tab %d: %s view %s min %s" % [label, tab, rect, view.size, panel.get_combined_minimum_size()])
		_check(view.grow(0.5).encloses(rect), "%s: tech panel tab %d %s outside the view %s" % [label, tab, rect, view.size])
	panel.hide()
	await _wait(6)
