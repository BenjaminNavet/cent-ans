class_name BattleInput
extends Node

## CB0 : entrées de bataille (clics, glisser, touches, groupes, sélection rapide), extraites de
## `battle_scene.gd` (même comportement, seul l'emplacement change ; voir
## `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`, section CB0). Nœud enfant de
## `BattleScene` : lit l'état partagé sur `scene` (sélection, unités, caméra, HUD, pont) et
## notifie ses décisions par signal ; la scène connecte ces signaux à `issue()`, au rendu et au
## pont. `game/tests/smoke.gd` continue d'appeler `scene.issue` et `scene.handle_group_key`
## (délégations fines gardées sur la scène).

signal command_requested(command: Dictionary)
signal selection_changed(ids: Array)
signal camera_focus_requested(point: Vector3)
signal pause_toggled
signal speed_step(delta: int)
signal help_toggled
signal markers_toggled
signal screenshot_requested

const DOUBLE_CLICK_MS := 350

## La scène, assignée par elle avant `add_child`.
var scene: BattleScene = null

var _left_press: Vector2 = Vector2(-1, -1)
var _right_press: Vector2 = Vector2(-1, -1)
var _right_press_ground: Vector3 = Vector3.ZERO
var _last_right_click_ms: int = -10000
var _last_group_ms: int = -10000


func _unhandled_input(event: InputEvent) -> void:
	if scene.battle == null:
		return
	if event is InputEventKey and event.pressed and not event.echo:
		var key := event as InputEventKey
		# F5b : chiffres de la rangée (touche physique, AZERTY compris) = groupes de sélection.
		if key.physical_keycode >= KEY_1 and key.physical_keycode <= KEY_9:
			handle_group_key(int(key.physical_keycode - KEY_0), key.ctrl_pressed or key.meta_pressed)
			return
		match event.keycode:
			KEY_ENTER, KEY_KP_ENTER:
				if scene.deployment != null:
					scene.deployment.finish()
			KEY_SPACE:
				pause_toggled.emit()
			KEY_PLUS, KEY_EQUAL, KEY_KP_ADD:
				speed_step.emit(1)
			KEY_MINUS, KEY_KP_SUBTRACT:
				speed_step.emit(-1)
			KEY_F1:
				help_toggled.emit()
			KEY_U:
				markers_toggled.emit()
			KEY_F:
				_on_command("formation")
			KEY_G:
				_on_command("fire_at_will")
			KEY_H:
				_on_command("halt")
			KEY_C:
				scene._toggle_camera_follow()
			KEY_ESCAPE:
				scene.selected.clear()
				selection_changed.emit(scene.selected)
			KEY_F12:
				screenshot_requested.emit()
	elif event is InputEventMouseButton:
		var button := event as InputEventMouseButton
		if button.button_index == MOUSE_BUTTON_LEFT:
			if button.pressed:
				_left_press = button.position
			else:
				_finish_left(button.position, button.shift_pressed)
		elif button.button_index == MOUSE_BUTTON_RIGHT:
			if button.pressed:
				_right_press = button.position
				_right_press_ground = scene.ground_point(button.position)
			else:
				_finish_right(button.position)
	elif event is InputEventMouseMotion and _left_press.x < 0.0 and scene.markers != null:
		# B2 : survol d'une troupe sur le terrain = repère mis en évidence.
		var hover_at := (event as InputEventMouseMotion).position
		var hover := scene.pick_unit(hover_at, scene.player_side)
		scene.markers.world_hover = hover if hover >= 0 else scene.pick_unit(hover_at, scene.enemy_side)
	elif event is InputEventMouseMotion and _left_press.x >= 0.0:
		var motion := event as InputEventMouseMotion
		var rect := Rect2(_left_press, motion.position - _left_press).abs()
		scene._drag_rect.visible = rect.size.length() > 8.0
		scene._drag_rect.position = rect.position
		scene._drag_rect.size = rect.size


## Ctrl+n (Cmd+n sous macOS) : enregistre la sélection ; n : la rappelle, et un second appui
## rapide centre la caméra sur le groupe.
func handle_group_key(number: int, save: bool) -> void:
	if save:
		scene.hud.groups.save(number, scene.selected)
		return
	var ids := scene.hud.groups.recall(number, scene.units)
	if ids.is_empty():
		return
	var again := scene.selected == ids and Time.get_ticks_msec() - _last_group_ms < 600
	_last_group_ms = Time.get_ticks_msec()
	scene.selected = ids
	selection_changed.emit(scene.selected)
	if again:
		var center := Vector3.ZERO
		for unit in scene.units:
			if ids.has(int(unit["id"])):
				center += Vector3(float(unit["x"]), 0, float(unit["z"]))
		camera_focus_requested.emit(center / ids.size())


func _finish_left(position: Vector2, additive: bool) -> void:
	var rect := Rect2(_left_press, position - _left_press).abs()
	_left_press = Vector2(-1, -1)
	scene._drag_rect.visible = false
	if not additive:
		scene.selected.clear()
	if rect.size.length() > 8.0:
		for unit in scene.units:
			if str(unit["side"]) != scene.player_side or not bool(unit["present"]):
				continue
			var screen := scene._unit_screen(unit)
			if screen.x > -1e5 and rect.has_point(screen) and not scene.selected.has(int(unit["id"])):
				scene.selected.append(int(unit["id"]))
	else:
		var picked := scene.pick_unit(position, scene.player_side)
		if picked >= 0:
			if additive and scene.selected.has(picked):
				scene.selected.erase(picked)
			elif not scene.selected.has(picked):
				scene.selected.append(picked)
	selection_changed.emit(scene.selected)


func _finish_right(position: Vector2) -> void:
	var press := _right_press
	_right_press = Vector2(-1, -1)
	if scene.selected.is_empty() or scene.replay_mode:  # EP13 : aucun ordre pendant un rejeu
		return
	if scene.deployment != null and scene.deployment.active:
		_deploy_selection(press, position)
		return
	var now := Time.get_ticks_msec()
	var double_click := now - _last_right_click_ms < DOUBLE_CLICK_MS
	_last_right_click_ms = now
	if press.distance_to(position) > 20.0:
		# Glisser-droit : ligne de p0 à p1, front tourné à l'opposé de la caméra.
		var p0 := _right_press_ground
		var p1 := scene.ground_point(position)
		var dir := Vector2(p1.x - p0.x, p1.z - p0.z)
		if dir.length() < 2.0:
			return
		var normal := Vector2(-dir.y, dir.x).normalized()
		var mid := (p0 + p1) * 0.5
		var cam := scene.camera_rig.camera.global_position
		if normal.dot(Vector2(mid.x - cam.x, mid.z - cam.z)) < 0.0:
			normal = -normal
		command_requested.emit({"type": "move", "units": scene.selected.duplicate(), "x": mid.x, "z": mid.z, "run": double_click, "facing": atan2(normal.x, normal.y)})
		return
	var enemy := scene.pick_unit(position, scene.enemy_side)
	if enemy >= 0:
		command_requested.emit({"type": "attack", "units": scene.selected.duplicate(), "target": enemy, "run": true})
		return
	var point := scene.ground_point(position)
	command_requested.emit({"type": "move", "units": scene.selected.duplicate(), "x": point.x, "z": point.z, "run": double_click})


func _on_command(command: String) -> void:
	match command:
		"pause":
			pause_toggled.emit()
			return
		"withdraw_all":
			var all: Array[int] = []
			for unit in scene.units:
				if str(unit["side"]) == scene.player_side and bool(unit["present"]) and str(unit["state"]) != "routing" and not bool(unit["withdrawing"]):
					all.append(int(unit["id"]))
			if not all.is_empty():
				command_requested.emit({"type": "withdraw", "units": all})
			return
	var ids := _available_selection()
	if ids.is_empty():
		return
	match command:
		"halt":
			command_requested.emit({"type": "halt", "units": ids})
		"withdraw":
			command_requested.emit({"type": "withdraw", "units": ids})
		"fire_at_will":
			var shooters: Array[int] = []
			var enable := false
			for unit in scene.units:
				if ids.has(int(unit["id"])) and bool(unit["can_shoot"]):
					shooters.append(int(unit["id"]))
					enable = enable or not bool(unit["fire_at_will"])
			if not shooters.is_empty():
				command_requested.emit({"type": "fire_at_will", "units": shooters, "enabled": enable})
		"formation":
			for unit in scene.units:
				if ids.has(int(unit["id"])):
					command_requested.emit({"type": "formation", "units": [int(unit["id"])], "kind": _next_formation(unit)})


func _available_selection() -> Array[int]:
	var ids: Array[int] = []
	for unit in scene.units:
		if scene.selected.has(int(unit["id"])) and bool(unit["present"]) and str(unit["state"]) != "routing":
			ids.append(int(unit["id"]))
	return ids


## Formation suivante autorisée pour la famille de l'unité.
func _next_formation(unit: Dictionary) -> String:
	var cycle: Array = ["line", "column"]
	match str(unit["category"]):
		"infantry":
			cycle = ["line", "column", "square"]
		"cavalry":
			cycle = ["line", "wedge", "column"]
		"siege":
			cycle = ["line"]
	var current := cycle.find(str(unit["formation"]))
	return cycle[(current + 1) % cycle.size()]


func _deploy_selection(press: Vector2, release: Vector2) -> void:
	var p1 := scene.ground_point(release)
	var p0 := _right_press_ground if press.x >= 0.0 and press.distance_to(release) > 20.0 else p1
	scene.deployment.place(scene.selected.duplicate(), p0, p1, scene.camera_rig.camera.global_position)
	scene._refresh_view(true)
