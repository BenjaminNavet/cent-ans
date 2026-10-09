extends TestCase

## CB-M3 : ordres en file (Maj + clic droit).
## 1. La borne vient des règles (`battle_queue_max` = 8 par RuleValues, `data/rules/battle_queue.json`).
## 2. Intégration sur la démo autonome (`battle.tscn`) : Maj + clic droit envoie `queue: true` et
##    le cœur garde l'ordre en file (`get_units()[i].queue`) ; points de passage numérotés au sol ;
##    aperçu en direct avec Maj depuis le dernier point de la file ; clic droit simple : pas de
##    `queue`, la file est vidée ; file pleine : rien d'envoyé, curseur `forbidden` et infobulle
##    tant que Maj est tenue.
##
## Usage : godot --headless --path game --script res://tests/cb_m3_queue_test.gd

var _scene: Node = null


func _init() -> void:
	await process_frame
	check(int(RuleValues.value("battle_queue_max", -1.0)) == 8, "battle_queue_max from RuleValues (got %s)" % RuleValues.value("battle_queue_max", -1.0))
	await _check_integration()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	finish()


func _check_integration() -> void:
	root.size = Vector2i(1440, 900)  # headless : la fenêtre par défaut est minuscule (64x64)
	# Les événements poussés sont en pixels de fenêtre : le test ne dépend pas de la « Taille de l'interface » du joueur.
	(root.get_node("Settings")).call("use_test_file")
	root.content_scale_factor = 1.0
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.autoplay = true
	_scene.log_orders_for_test = true
	root.add_child(_scene)
	for _i in 6:
		await process_frame
	if not check(_scene.battle != null and (_scene.units as Array).size() > 0, "battle demo failed to stage (run core/build.sh?)"):
		return
	if not check(_scene.battle.has_method("preview_path_from") and _scene.battle.has_method("preview_paths_queued"), "bridge without CB-M3 queries (run core/build.sh)"):
		return
	_scene.battle.call("set_ai", _scene.player_side, false)
	_scene.paused = true
	var preview: BattlePathPreview = _scene.path_preview
	if not check(preview != null, "no path preview"):
		return
	var own := -1
	for unit in _scene.units:
		if bool(unit["present"]) and str(unit["side"]) == _scene.player_side:
			own = int(unit["id"])
			break
	if not check(own >= 0, "no present regiment"):
		return
	_scene.selected.assign([own])
	var unit := _unit(own)
	var start := Vector3(float(unit["x"]), 0.0, float(unit["z"]))
	var goals: Array[Vector3] = [start + Vector3(40, 0, 30), start + Vector3(-30, 0, 60), start + Vector3(20, 0, 90)]
	_scene.camera_rig.look_at_point(goals[1], 320.0, 0.0)
	for _i in 3:
		await process_frame
	var camera: Camera3D = _scene.camera_rig.camera

	# Clic droit simple, puis deux Maj + clic droit : un ordre et deux ordres en file.
	_scene.issued_log.clear()
	for k in goals.size():
		var screen := camera.unproject_position(_ground(goals[k]))
		_right_click(screen, k > 0)
	var log: Array = _scene.issued_log
	check(log.size() == 3, "three orders sent: %s" % [log])
	if log.size() == 3:
		check(not log[0].has("queue"), "a plain right click sends no queue flag")
		check(bool(log[1].get("queue", false)) and bool(log[2].get("queue", false)), "Shift + right click sends queue: true")
	_scene._refresh_view(true)
	var queue: Array = _unit(own).get("queue", [])
	check(queue.size() == 2, "the core keeps two queued orders (got %s)" % [queue])
	if queue.size() == 2:
		var last: Dictionary = queue[1]
		check(Vector2(float(last["x"]), float(last["z"])).distance_to(Vector2(goals[2].x, goals[2].z)) < 3.0, "last queued point where clicked")

	# Points de passage numérotés au sol, trajet segment par segment.
	preview._orders_key = ""
	preview.update_orders(_scene.units, _scene.selected, 1000.0)
	check(preview.orders_mesh().visible, "queued path drawn")
	check(preview.waypoint_numbers() == ["1", "2", "3"], "waypoints numbered 1-3 (got %s)" % [preview.waypoint_numbers()])
	check(preview.ghost_count() >= 1, "ghost at the end of the queue")

	# Aperçu en direct avec Maj : il part du dernier point de la file.
	var aim := goals[2] + Vector3(50, 0, 0)
	var legs: Array = preview.compute([own], _scene.units, aim, NAN, 2000.0, true)
	if check(legs.size() == 1 and bool(legs[0]["ok"]), "queued live preview computed: %s" % [legs]):
		var path: PackedVector3Array = legs[0]["path"]
		var from := Vector2(path[0].x, path[0].z)
		var anchor: Dictionary = queue[1] if queue.size() == 2 else {}
		check(not anchor.is_empty() and from.distance_to(Vector2(float(anchor["x"]), float(anchor["z"]))) < 0.5, "Shift preview starts from the last queued point (%s)" % from)
	check(preview.waypoint_numbers().is_empty(), "queue hidden during the live preview")
	var plain: Array = preview.compute([own], _scene.units, aim, NAN, 2001.0, false)
	if plain.size() == 1 and bool(plain[0]["ok"]):
		var here: Vector3 = (plain[0]["path"] as PackedVector3Array)[0]
		var pos := _unit(own)
		check(Vector2(here.x, here.z).distance_to(Vector2(float(pos["x"]), float(pos["z"]))) < 0.5, "plain preview starts from the regiment")
	preview.clear_live()

	# File pleine : Maj + clic droit ne part pas ; curseur interdit et infobulle avec Maj tenue.
	var limit := int(RuleValues.value("battle_queue_max", 8.0))
	for k in limit:
		_scene.issue({"type": "move", "units": [own], "x": start.x + 5.0 * k, "z": start.z + 100.0, "run": false, "queue": true})
	_scene._refresh_view(true)
	check((_unit(own).get("queue", []) as Array).size() == limit, "queue filled to %d" % limit)
	check(BattlePathPreview.queue_full(_scene.units, _scene.selected), "queue_full sees the full queue")
	var refused: Dictionary = _scene.battle.call("issue_command", {"type": "move", "units": [own], "x": start.x, "z": start.z + 120.0, "queue": true})
	check(not bool(refused.get("ok", true)) and str(refused.get("error", "")).contains("file d'ordres pleine"), "core refuses a ninth queued order in French: %s" % [refused])
	_scene.issued_log.clear()
	var spot := camera.unproject_position(_ground(goals[0]))
	_right_click(spot, true)
	check(_scene.issued_log.is_empty(), "no order sent with a full queue: %s" % [_scene.issued_log])
	_shift(true)
	_scene.markers.world_hover = -1
	_scene.note_mouse(spot)
	_scene._update_hover_cursor()
	check(_scene.cursor.context == "forbidden", "forbidden cursor with Shift and a full queue (got %s)" % _scene.cursor.context)
	check(_scene.queue_tip != null and _scene.queue_tip.text().contains("File d'ordres pleine"), "French tooltip shown")
	_shift(false)
	_scene.note_mouse(spot + Vector2(1, 0))
	_scene._update_hover_cursor()
	check(_scene.cursor.context != "forbidden", "Shift released: cursor back (got %s)" % _scene.cursor.context)
	check(_scene.queue_tip == null or _scene.queue_tip.text() == "", "tooltip hidden without Shift")

	# Clic droit simple : ordre accepté, la file est vidée.
	_scene.issued_log.clear()
	_right_click(spot, false)
	_scene._refresh_view(true)
	check(_scene.issued_log.size() == 1, "plain order sent with a full queue")
	check((_unit(own).get("queue", []) as Array).is_empty(), "a plain order empties the queue")


func _ground(p: Vector3) -> Vector3:
	return Vector3(p.x, _scene.terrain.height_at(p.x, p.z), p.z)


func _unit(id: int) -> Dictionary:
	for unit in _scene.battle.call("get_units"):
		if int(unit["id"]) == id:
			return unit
	return {}


func _right_click(pos: Vector2, shift: bool) -> void:
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_RIGHT
		event.pressed = pressed
		event.shift_pressed = shift
		event.position = pos
		event.global_position = pos
		_scene.get_viewport().push_input(event)
	# Pas de double clic droit (350 ms) entre deux ordres du test.
	_scene.input._last_right_click_ms = -10000


func _shift(pressed: bool) -> void:
	var event := InputEventKey.new()
	event.keycode = KEY_SHIFT
	event.physical_keycode = KEY_SHIFT
	event.pressed = pressed
	Input.parse_input_event(event)
	Input.flush_buffered_events()
