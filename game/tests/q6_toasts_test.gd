extends TestCase

## Test headless Q6 (recette du 2026-09-28) : fenêtres ouvertes par le joueur et zone des avis
## (`TOASTS`, haut gauche), vérifié par l'état des nœuds et de vrais clics (`push_input`).
##  1. fenêtre 1920×1080, taille d'interface 1,25 (vue 1280×720) : le registre des agents (G)
##     passe au-dessus des avis et du journal ; un clic sur la ligne d'un agent recruté le
##     sélectionne (il tombait sur `UiZone_TOASTS/Stack/EventLog`) ;
##  2. règle générale : toute fenêtre du joueur posée en haut à gauche (registre, « Colonies »)
##     est dessinée et cliquable au-dessus de la zone `TOASTS` ;
##  3. vue étroite (fenêtre 1280×720, taille 1,25 → vue 1137×640) : avis et journal tiennent
##     dans la largeur de la zone (le texte se replie au lieu d'être coupé au bord).
## Usage : godot --headless --path game --script res://tests/q6_toasts_test.gd

const LONG_TOAST := "Bohême propose un traité. Traité entre la Bohême et la France : paix, commerce et mariage de la princesse avec votre héritier."


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	settings.call("use_test_file")
	settings.call("set_value", "tutorial/enabled", false, false)
	settings.call("set_value", "game/autosave_interval", 0, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	root.size = Vector2i(1920, 1080)
	settings.call("set_value", "interface/ui_size", 1.25, false)
	var map := await _load_map()
	await _test_registry_click(map)
	await _test_top_left_windows(map)
	await _test_narrow_toasts(map)
	settings.call("set_value", "interface/ui_size", 1.0, false)
	map.queue_free()
	await process_frame


func _load_map() -> Node3D:
	var facade: Node = root.get_node("/root/SimFacade")
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for _i in 10:
		await process_frame
	return map


func _frames(count: int = 4) -> void:
	for _i in count:
		await process_frame


## Position fenêtre d'un point du canevas (échelle de l'interface comprise).
func _to_window(point: Vector2) -> Vector2:
	return root.get_final_transform() * point


func _click(point: Vector2) -> void:
	var at := _to_window(point)
	var motion := InputEventMouseMotion.new()
	motion.position = at
	motion.global_position = at
	root.push_input(motion)
	for pressed in [true, false]:
		var button := InputEventMouseButton.new()
		button.button_index = MOUSE_BUTTON_LEFT
		button.pressed = pressed
		button.position = at
		button.global_position = at
		root.push_input(button)
	await _frames(2)


## Contrôle sous la souris au point `point` (coordonnées du canevas).
func _hovered_at(point: Vector2) -> Control:
	var motion := InputEventMouseMotion.new()
	motion.position = _to_window(point)
	motion.global_position = motion.position
	root.push_input(motion)
	await process_frame
	return root.gui_get_hovered_control()


func _fill_toasts(map: Node3D) -> void:
	for i in 3:
		map.ui.show_toast("%d événements féodaux vous concernent : hommages, successions et querelles de vassaux. %s" % [37 + i, LONG_TOAST])
	var events: Array = []
	for i in 6:
		events.append({"kind": "treaty_proposed", "faction": map.player_faction, "text": LONG_TOAST})
	map.ui.add_events(events, "Printemps 1337")
	map.ui.set_log_expanded(true)
	await _frames()


func _test_registry_click(map: Node3D) -> void:
	var ctl: Node = map.agents_ctl
	var result: Dictionary = ctl.recruit(ctl.recruit_place(), "spy")
	check(not result.is_empty(), "recruit returned nothing")
	map.sim.call("end_turn")
	map.refresh_all()
	await _fill_toasts(map)
	ctl.toggle_registry()
	await _frames(6)
	var row: Button = null
	for child in ctl._registry_list.get_children():
		if child is Button and not child.is_queued_for_deletion():
			row = child
			break
	if not check(row != null, "no agent row in the registry"):
		return
	var center := row.get_global_rect().get_center()
	var hovered := await _hovered_at(center)
	check(hovered != null and (hovered == row or row.is_ancestor_of(hovered)), "registry row covered by %s" % (hovered.get_path() if hovered != null else "nothing"))
	ctl.selected_agent = ""
	await _click(center)
	check(ctl.selected_agent != "", "click on the agent row did not select the agent")
	ctl.registry.hide()
	await _frames()


func _test_top_left_windows(map: Node3D) -> void:
	var zone: Control = map.ui.get_node("UiZone_TOASTS")
	var windows: Array = []
	map.agents_ctl.toggle_registry()
	windows.append(map.agents_ctl.registry)
	await _frames(6)
	await _check_above(map, zone, map.agents_ctl.registry)
	map.agents_ctl.registry.hide()
	map.holdings_ctl.toggle()
	await _frames(6)
	await _check_above(map, zone, map.holdings_ctl.panel)
	map.holdings_ctl.toggle()
	await _frames()


## `window` est dessinée après la zone des avis et reçoit la souris là où elle la recouvre.
func _check_above(map: Node3D, zone: Control, window: Control) -> void:
	if not check(window != null and window.visible, "%s not open" % (window.name if window != null else "window")):
		return
	var window_index := window.get_index() if window.get_parent() == map.ui else -1
	if window.get_parent() == map.ui:
		check(window_index > zone.get_index(), "%s (index %d) is under the TOASTS zone (index %d)" % [window.name, window_index, zone.get_index()])
	var overlap := window.get_global_rect().intersection(zone.get_global_rect())
	if overlap.has_area():
		var hovered := await _hovered_at(overlap.get_center())
		check(hovered != null and (hovered == window or window.is_ancestor_of(hovered)), "%s covered by %s" % [window.name, hovered.get_path() if hovered != null else "nothing"])


func _test_narrow_toasts(map: Node3D) -> void:
	root.size = Vector2i(1280, 720)
	await _frames(6)
	await _fill_toasts(map)
	await _frames(4)
	var zone: Control = map.ui.get_node("UiZone_TOASTS")
	var zone_rect := zone.get_global_rect()
	check(zone_rect.size.x < 320.0, "narrow view expected, TOASTS zone is %s" % zone_rect)
	var log: Control = map.ui.event_log
	check(log.get_global_rect().end.x <= zone_rect.end.x + 0.5, "event log %s wider than the TOASTS zone %s" % [log.get_global_rect(), zone_rect])
	var log_text: Control = map.ui.log_text
	check(log_text.get_global_rect().end.x <= zone_rect.end.x + 0.5, "log text %s cut by the TOASTS zone %s" % [log_text.get_global_rect(), zone_rect])
	var toasts: Array = UiZones.layout().toasts()
	check(not toasts.is_empty(), "no toast shown")
	for entry: Control in toasts:
		if not entry.visible:
			continue
		var label := entry.find_child("Text", true, false) as Label
		var rect := label.get_global_rect()
		check(rect.end.x <= zone_rect.end.x + 0.5, "toast text %s cut by the TOASTS zone %s" % [rect, zone_rect])
		check(label.get_line_count() > 1, "long toast should wrap on several lines")
	root.size = Vector2i(1920, 1080)
	await _frames()
