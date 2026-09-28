extends SceneTree

## CB6 : formations de groupe (préréglages d'attaque, de défense et de marche).
## 1. Fonctions pures : `BattleFormationPicker.shortcut_index` (Alt+Maj+1…9), `tooltip_for`,
##    `slot_orders` (un `move` par place : largeur, `match_speed`, `group_tag`),
##    `BattleGroups.lock_as` (verrou dans la forme des places).
## 2. Intégration sur la démo autonome (`battle.tscn`) : six préréglages du pont, sélecteur visible
##    (Attaque / Défense / Marche, infobulles), Alt+Maj+2 active « La herse » ; clic droit avec
##    le préréglage actif = fantômes aux places du cœur, un `move` par régiment et groupe
##    verrouillé ; puis déploiement : « Placer en formation » propose (fantômes), un second appui
##    valide (`deploy_unit` avec largeur), les régiments sont à leurs places.
##
## Usage : godot --headless --path game --script res://tests/cb6_group_formation_test.gd

var _failures := 0
var _scene: Node = null


func _init() -> void:
	await process_frame
	_hermetic_settings()
	_check_pure()
	await _check_integration()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	print("cb6_group_formation_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


## Réglages par défaut (fichier de test) : la « Taille de l'interface » du joueur
## (`user://settings.cfg`, partagé par toutes les copies du dépôt) change l'échelle de la fenêtre,
## donc le point visé par le clic droit simulé.
func _hermetic_settings() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("_apply_ui_scale")


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("cb6_group_formation_test: " + message)
	return condition


func _key(code: Key, alt: bool, shift: bool) -> InputEventKey:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.alt_pressed = alt
	event.shift_pressed = shift
	event.pressed = true
	return event


func _check_pure() -> void:
	_check(BattleFormationPicker.shortcut_index(_key(KEY_3, true, true)) == 3, "Alt+Shift+3 is preset 3")
	_check(BattleFormationPicker.shortcut_index(_key(KEY_3, false, true)) == 0, "Shift+3 alone is not a preset")
	_check(BattleFormationPicker.shortcut_index(_key(KEY_3, true, false)) == 0, "Alt+3 is left to the abilities (CB4)")
	_check(BattleFormationPicker.shortcut_index(_key(KEY_A, true, true)) == 0, "Alt+Shift+A is not a preset")
	var tip := BattleFormationPicker.tooltip_for({"name_fr": "La herse", "description_fr": "Défense à l'anglaise."}, 2)
	_check(tip.contains("La herse") and tip.contains("Défense à l'anglaise.") and tip.contains("Alt+Maj+2"), "tooltip: name, description, shortcut (%s)" % tip)

	var places := [
		{"id": 3, "x": 10.0, "z": 20.0, "facing": 0.5, "order_width": 0.0},
		{"id": 5, "x": 40.0, "z": 25.0, "facing": 0.2, "order_width": 33.0},
	]
	var orders := BattleFormationPicker.slot_orders(places, 7, true, false)
	_check(orders.size() == 2, "one move per place")
	if orders.size() == 2:
		_check(str(orders[0]["type"]) == "move" and orders[0]["units"] == [3] and bool(orders[0]["run"]), "individual move of the first place")
		_check(not orders[0].has("width") and float(orders[1]["width"]) == 33.0, "width only where the preset sets files")
		_check(int(orders[0]["group_tag"]) == 7 and bool(orders[1]["match_speed"]), "shared tag, pace of the slowest")
		_check(not orders[0].has("queue"), "not queued by default")
	var queued := BattleFormationPicker.slot_orders(places, 0, false, true)
	_check(bool(queued[0]["queue"]) and not queued[0].has("group_tag"), "queued orders; no tag without a group")

	var groups := BattleGroups.new()
	var tag := groups.lock_as([{"id": 3, "x": 0.0, "z": 0.0, "facing": 0.0}, {"id": 5, "x": 40.0, "z": 0.0, "facing": 0.0}])
	_check(tag > 0 and groups.locked_group_for([3, 5]) == tag, "lock_as locks the places")
	var moved := groups.lock_places(tag, Vector2(100, 100), NAN)
	_check(moved.size() == 2 and absf(float(moved[1]["x"]) - float(moved[0]["x"]) - 40.0) < 1e-3, "the lock keeps the formation shape")
	var again := groups.lock_as([{"id": 5, "x": 0.0, "z": 0.0, "facing": 0.0}, {"id": 8, "x": 20.0, "z": 0.0, "facing": 0.0}])
	_check(again != tag and groups.lock_of(3) == 0 and groups.lock_of(5) == again, "a new formation takes its regiments out of their old group")
	_check(groups.lock_as([{"id": 9, "x": 0.0, "z": 0.0, "facing": 0.0}]) == 0, "a single regiment is not locked")


func _check_integration() -> void:
	root.size = Vector2i(1440, 900)  # headless : la fenêtre par défaut est minuscule (64x64)
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.autoplay = true
	_scene.log_orders_for_test = true
	_scene.paused = true  # temps figé : le déploiement reste possible (aucun tick)
	root.add_child(_scene)
	for _i in 6:
		await process_frame
	if not _check(_scene.battle != null and (_scene.units as Array).size() > 0, "battle demo failed to stage (run core/build.sh?)"):
		return
	if not _check(_scene.battle.has_method("formation_presets") and _scene.battle.has_method("formation_slots"), "bridge without CB6 queries (run core/build.sh)"):
		return
	_scene.battle.call("set_ai", _scene.player_side, false)
	var presets: Array = _scene.battle.call("formation_presets")
	_check(presets.size() == 6, "six presets (%d)" % presets.size())
	var picker: BattleFormationPicker = _scene.formation_picker
	if not _check(picker != null and picker.visible and picker.presets.size() == 6, "formation picker shown"):
		return
	_check(_has_row(picker, "Row_attack"), "attack row")
	_check(_has_row(picker, "Row_defense") and _has_row(picker, "Row_march"), "defence and march rows")
	var harrow_button: Button = picker._buttons.get("harrow")
	_check(harrow_button != null and harrow_button.tooltip_text.contains("Crécy"), "harrow tooltip from description_fr")
	# Pas de chevauchement avec le journal (haut droite) : sous lui, au-dessus du bandeau.
	var rect := picker.get_global_rect()
	_check(rect.position.y > 300.0 and rect.end.y <= 900.0 - BattleHud.BAND_HEIGHT, "picker above the card band, below the log (%s)" % rect)

	# Alt+Maj+2 : « La herse ».
	_scene.get_viewport().push_input(_key(KEY_2, true, true))
	_check(picker.active_id == "harrow" and harrow_button.button_pressed, "Alt+Shift+2 picks the harrow (%s)" % picker.active_id)
	_check(_scene.hud.groups.groups.is_empty(), "the shortcut does not recall selection group 2")

	# Bataille : toute l'armée du joueur, clic droit devant elle.
	var own: Array[int] = []
	var center := Vector3.ZERO
	for unit in _scene.units:
		if bool(unit["present"]) and str(unit["side"]) == _scene.player_side:
			own.append(int(unit["id"]))
			center += Vector3(float(unit["x"]), 0, float(unit["z"]))
	if not _check(own.size() >= 3, "the player has an army (%d)" % own.size()):
		return
	center /= own.size()
	var ahead := Vector3(0, 0, 60.0 if _scene.player_side == "attacker" else -60.0)
	var target := center + ahead
	_scene.selected.assign(own)
	_scene.camera_rig.look_at_point(target, 420.0, 0.0)
	for _i in 3:
		await process_frame
	var camera: Camera3D = _scene.camera_rig.camera
	var screen := camera.unproject_position(Vector3(target.x, _scene.terrain.height_at(target.x, target.z), target.z))
	_scene.issued_log.clear()
	_press_right(screen, true)
	_scene.input._preview_right(screen, true)
	_check(_scene.path_preview.live_ghost_sizes().size() >= own.size() - 1, "ghosts at the preset's places (%d for %d)" % [_scene.path_preview.live_ghost_sizes().size(), own.size()])
	_press_right(screen, false)
	_scene.input._last_right_click_ms = -10000
	var log: Array = _scene.issued_log
	_check(log.size() == own.size(), "one move per regiment (%d for %d)" % [log.size(), own.size()])
	var tag: int = _scene.hud.groups.locked_group_for(own)
	_check(tag > 0, "the group is locked in the formation")
	if log.size() == own.size():
		var shared := true
		for command in log:
			shared = shared and str(command["type"]) == "move" and int(command.get("group_tag", -1)) == tag and bool(command.get("match_speed", false))
		_check(shared, "moves share the lock tag and the pace of the slowest (%s)" % [log[0]])
	# Les places sont celles du cœur.
	var direction := Vector2(target.x - center.x, target.z - center.z)
	var slots: Array = _scene.battle.call("formation_slots", "harrow", PackedInt32Array(own), target.x, target.z, atan2(direction.x, direction.y))
	var same := slots.size() == log.size()
	for k in mini(slots.size(), log.size()):
		same = same and absf(float(slots[k]["x"]) - float(log[k]["x"])) < 0.5 and absf(float(slots[k]["z"]) - float(log[k]["z"])) < 0.5
	_check(same, "orders go to the core's places")

	# Déploiement : proposition puis validation.
	_scene.hud.groups.locks.clear()
	picker.toggle("harrow")  # désactive
	_check(not picker.is_active(), "clicking the active preset turns it off")
	var controller := DeploymentController.new()
	controller.name = "Deployment"
	_scene.add_child(controller)
	if not _check(controller.open(_scene), "deployment opens before the first tick"):
		controller.queue_free()
		return
	_scene.deployment = controller
	await process_frame
	_check(picker.place_button.visible and picker.place_button.disabled, "place button shown in deployment, disabled without preset")
	picker.select_index(2)
	_scene.selected.clear()
	picker.on_place_pressed()
	_check(picker.pending.size() == own.size(), "a proposal for the whole army (%d)" % picker.pending.size())
	_check(_scene.path_preview.live_ghost_sizes().size() == picker.pending.size(), "ghosts of the proposal (%d)" % _scene.path_preview.live_ghost_sizes().size())
	_check(picker.place_button.text == "Valider la formation" and picker.cancel_button.visible, "the button now validates")
	var zone: Dictionary = controller.zone
	var inside := true
	for slot in picker.pending:
		inside = inside and float(slot["x"]) >= float(zone["x0"]) - 0.01 and float(slot["x"]) <= float(zone["x1"]) + 0.01 and float(slot["z"]) >= float(zone["z0"]) - 0.01 and float(slot["z"]) <= float(zone["z1"]) + 0.01
	_check(inside, "proposal inside the deployment zone")
	var proposal: Array = picker.pending.duplicate(true)
	var placed := picker.apply()
	_check(placed == own.size(), "every regiment deployed (%d)" % placed)
	_check(picker.pending.is_empty() and picker.place_button.text == "Placer en formation", "proposal cleared")
	var at_place := true
	for slot in proposal:
		var unit := _unit(int(slot["id"]))
		at_place = at_place and Vector2(float(unit["x"]), float(unit["z"])).distance_to(Vector2(float(slot["x"]), float(slot["z"]))) < 0.5
	_check(at_place, "regiments stand at their places")
	# Annuler.
	picker.on_place_pressed()
	picker.cancel()
	_check(picker.pending.is_empty() and not picker.cancel_button.visible, "cancel drops the proposal")
	controller.finish()


func _has_row(picker: Node, row_name: String) -> bool:
	return picker.find_child(row_name, true, false) != null


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
