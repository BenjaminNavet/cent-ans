extends SceneTree

## CB1 : formation au glisser et verrouillage de groupe.
## 1. Fonctions pures : `FormationDrag.drag_line` (point, orientation, largeur),
##    `split_widths` (prorata des effectifs), `lateral_order`, `lock_shape` + `rigid_places`
##    (translation et rotation rigides), `BattleGroups` (verrou, sortie d'une unité en déroute),
##    `BattleInput.locked_orders` (`move` individuels, `match_speed`, même `group_tag`).
## 2. Intégration sur la démo autonome (`battle.tscn`) : glisser-droit sur deux régiments =
##    fantômes à la taille d'arrivée pendant le glisser, un `move` avec `width` ; le cœur prend la
##    largeur (`line_files`) ; clic droit simple sans `width` ; Ctrl+G verrouille (cadenas sur les
##    cartes) et un clic droit envoie des `move` individuels du groupe.
##
## Usage : godot --headless --path game --script res://tests/cb1_drag_formation_test.gd

var _failures := 0
var _scene: Node = null


func _init() -> void:
	await process_frame
	_check_pure()
	await _check_integration()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	print("cb1_drag_formation_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("cb1_drag_formation_test: " + message)
	return condition


func _near(a: float, b: float, eps: float = 1e-3) -> bool:
	return absf(a - b) <= eps


func _check_pure() -> void:
	# Glisser de 100 m le long de x, caméra au sud : front vers le nord (+z), largeur 100 m.
	var line := FormationDrag.drag_line(Vector3(0, 0, 50), Vector3(100, 0, 50), Vector3(50, 80, -200))
	_check(bool(line["ok"]), "a 100 m drag is a line")
	_check(_near(float(line["width"]), 100.0), "drag width = its length (%s)" % line["width"])
	_check(_near(float(line["facing"]), 0.0), "front faces away from the camera (%s)" % line["facing"])
	_check((line["mid"] as Vector3).is_equal_approx(Vector3(50, 0, 50)), "order point at the middle")
	_check(not bool(FormationDrag.drag_line(Vector3.ZERO, Vector3(1, 0, 0), Vector3.ZERO)["ok"]), "a 1 m drag is a click")

	var shares := FormationDrag.split_widths([100, 300], 110.0, 10.0)
	_check(shares.size() == 2 and _near(shares[0], 25.0) and _near(shares[1], 75.0), "widths shared by strength (%s)" % [shares])
	_check(FormationDrag.split_widths([], 50.0, 5.0).is_empty(), "no regiment, no share")

	var order := FormationDrag.lateral_order([Vector2(30, 0), Vector2(-10, 5), Vector2(10, -3)], 0.0)
	_check(order == [1, 2, 0], "left-to-right order along the drag axis (%s)" % [order])

	var units := [
		{"id": 4, "x": 90.0, "z": 100.0, "facing": 0.0},
		{"id": 7, "x": 110.0, "z": 100.0, "facing": 0.0},
		{"id": 9, "x": 100.0, "z": 80.0, "facing": 0.2},
	]
	var shape := FormationDrag.lock_shape(units)
	# Translation seule : chaque place garde son décalage au centre (100, 93.33).
	var moved := FormationDrag.rigid_places(shape, [4, 7, 9], Vector2(200, 193.3333), NAN)
	_check(moved.size() == 3, "one place per member")
	if moved.size() == 3:
		var ref := float(shape["facing"])
		_check(absf(ref - 0.0667) < 0.01, "reference facing = mean (%s)" % ref)
		for k in 3:
			var dx := float(moved[k]["x"]) - float(units[k]["x"])
			var dz := float(moved[k]["z"]) - float(units[k]["z"])
			_check(_near(dx, 100.0, 0.01) and _near(dz, 100.0, 0.01), "rigid translation of %d (%s, %s)" % [k, dx, dz])
			_check(_near(float(moved[k]["facing"]), float(units[k]["facing"]), 1e-4), "own facing kept")
	# Rotation d'un quart de tour : distances entre membres conservées, orientations tournées.
	var turned := FormationDrag.rigid_places(shape, [4, 7, 9], Vector2(0, 0), float(shape["facing"]) + PI / 2)
	if turned.size() == 3:
		var d_before := Vector2(90, 100).distance_to(Vector2(110, 100))
		var d_after := Vector2(turned[0]["x"], turned[0]["z"]).distance_to(Vector2(turned[1]["x"], turned[1]["z"]))
		_check(_near(d_before, d_after, 1e-3), "rigid rotation keeps distances (%s vs %s)" % [d_before, d_after])
		_check(_near(angle_difference(float(turned[0]["facing"]), float(units[0]["facing"]) + PI / 2), 0.0, 1e-4), "facings turned with the group")

	var groups := BattleGroups.new()
	var roster := [
		{"id": 4, "x": 90.0, "z": 100.0, "facing": 0.0, "present": true, "state": "idle"},
		{"id": 7, "x": 110.0, "z": 100.0, "facing": 0.0, "present": true, "state": "idle"},
		{"id": 9, "x": 100.0, "z": 80.0, "facing": 0.0, "present": true, "state": "idle"},
	]
	var tag := groups.toggle_lock([4, 7, 9], roster)
	_check(tag > 0 and groups.is_locked(7) and groups.locked_group_for([4, 9]) == tag, "Ctrl+G locks the selection")
	var orders := BattleInput.locked_orders(groups.lock_places(tag, Vector2(300, 300), NAN), tag, false, true)
	_check(orders.size() == 3, "a locked group order = one move per regiment")
	for command in orders:
		_check((command["units"] as Array).size() == 1 and bool(command["match_speed"]) and int(command["group_tag"]) == tag and bool(command["queue"]) and command.has("facing") and not command.has("width"), "individual move, match_speed, shared tag: %s" % command)
	roster[2]["state"] = "routing"
	groups.prune_locks(roster)
	_check(not groups.is_locked(9) and groups.is_locked(4), "a routing regiment leaves the locked group")
	roster[1]["present"] = false
	groups.prune_locks(roster)
	_check(groups.locks.is_empty(), "a group of one comes undone")
	tag = groups.toggle_lock([4, 7], [roster[0], {"id": 7, "x": 0.0, "z": 0.0, "facing": 0.0, "present": true, "state": "idle"}])
	_check(tag > 0 and groups.toggle_lock([4, 7], roster) == 0 and groups.locks.is_empty(), "Ctrl+G again unlocks")


func _check_integration() -> void:
	root.size = Vector2i(1440, 900)  # headless : la fenêtre par défaut est minuscule (64x64)
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.autoplay = true
	_scene.log_orders_for_test = true
	root.add_child(_scene)
	for _i in 6:
		await process_frame
	if not _check(_scene.battle != null and (_scene.units as Array).size() > 0, "battle demo failed to stage (run core/build.sh?)"):
		return
	if not _check(_scene.battle.has_method("formation_extent"), "bridge without CB1 queries (run core/build.sh)"):
		return
	_scene.battle.call("set_ai", _scene.player_side, false)
	_scene.paused = true
	var own: Array[int] = []
	for unit in _scene.units:
		if bool(unit["present"]) and str(unit["side"]) == _scene.player_side and str(unit["category"]) == "infantry" and own.size() < 2:
			own.append(int(unit["id"]))
	if not _check(own.size() == 2, "two infantry regiments of the player"):
		return
	_scene.selected.assign(own)
	var a := _unit(own[0])
	var b := _unit(own[1])
	var center := Vector3((float(a["x"]) + float(b["x"])) * 0.5, 0.0, (float(a["z"]) + float(b["z"])) * 0.5)
	var ahead := Vector3(0, 0, 40.0 if _scene.player_side == "attacker" else -40.0)
	var p0 := center + ahead + Vector3(-110, 0, 0)
	var p1 := center + ahead + Vector3(110, 0, 0)
	_scene.camera_rig.look_at_point(center + ahead, 420.0, 0.0)
	for _i in 3:
		await process_frame
	var camera: Camera3D = _scene.camera_rig.camera
	var s0 := camera.unproject_position(_ground(p0))
	var s1 := camera.unproject_position(_ground(p1))

	# Glisser-droit : fantômes pendant le glisser, à la taille d'arrivée, puis un `move` avec `width`.
	_scene.issued_log.clear()
	_press_right(s0, true)
	_motion(s1)
	_scene.input._preview_right(s1, true)
	var sizes: Array[Vector2] = _scene.path_preview.live_ghost_sizes()
	_check(sizes.size() == 2, "two live ghosts during the drag (%s)" % [sizes])
	var wide_ghost := false
	for size in sizes:
		wide_ghost = wide_ghost or size.x > float(a["width"]) + 20.0
	_check(wide_ghost, "ghosts take the dragged width (%s vs %s)" % [sizes, a["width"]])
	_press_right(s1, false)
	var log: Array = _scene.issued_log
	_check(log.size() == 1, "one order for the drag (%s)" % [log])
	if log.size() == 1:
		_check(float(log[0].get("width", 0.0)) > 150.0, "the drag sends its width (%s)" % log[0])
		_check(not log[0].has("match_speed") and not log[0].has("group_tag"), "an unlocked drag is one group move")
	_scene._refresh_view(true)
	_check(int(_unit(own[0]).get("line_files", -1)) > 0, "the core takes the width (line_files %s)" % _unit(own[0]).get("line_files", -1))

	# Clic droit simple : pas de largeur.
	_scene.issued_log.clear()
	_press_right(s0, true)
	_press_right(s0, false)
	_scene.input._last_right_click_ms = -10000
	_check(_scene.issued_log.size() == 1 and not (_scene.issued_log[0] as Dictionary).has("width"), "a plain right click sends no width (%s)" % [_scene.issued_log])

	# Ctrl+G : groupe verrouillé, cadenas, puis ordres individuels du groupe.
	_scene.selected.assign(own)
	_ctrl_key(KEY_G)
	_check(_scene.hud.groups.locked_group_for(own) > 0, "Ctrl+G locks the selection")
	_scene.hud.update_cards(_scene.units, _scene.player_side, _scene.selected)
	var card: UnitCard = _scene.hud._cards.get(own[0])
	_check(card != null and card.locked, "padlock on the card of a locked regiment")
	_scene.issued_log.clear()
	_press_right(s1, true)
	_press_right(s1, false)
	_scene.input._last_right_click_ms = -10000
	log = _scene.issued_log
	_check(log.size() == 2, "a locked group sends one move per regiment (%s)" % [log])
	if log.size() == 2:
		var tag := int(log[0].get("group_tag", -1))
		_check(tag > 0 and int(log[1].get("group_tag", -1)) == tag and bool(log[0].get("match_speed", false)), "shared tag and match_speed (%s)" % [log])
	_scene._refresh_view(true)
	_check(bool(_unit(own[0]).get("match_speed", false)) and int(_unit(own[0]).get("group_tag", -1)) > 0, "the core walks the group at the pace of the slowest")
	_ctrl_key(KEY_G)
	_check(_scene.hud.groups.locked_group_for(own) == 0, "Ctrl+G again unlocks")


func _ground(p: Vector3) -> Vector3:
	return Vector3(p.x, _scene.terrain.height_at(p.x, p.z), p.z)


func _unit(id: int) -> Dictionary:
	for unit in _scene.battle.call("get_units"):
		if int(unit["id"]) == id:
			return unit
	return {}


func _press_right(pos: Vector2, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = MOUSE_BUTTON_RIGHT
	event.pressed = pressed
	event.position = pos
	event.global_position = pos
	_scene.get_viewport().push_input(event)


func _motion(pos: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = pos
	event.global_position = pos
	event.button_mask = MOUSE_BUTTON_MASK_RIGHT
	_scene.get_viewport().push_input(event)


func _ctrl_key(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.ctrl_pressed = true
		event.pressed = pressed
		_scene.get_viewport().push_input(event)
