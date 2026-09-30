extends SceneTree

## CB5 : alertes de bataille (colonne, fusion, borne à 5, clic -> caméra + minicarte).
## 1. `BattleAlertsColumn` en isolation (fonction pure autant que possible : fusion, borne,
##    expiration, signal `pinged`).
## 2. Intégration : démo autonome de bataille (`battle.tscn`) — la colonne existe, `get_alerts`
##    répond, un clic recentre la caméra et fait pulser la minicarte.
##
## Usage : godot --headless --path game --script res://tests/cb5_alerts_test.gd

var _failures := 0
var _scene: Node = null


func _init() -> void:
	await process_frame
	await _check_column_pure()
	await _check_integration()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	print("cb5_alerts_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("cb5_alerts_test: " + message)
	return condition


func _alert(kind: String, time: float, x: float, z: float, side: String = "attacker", unit: int = 1) -> Dictionary:
	return {"kind": kind, "time": time, "x": x, "z": z, "side": side, "unit": unit}


func _check_column_pure() -> void:
	var col: Control = load("res://scripts/battle/battle_alerts_column.gd").new()
	root.add_child(col)
	await process_frame

	# Fusion : même type, même zone, dans la fenêtre -> une seule entrée, compte 2.
	col.push_alerts([_alert("rout", 0.0, 100.0, 200.0)])
	col.push_alerts([_alert("rout", 1.0, 105.0, 202.0)])
	_check(col._entries.size() == 1, "same kind/zone/window merges into one entry")
	if col._entries.size() == 1:
		_check(int(col._entries[0]["count"]) == 2, "merged entry counts 2")

	# Zone différente : pas de fusion.
	col.push_alerts([_alert("rout", 1.5, 900.0, 900.0, "defender", 9)])
	_check(col._entries.size() == 2, "a different zone does not merge: %d" % col._entries.size())

	# Hors fenêtre de fusion (le premier avait duré > merge_window_s depuis) : nouvelle entrée.
	col.push_alerts([_alert("rout", 1.5 + col.merge_window_s() + 1.0, 101.0, 201.0)])
	_check(col._entries.size() == 3, "past the merge window: a new entry")

	# Borne à 5 (data/rules/battle_alerts.json) : 7 types distincts, zones éloignées.
	col._entries.clear()
	var kinds := ["rout", "general_down", "flanked", "reinforcements", "ammo_out", "wall_breached", "gate_destroyed"]
	for i in kinds.size():
		col.push_alerts([_alert(kinds[i], float(i) * 1000.0, float(i) * 5000.0, 0.0, "", -1)])
	_check(col._entries.size() == kinds.size(), "all %d alerts kept internally" % kinds.size())
	_check(col.max_shown() == 5, "the column caps display at 5 (RuleValues/fallback)")
	_check(col.visible_entries().size() == col.max_shown(), "visible_entries() respects the cap")
	# La plus importante (général tombé) doit être affichée.
	var shown_kinds := []
	for entry in col.visible_entries():
		shown_kinds.append(str(entry["kind"]))
	_check(shown_kinds.has("general_down"), "the most important alert is shown: %s" % [shown_kinds])

	# Expiration : une entrée à durée écoulée disparaît et redessine.
	col._entries.clear()
	col.push_alerts([_alert("ammo_out", 0.0, 0.0, 0.0, "", -1)])
	col._entries[0]["remaining"] = 0.01
	col._process(0.02)
	_check(col._entries.is_empty(), "an expired alert is removed")

	# Clic -> signal `pinged` avec les coordonnées de l'alerte.
	col._entries.clear()
	for child in col._box.get_children():
		child.queue_free()
	col.push_alerts([_alert("gate_destroyed", 0.0, 321.0, 654.0, "", -1)])
	var got: Array = []
	col.pinged.connect(func(x: float, z: float) -> void: got.append([x, z]))
	await process_frame
	_check(col._box.get_child_count() > 0, "the column built a row")
	if col._box.get_child_count() > 0:
		(col._box.get_child(0) as BaseButton).pressed.emit()
		_check(got.size() == 1, "clicking a row emits pinged once")
		if got.size() == 1:
			_check(absf(float(got[0][0]) - 321.0) < 0.01 and absf(float(got[0][1]) - 654.0) < 0.01, "pinged carries the alert's world position")
	col.queue_free()


func _check_integration() -> void:
	root.size = Vector2i(1600, 900)  # headless : la fenêtre par défaut est minuscule (64x64)
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.autoplay = true
	root.add_child(_scene)
	for _i in 6:
		await process_frame
	if not _check(_scene.battle != null, "battle demo failed to stage (run core/build.sh?)"):
		return
	var hud: CanvasLayer = _scene.hud
	if not _check(hud.alerts_column != null and hud.root.is_ancestor_of(hud.alerts_column), "alerts column is under hud.root (VN4 : pile TOASTS)"):
		return
	var alerts: Array = _scene.battle.call("get_alerts")
	_check(alerts is Array, "get_alerts() answers an Array (bridge wired)")
	# Clic direct (sans attendre une vraie alerte en jeu) : caméra + repère minicarte.
	_scene._on_alert_pinged(400.0, 250.0)
	var target: Vector3 = _scene.camera_rig.target
	_check(absf(target.x - 400.0) < 0.01 and absf(target.z - 250.0) < 0.01, "the camera targets the alert's world position")
	_check(hud.minimap._ping_time >= 0.0, "the minimap ping is armed after a click")
