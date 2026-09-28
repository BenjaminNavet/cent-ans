extends SceneTree

## Test headless Q6 (recette du 2026-09-28) : boutons du panneau latéral (`SIDE_PANEL`) atteignables
## à la souris. Configuration du joueur : fenêtre 1920×1080, taille d'interface 1,25 (vue 1280×720) ;
## aussi 1280×720 en taille 1,0 et 1280×640. Pour chaque configuration :
##  1. panneau de province de la capitale (`flow.focus_province`), section des édits : « Changer
##     d'édit » ;
##  2. panneau de colonie de la capitale (`settlements_ctl.open_settlement`) : « Recruter ».
## Le bouton est amené dans la partie visible de son défilement, puis il doit être entièrement
## dans la vue et dans la zone `SIDE_PANEL`, et le contrôle sous son centre (mouvement de souris
## injecté) doit être le bouton lui-même, pas la minicarte ni le bouton de fin de tour.
## Usage : godot --headless --path game --script res://tests/q6_side_panel_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
## (taille de fenêtre, taille d'interface)
const CONFIGS := [
	[Vector2i(1920, 1080), 1.25],
	[Vector2i(1280, 720), 1.0],
	[Vector2i(1280, 640), 1.0],
]

var _failures := 0
var map: Node3D


func _init() -> void:
	await process_frame
	await _run()
	print("q6_side_panel_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("q6_side_panel_test: " + message)
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
		await _test_edict_button(label)
		await _test_recruit_button(label)
	map.queue_free()
	await process_frame


func _capital() -> String:
	return str(root.get_node("/root/SimFacade").faction_info(map.player_faction).get("capital", ""))


func _test_edict_button(label: String) -> void:
	map.flow.focus_province(_capital())
	await _wait(20)
	var panel: Control = map.ui.province_panel
	if not _check(panel.is_visible_in_tree(), "%s: province panel not open" % label):
		return
	var section: Control = panel.get("edict_section")
	if not _check(section != null, "%s: no edict section" % label):
		return
	for tab in panel.tabs.get_tab_count():
		if panel.tabs.get_tab_control(tab).is_ancestor_of(section):
			panel.tabs.current_tab = tab
	await _wait(10)
	var choose: Button = section.get("choose_button")
	if _check(choose != null and choose.is_visible_in_tree(), "%s: « Changer d'édit » not visible" % label):
		await _check_reachable(choose, "%s, edicts" % label)
	panel.hide()
	await _wait(6)


func _test_recruit_button(label: String) -> void:
	var state: Dictionary = map.sim.call("get_province_state", _capital())
	map.settlements_ctl.open_settlement(str(state.get("city", "")), false)
	await _wait(20)
	var panel: Control = map.settlements_ctl.panel
	if not _check(panel != null and panel.is_visible_in_tree(), "%s: settlement panel not open" % label):
		return
	var recruit: Button = panel.get("recruit_button")
	if _check(recruit != null and recruit.is_visible_in_tree(), "%s: « Recruter » not visible" % label):
		await _check_reachable(recruit, "%s, recruit" % label)
	panel.hide()
	await _wait(6)


## Amène `button` dans son défilement, puis vérifie qu'il est dans la vue et la zone latérale et
## que la souris posée sur son centre le survole.
func _check_reachable(button: Button, label: String) -> void:
	var node := button.get_parent()
	while node != null:
		if node is ScrollContainer:
			(node as ScrollContainer).ensure_control_visible(button)
		node = node.get_parent()
	await _wait(4)
	var rect := button.get_global_rect()
	var view := root.get_visible_rect()
	var side := UiZones.rect(UiZones.Zone.SIDE_PANEL).grow(0.5)
	_check(view.grow(0.5).encloses(rect), "%s: « %s » %s outside the view %s" % [label, button.text, rect, view.size])
	_check(side.encloses(rect), "%s: « %s » %s outside the side panel zone %s" % [label, button.text, rect, side])
	var point := root.get_final_transform() * rect.get_center()
	var motion := InputEventMouseMotion.new()
	motion.position = point
	motion.global_position = point
	Input.parse_input_event(motion)
	Input.flush_buffered_events()
	await _wait(2)
	var hovered := root.gui_get_hovered_control()
	var ok := hovered != null and (hovered == button or button.is_ancestor_of(hovered))
	_check(ok, "%s: « %s » center %s covered by %s" % [label, button.text, rect.get_center(), hovered.get_path() if hovered != null else "nothing"])
