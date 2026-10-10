extends TestCase

## BCTRL : contrôles de bataille (ordres depuis la minicarte, pivot sur place, signets de caméra,
## pause automatique sur alerte, unité suivante au repos, attaque au pas, sélection par classe).
## 1. Fonctions pures (`pivot_orders`, `next_idle`, `bookmark_slot`, `mods_of`, table, aide).
## 2. Intégration sur la démo autonome (`battle.tscn`).
##
## Usage : godot --headless --path game --script res://tests/bctrl_test.gd

var _scene: Node = null


func _init() -> void:
	await process_frame
	_check_pure()
	await _check_integration()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	finish()


func _key(code: Key, ctrl: bool = false, alt: bool = false, shift: bool = false) -> InputEventKey:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.ctrl_pressed = ctrl
	event.alt_pressed = alt
	event.shift_pressed = shift
	event.pressed = true
	return event


func _press(code: Key, ctrl: bool = false, alt: bool = false, shift: bool = false) -> void:
	for pressed in [true, false]:
		var event := _key(code, ctrl, alt, shift)
		event.pressed = pressed
		_scene.get_viewport().push_input(event)


func _check_pure() -> void:
	# Pivot : milieu du glisser près du centre = un move par régiment vers sa propre position.
	var units := [
		{"id": 1, "side": "a", "present": true, "state": "idle", "x": 100.0, "z": 50.0, "can_shoot": true, "render": "archer"},
		{"id": 2, "side": "a", "present": true, "state": "marching", "x": 110.0, "z": 50.0, "can_shoot": false, "render": "cavalry"},
		{"id": 3, "side": "a", "present": true, "state": "idle", "x": 130.0, "z": 50.0},
		{"id": 4, "side": "b", "present": true, "state": "idle", "x": 0.0, "z": 0.0},
	]
	var pivot := BattleInput.pivot_orders(units, [1], Vector3(101.0, 0, 51.0), 1.5, false)
	check(pivot.size() == 1 and pivot[0]["x"] == 100.0 and pivot[0]["z"] == 50.0 and pivot[0]["facing"] == 1.5 and not pivot[0].has("queue"), "pivot: move on the spot with facing (%s)" % [pivot])
	check(BattleInput.pivot_orders(units, [1], Vector3(110.0, 0, 50.0), 1.5, false).is_empty(), "pivot: a drag far from the unit is a normal line")
	check(BattleInput.pivot_orders(units, [1, 2], Vector3(105.0, 0, 50.0), 0.0, true).size() == 2, "pivot: two regiments, centred")
	check(bool(BattleInput.pivot_orders(units, [1], Vector3(100, 0, 50), 0.0, true)[0].get("queue", false)), "pivot: queued flag")
	# Unité suivante au repos : rotation circulaire.
	check(BattleInput.next_idle(units, "a", [], 1) == 1, "next idle from nothing = first idle")
	check(BattleInput.next_idle(units, "a", [1], 1) == 3, "next idle skips the marching regiment")
	check(BattleInput.next_idle(units, "a", [3], 1) == 1, "next idle wraps around")
	check(BattleInput.next_idle(units, "a", [1], -1) == 3, "previous idle wraps around")
	check(BattleInput.next_idle(units, "c", [], 1) == -1, "no idle unit = -1")
	# Signets et modificateurs.
	check(BattleHotkeys.bookmark_slot(_key(KEY_F3, true), true) == 3 and BattleHotkeys.bookmark_slot(_key(KEY_F3), true) == 0, "Ctrl+F3 saves slot 3")
	check(BattleHotkeys.bookmark_slot(_key(KEY_F4), false) == 4 and BattleHotkeys.bookmark_slot(_key(KEY_F1), false) == 0, "F4 recalls, F1 stays help")
	check(BattleHotkeys.mods_of(_key(KEY_A, true, false, true)) == "ctrl_shift", "Ctrl+Shift mods")
	check(BattleHotkeys.action_for(_key(KEY_A, true, false, true)) == "select_shooters" and BattleHotkeys.action_for(_key(KEY_C, true, false, true)) == "select_cavalry", "class selection keys")
	check(BattleHotkeys.action_for(_key(KEY_PERIOD)) == "next_idle", "period = next idle")
	var seen := {}
	for row in BattleHotkeys.BINDINGS:
		if not row.has("key"):
			continue
		var slot := "%d/%s/%s" % [int(row["key"]), str(row.get("mods", "")), str(row.get("physical", false))]
		check(not seen.has(slot), "key taken twice: %s" % slot)
		seen[slot] = true
	var help := BattleHotkeys.help_bbcode()
	for needle in ["Ctrl+F2…F4", "Alt + clic droit", "clic droit minicarte", "pivoter sur place", "Ctrl+Maj+A", "unité suivante au repos"]:
		check(help.contains(needle), "help mentions %s" % needle)
	check(Settings.DEFAULTS.get("battle/auto_pause_on_alert", true) == false, "auto pause off by default")


func _check_integration() -> void:
	root.size = Vector2i(1440, 900)
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.autoplay = true
	_scene.log_orders_for_test = true
	root.add_child(_scene)
	for _i in 6:
		await process_frame
	if not check(_scene.battle != null and (_scene.units as Array).size() > 0, "battle demo failed to stage"):
		return
	_scene.battle.call("set_ai", _scene.player_side, false)
	_scene.paused = true
	var mine: Array[int] = []
	var shooter := -1
	var foot := -1
	for unit in _scene.battle.call("get_units"):
		if not bool(unit["present"]) or str(unit["side"]) != _scene.player_side:
			continue
		mine.append(int(unit["id"]))
		if shooter < 0 and bool(unit["can_shoot"]) and str(unit["category"]) != "siege":
			shooter = int(unit["id"])
		elif foot < 0 and str(unit["category"]) == "infantry":
			foot = int(unit["id"])
	if not check(foot >= 0, "a foot regiment of the player"):
		return

	# Clic droit minicarte = move de la sélection (Maj : en file).
	_scene.selected.assign([foot])
	_scene.issued_log.clear()
	_scene.hud.minimap.order_requested.emit(Vector2(600, 300), false)
	var log: Array = _scene.issued_log
	check(log.size() == 1 and str(log[0].get("type", "")) == "move" and log[0]["units"] == [foot] and is_equal_approx(float(log[0]["x"]), 600.0) and is_equal_approx(float(log[0]["z"]), 300.0), "minimap order sends move (%s)" % [log])
	_scene.issued_log.clear()
	_scene.hud.minimap.order_requested.emit(Vector2(610, 310), true)
	check(_scene.issued_log.is_empty() or bool(_scene.issued_log[0].get("queue", false)), "queued minimap order carries queue")
	_scene.selected.clear()
	_scene.issued_log.clear()
	_scene.hud.minimap.order_requested.emit(Vector2(600, 300), false)
	check(_scene.issued_log.is_empty(), "no order without selection")

	# Signets de caméra.
	var rig = _scene.camera_rig
	rig.look_at_point(Vector3(300, 0, 200), 150.0, 0.7)
	_press(KEY_F2, true)
	rig.look_at_point(Vector3(900, 0, 600), 220.0, 0.0)
	_press(KEY_F2)
	rig.snap()
	check(Vector2(rig.target.x, rig.target.z).distance_to(Vector2(300, 200)) < 0.5 and absf(rig.yaw - 0.7) < 0.01, "F2 recalls the saved camera (%s)" % [rig.target])

	# Sélection par classe et unité au repos.
	_press(KEY_A, true, false, true)
	check(not _scene.selected.is_empty() or shooter < 0, "Ctrl+Shift+A selects shooters")
	for id in _scene.selected:
		check(bool(_unit(int(id))["can_shoot"]), "selected unit %d can shoot" % id)
	_scene.selected.clear()
	_press(KEY_PERIOD)
	check(_scene.selected.size() <= 1, "next idle selects at most one unit")

	# Attaque au pas : Alt au relâchement du clic droit -> run:false (appel direct, sans picking).
	var enemy := -1
	for unit in _scene.battle.call("get_units"):
		if str(unit["side"]) == _scene.enemy_side and bool(unit["present"]):
			enemy = int(unit["id"])
			break
	_scene.selected.assign([foot])
	_scene.issued_log.clear()
	_scene.input.command_requested.emit({"type": "attack", "units": [foot], "target": enemy, "run": not true})
	check(_scene.issued_log.size() == 1 and not bool(_scene.issued_log[0]["run"]), "attack with run:false is accepted by the core")

	# Pause automatique : réglage off = rien ; on = pause à la déroute du joueur.
	_scene.paused = false
	var alert := [{"kind": "rout", "time": 1.0, "x": 10.0, "z": 10.0, "side": _scene.player_side, "unit": foot}]
	_scene._auto_pause_on_alerts(alert)
	check(not _scene.paused, "auto pause disabled by default")
	root.get_node("/root/Settings").call("set_value", "battle/auto_pause_on_alert", true, false)
	_scene._auto_pause_on_alerts([{"kind": "rout", "time": 1.0, "x": 10.0, "z": 10.0, "side": _scene.enemy_side, "unit": 99}])
	check(not _scene.paused, "enemy rout does not pause")
	_scene._auto_pause_on_alerts(alert)
	check(_scene.paused, "player rout pauses when enabled")
	_scene.paused = false
	_scene.replay_mode = true
	_scene._auto_pause_on_alerts(alert)
	check(not _scene.paused, "never during a replay")
	_scene.replay_mode = false


func _unit(id: int) -> Dictionary:
	for unit in _scene.battle.call("get_units"):
		if int(unit["id"]) == id:
			return unit
	return {}
