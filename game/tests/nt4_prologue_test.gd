extends SceneTree

## NT4 : bataille-prologue guidée — chargement des étapes, évaluation pure de chaque condition,
## entrée du menu, puis bataille réelle : ennemi passif, passage de chaque étape sur l'état de la
## bataille (caméra, sélection, marche, front, charge, tir, pause) et fin sur la victoire.
## Usage : godot --headless --path game --script res://tests/nt4_prologue_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

const EXPECTED := ["intro", "camera", "select", "move", "formation", "charge", "fire", "pause", "victory", "outro"]

var _failures := 0


func _init() -> void:
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
	_check_data()
	_check_evaluate()
	if not ClassDB.class_exists("BattleSim"):
		print("nt4: extension absente, partie bataille ignorée")
	else:
		await _check_menu_entry()
		await _check_battle()
	if _failures == 0:
		print("nt4 battle prologue OK")
	quit(1 if _failures > 0 else 0)


func _check_data() -> void:
	var data := BattlePrologue.load_data()
	_check(not data.is_empty(), "prologue data loaded from %s" % BattlePrologue.data_path())
	var ids := []
	for step in data.get("steps", []):
		ids.append(str(step["id"]))
	_check(ids == EXPECTED, "steps in order: %s" % str(ids))
	_check(str((data.get("battle", {}) as Dictionary).get("player_side", "")) == "defender", "player leads the English")
	var prologue := BattlePrologue.new()
	prologue.setup(data)
	_check(prologue.index_of("victory") == 8, "victory step index")
	prologue.free()


## Chaque condition, fausse puis vraie, sur des états construits à la main.
func _check_evaluate() -> void:
	var unit := {"id": 1, "side": "defender", "present": true, "category": "cavalry", "state": "idle", "formation": "line", "width": 40.0, "x": 100.0, "z": 100.0}
	var enemy := {"id": 2, "side": "attacker", "present": true, "category": "infantry", "state": "idle", "formation": "line", "width": 40.0, "x": 100.0, "z": 300.0}
	var camera := {"target": Vector3(600, 0, 200), "distance": 220.0, "yaw": 0.0}
	var state := {"player_side": "defender", "selected": [], "units": [unit, enemy], "paused": false, "camera": camera, "finished": false, "winner": ""}
	var base := BattlePrologue.baseline_of(state)
	var ev := func(condition: Dictionary, changes: Dictionary) -> bool:
		var s := state.duplicate(true)
		for key in changes:
			s[key] = changes[key]
		return BattlePrologue.evaluate(condition, s, base)
	_check(not ev.call({"type": "manual"}, {}), "manual never passes by itself")
	_check(not ev.call({"type": "camera_moved", "min_distance": 40.0}, {}), "camera still")
	_check(ev.call({"type": "camera_moved", "min_distance": 40.0}, {"camera": {"target": Vector3(660, 0, 200), "distance": 220.0, "yaw": 0.0}}), "camera panned")
	_check(ev.call({"type": "camera_moved"}, {"camera": {"target": Vector3(600, 0, 200), "distance": 150.0, "yaw": 0.0}}), "camera zoomed")
	_check(ev.call({"type": "camera_moved"}, {"camera": {"target": Vector3(600, 0, 200), "distance": 220.0, "yaw": 0.6}}), "camera turned")
	_check(not ev.call({"type": "selection", "min_count": 1}, {"selected": [2]}), "enemy selection does not count")
	_check(ev.call({"type": "selection", "min_count": 1}, {"selected": [1]}), "own regiment selected")
	var moved := unit.duplicate()
	moved["x"] = 130.0
	_check(not ev.call({"type": "moved", "min_distance": 25.0}, {}), "nobody moved")
	_check(ev.call({"type": "moved", "min_distance": 25.0}, {"units": [moved, enemy]}), "regiment arrived 30 m away")
	var wide := unit.duplicate()
	wide["width"] = 60.0
	var column := unit.duplicate()
	column["formation"] = "column"
	_check(not ev.call({"type": "formation_changed", "min_ratio": 0.15}, {}), "front unchanged")
	_check(ev.call({"type": "formation_changed", "min_ratio": 0.15}, {"units": [wide, enemy]}), "front widened by dragging")
	_check(ev.call({"type": "formation_changed"}, {"units": [column, enemy]}), "formation changed")
	var charging := unit.duplicate()
	charging["state"] = "charging"
	var charge := {"type": "unit_state", "category": "cavalry", "states": ["charging", "melee"]}
	_check(not ev.call(charge, {}), "no charge yet")
	_check(ev.call(charge, {"units": [charging, enemy]}), "cavalry charging")
	_check(not ev.call({"type": "unit_state", "category": "ranged", "states": ["shooting"]}, {"units": [charging, enemy]}), "no archer shooting")
	_check(not ev.call({"type": "paused"}, {}), "running")
	_check(ev.call({"type": "paused"}, {"paused": true}), "paused")
	_check(not ev.call({"type": "victory"}, {"finished": true, "winner": "attacker"}), "defeat is not victory")
	_check(ev.call({"type": "victory"}, {"finished": true, "winner": "defender"}), "victory")


func _check_menu_entry() -> void:
	var menu := HistoricalBattlesMenu.new()
	root.add_child(menu)
	await process_frame
	_check(menu.prologue_button != null, "prologue entry in the historical battles screen")
	var start_menu: Control = (load("res://scenes/start_menu.tscn") as PackedScene).instantiate()
	root.add_child(start_menu)
	await process_frame
	_check(start_menu.get("historical_button") != null, "historical battles entry kept in the main menu")
	start_menu.queue_free()
	menu.queue_free()
	await process_frame


func _check_battle() -> void:
	var data := BattlePrologue.load_data()
	BattleScene.custom_config = (data["battle"] as Dictionary).duplicate(true)
	BattleScene.prologue_data = data
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	root.add_child(scene)
	await process_frame
	await process_frame
	var battle: Object = scene.get("battle")
	var prologue: BattlePrologue = scene.get("prologue")
	if not _check(battle != null and prologue != null, "prologue battle built"):
		scene.queue_free()
		return
	_check(BattleScene.prologue_data.is_empty(), "prologue data consumed")
	_check(scene.get("deployment") == null, "no deployment phase")
	_check(str(scene.get("player_side")) == "defender", "player side")
	_check((battle.call("get_units") as Array).size() == 8, "4 regiments each")
	_check(prologue.overlay != null and prologue.overlay.visible, "guide shown")
	_check(not prologue.enemy_ai_enabled, "enemy passive at first")
	_check(prologue.current_step_id() == "intro", "starts on intro")
	scene.set("paused", true)  # le test fait avancer la bataille lui-même
	var enemy_start := _positions(battle, "attacker")
	# Introduction : « Continuer ».
	prologue.overlay.continue_pressed.emit()
	_check(prologue.current_step_id() == "camera", "continue -> camera")
	_check(not prologue.check_now(), "camera untouched")
	var rig: Node = scene.get("camera_rig")
	rig.set("target", (rig.get("target") as Vector3) + Vector3(80, 0, 0))
	_pass(prologue, "camera")
	# Sélection.
	var own := _own(battle, "defender")
	scene.set("selected", [int(own["infantry"])] as Array[int])
	_pass(prologue, "select")
	# Marche.
	scene.call("issue", {"type": "move", "units": [int(own["infantry"])], "x": _unit(battle, own["infantry"])["x"] + 40.0, "z": _unit(battle, own["infantry"])["z"]})
	_run(scene, battle, 25.0)
	_pass(prologue, "move")
	# Front (glisser) ou formation.
	scene.call("issue", {"type": "formation", "units": [int(own["infantry"])], "kind": "column"})
	_run(scene, battle, 3.0)
	_pass(prologue, "formation")
	# Charge de cavalerie.
	var target := _nearest_enemy(battle, _unit(battle, own["cavalry"]))
	scene.call("issue", {"type": "attack", "units": [int(own["cavalry"])], "target": target, "run": true})
	var charged := false
	for _i in 120:
		_run(scene, battle, 0.5)
		if prologue.check_now():
			charged = true
			break
	_check(charged, "cavalry charge seen")
	_advance(prologue, "charge")
	# Tir d'archers : on les approche de l'ennemi.
	var archers := _unit(battle, own["ranged"])
	var foe := _unit(battle, _nearest_enemy(battle, archers))
	scene.call("issue", {"type": "move", "units": [int(own["ranged"])], "x": float(foe["x"]), "z": lerpf(float(archers["z"]), float(foe["z"]), 0.6), "run": true})
	var fired := false
	for _i in 240:
		_run(scene, battle, 0.5)
		if prologue.check_now():
			fired = true
			break
	_check(fired, "archers shooting seen")
	_advance(prologue, "fire")
	var enemy_moved := _max_shift(enemy_start, _positions(battle, "attacker"))
	print("nt4: passive enemy max shift %.1f m" % enemy_moved)
	# Pause.
	scene.set("paused", false)
	_check(not prologue.check_now(), "not paused yet")
	scene.set("paused", true)
	_pass(prologue, "pause")
	_check(prologue.current_step_id() == "victory", "pause -> victory")
	_check(prologue.enemy_ai_enabled, "enemy wakes up on the victory step")
	# Victoire : l'IA mène aussi le joueur jusqu'à la fin.
	battle.call("set_ai", "defender", true)
	for _i in 1200:
		if bool(battle.call("is_finished")):
			break
		battle.call("tick", 0.5)
	scene.set("units", battle.call("get_units"))
	var outcome: Dictionary = battle.call("get_outcome") if bool(battle.call("is_finished")) else {}
	print("nt4: battle finished=%s winner=%s at %.0f s" % [battle.call("is_finished"), outcome.get("winner", "?"), float(battle.call("get_elapsed"))])
	_check(str(outcome.get("winner", "")) == "defender", "the English win the skirmish")
	_pass(prologue, "victory")
	_check(prologue.current_step_id() == "outro", "victory -> outro")
	prologue.overlay.continue_pressed.emit()
	_check(prologue.done and not prologue.overlay.visible, "guide closed after the last step")
	scene.queue_free()
	await process_frame


## L'étape `id` doit passer à la vérification, puis on enchaîne (sans la pause de 0,8 s).
func _pass(prologue: BattlePrologue, id: String) -> void:
	_check(prologue.current_step_id() == id, "on step %s (at %s)" % [id, prologue.current_step_id()])
	_check(prologue.check_now(), "step %s passes" % id)
	_advance(prologue, id)


func _advance(prologue: BattlePrologue, id: String) -> void:
	if prologue.current_step_id() == id:
		prologue.advance()


func _run(scene: Node, battle: Object, seconds: float) -> void:
	var t := 0.0
	while t < seconds:
		battle.call("tick", 0.1)
		t += 0.1
	scene.set("units", battle.call("get_units"))


func _unit(battle: Object, id: int) -> Dictionary:
	for unit in battle.call("get_units"):
		if int(unit["id"]) == id:
			return unit
	return {}


## Premier régiment du camp `side` par catégorie ({category: id}).
func _own(battle: Object, side: String) -> Dictionary:
	var out := {}
	for unit in battle.call("get_units"):
		if str(unit["side"]) == side and not out.has(str(unit["category"])) and not bool(unit["is_general"]):
			out[str(unit["category"])] = int(unit["id"])
	for unit in battle.call("get_units"):
		if str(unit["side"]) == side and not out.has(str(unit["category"])):
			out[str(unit["category"])] = int(unit["id"])
	return out


func _nearest_enemy(battle: Object, unit: Dictionary) -> int:
	var best := -1
	var best_d := INF
	for other in battle.call("get_units"):
		if str(other["side"]) == str(unit["side"]) or not bool(other["present"]):
			continue
		var d := Vector2(float(other["x"]), float(other["z"])).distance_to(Vector2(float(unit["x"]), float(unit["z"])))
		if d < best_d:
			best_d = d
			best = int(other["id"])
	return best


func _positions(battle: Object, side: String) -> Dictionary:
	var out := {}
	for unit in battle.call("get_units"):
		if str(unit["side"]) == side:
			out[int(unit["id"])] = Vector2(float(unit["x"]), float(unit["z"]))
	return out


func _max_shift(before: Dictionary, after: Dictionary) -> float:
	var best := 0.0
	for id in before:
		if after.has(id):
			best = maxf(best, (before[id] as Vector2).distance_to(after[id]))
	return best


func _check(cond: bool, msg: String) -> bool:
	if not cond:
		_failures += 1
		push_error("nt4: " + msg)
	return cond
