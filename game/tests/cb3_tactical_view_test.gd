extends TestCase

## CB3 : vue tactique (Tab).
## 1. `BattleTacticalView.filter_spotted` (fonction pure) : les siens toujours, un ennemi
##    seulement s'il est `spotted` (champ absent = rétrocompatibilité, affiché).
## 2. Intégration : démo autonome de bataille (`battle.tscn`) — Tab entre en vue tactique
##    (caméra du dessus, `manual_pitch` actif, borné), Tab en ressort (caméra restaurée à
##    l'identique) ; Échap en sort aussi ; les ennemis non `spotted` sont absents des repères.
##
## Usage : godot --headless --path game --script res://tests/cb3_tactical_view_test.gd

var _scene: Node = null


func _init() -> void:
	await process_frame
	_check_filter_spotted()
	await _check_integration()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	finish()


func _check_filter_spotted() -> void:
	var mine := {"side": "attacker", "spotted": true}
	var enemy_spotted := {"side": "defender", "spotted": true}
	var enemy_hidden := {"side": "defender", "spotted": false}
	var enemy_no_field := {"side": "defender"}  # rétrocompatibilité : rejeu d'avant CB3
	var units := [mine, enemy_spotted, enemy_hidden, enemy_no_field]
	var shown: Array = BattleTacticalView.filter_spotted(units, "attacker")
	check(shown.has(mine), "own regiments always shown")
	check(shown.has(enemy_spotted), "spotted enemies shown")
	check(not shown.has(enemy_hidden), "unspotted enemies hidden")
	check(shown.has(enemy_no_field), "an enemy without the field stays shown (back-compat)")
	check(shown.size() == 3, "expected 3 shown, got %d" % shown.size())


func _check_integration() -> void:
	root.size = Vector2i(1440, 900)  # headless : la fenêtre par défaut est minuscule (64x64)
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	if not check(_scene.has_method("issue"), "battle scene script failed to load (run godot --import?)"):
		return
	_scene.autoplay = true
	root.add_child(_scene)
	for _i in 6:
		await process_frame
	if not check(_scene.battle != null and (_scene.units as Array).size() > 0, "battle demo failed to stage (run core/build.sh?)"):
		return
	_scene.paused = true
	await process_frame
	var tactical: BattleTacticalView = _scene.get("tactical_view")
	if not check(tactical != null, "no tactical view node"):
		return
	if not check(tactical.has_method("enter") and tactical.has_method("exit") and tactical.has_method("toggle"), "tactical view API missing"):
		return
	var cam: BattleCamera = _scene.camera_rig
	var before_target := cam.target
	var before_distance := cam.distance
	var before_yaw := cam.yaw
	var before_manual := cam.manual_pitch
	var before_manual_deg := cam.manual_pitch_deg

	check(not tactical.active, "tactical view starts closed")
	_press_key(KEY_TAB)  # entrée, par la touche (comme le joueur)
	await process_frame
	check(tactical.active, "Tab should enter the tactical view")
	check(cam.manual_pitch, "tactical view forces a manual (overhead) pitch")
	check(is_equal_approx(cam.manual_pitch_deg, BattleTacticalView.PITCH_DEG), "overhead pitch, got %f" % cam.manual_pitch_deg)
	check(cam.yaw == 0.0, "tactical view faces due north (yaw 0)")

	# Ennemis non repérés masqués des repères (le champ `spotted` vient du cœur, CB3).
	var units: Array = _scene.units
	var hidden_present := false
	for unit in units:
		if str(unit["side"]) != _scene.player_side and not bool(unit.get("spotted", true)):
			hidden_present = true
			break
	if hidden_present:
		_scene._update_markers(1.0)
		var markers: BattleUnitMarkers = _scene.markers
		for unit in units:
			if str(unit["side"]) != _scene.player_side and not bool(unit.get("spotted", true)):
				check(not markers._key_of.has(int(unit["id"])), "unspotted enemy %d should have no marker" % int(unit["id"]))
	else:
		print("cb3_tactical_view_test: no unspotted enemy in this demo battle, marker-hiding check skipped")

	_press_key(KEY_TAB)  # sortie, second appui
	await process_frame
	check(not tactical.active, "a second Tab should exit the tactical view")
	check(cam.target.is_equal_approx(before_target), "camera target restored")
	check(is_equal_approx(cam.distance, before_distance), "camera distance restored")
	check(is_equal_approx(cam.yaw, before_yaw), "camera yaw restored")
	check(cam.manual_pitch == before_manual, "manual pitch flag restored")
	if before_manual:
		check(is_equal_approx(cam.manual_pitch_deg, before_manual_deg), "manual pitch value restored")

	# Échap sort aussi de la vue tactique (au lieu de seulement désélectionner).
	_press_key(KEY_TAB)  # entrée de nouveau, pour le test d'Échap
	await process_frame
	check(tactical.active, "entered again for the Escape check")
	_press_key(KEY_ESCAPE)
	await process_frame
	check(not tactical.active, "Escape should exit the tactical view")


func _press_key(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		_scene.get_viewport().push_input(event)
