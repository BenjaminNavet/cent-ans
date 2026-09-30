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
signal tactical_view_toggled  # CB3 : touche Tab

const DOUBLE_CLICK_MS := 350

## La scène, assignée par elle avant `add_child`.
var scene: BattleScene = null

var _left_press: Vector2 = Vector2(-1, -1)
var _right_press: Vector2 = Vector2(-1, -1)
var _right_press_ground: Vector3 = Vector3.ZERO
var _last_right_click_ms: int = -10000
var _last_group_ms: int = -10000
## CB0 (sélection rapide) : double clic gauche (350 ms) sur une même troupe.
var _last_left_click_ms: int = -10000
var _last_left_click_unit: int = -1


func _unhandled_input(event: InputEvent) -> void:
	if scene.battle == null:
		return
	if event is InputEventKey and (event as InputEventKey).keycode == KEY_SHIFT and not event.echo:
		# CB-M3 : Maj pressée ou relâchée = ordre en file ou non : curseur et aperçu à revoir.
		scene.note_mouse(scene.get_viewport().get_mouse_position())
		if _right_press.x >= 0.0:
			_preview_right(scene.get_viewport().get_mouse_position(), false, event.pressed)
		return
	if event is InputEventKey and event.pressed and not event.echo:
		var key := event as InputEventKey
		# CB6 : Alt+Maj+1…6 = préréglage de formation de groupe (avant les groupes de sélection).
		if scene.formation_picker != null and BattleFormationPicker.shortcut_index(key) > 0:
			scene.formation_picker.select_index(BattleFormationPicker.shortcut_index(key))
			return
		# CB4 : Alt/Option+1…4 = capacité n° n de la sélection (avant les groupes de sélection).
		var slot := BattleHotkeys.ability_slot(key)
		if slot > 0:
			use_ability_slot(slot)
			return
		# F5b : chiffres de la rangée (touche physique, AZERTY compris) = groupes de sélection.
		if key.physical_keycode >= KEY_1 and key.physical_keycode <= KEY_9:
			handle_group_key(int(key.physical_keycode - KEY_0), key.ctrl_pressed or key.meta_pressed)
			return
		# CB2 : table unique des raccourcis (`BattleHotkeys`) : ordres, modes, verrou de groupe.
		var action := BattleHotkeys.action_for(key)
		if action != "":
			handle_action(action)
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
			KEY_A:
				# CB0 : sélection rapide, Ctrl/Cmd+A = toutes les troupes du joueur présentes.
				if key.ctrl_pressed or key.meta_pressed:
					_select_all_player_units()
			KEY_H:
				_on_command("halt")
			KEY_C:
				scene._toggle_camera_follow()
			KEY_TAB:
				# CB3 : vue tactique (caméra du dessus, ennemis non repérés masqués).
				tactical_view_toggled.emit()
			KEY_ESCAPE:
				# CB3 : Échap sort d'abord de la vue tactique si elle est ouverte.
				if scene.tactical_view != null and scene.tactical_view.active:
					scene.tactical_view.exit()
				elif not scene.selected.is_empty():
					scene.selected.clear()
					selection_changed.emit(scene.selected)
				else:
					scene.toggle_quit_menu()  # rien de sélectionné : « Quitter la bataille ? »
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
				_preview_right(button.position, false, button.shift_pressed)
			else:
				_finish_right(button.position, button.shift_pressed)
				if scene.path_preview != null:
					scene.path_preview.clear_live()
	elif event is InputEventMouseMotion and _left_press.x < 0.0 and scene.markers != null:
		# B2 : survol d'une troupe sur le terrain = repère mis en évidence.
		var hover_at := (event as InputEventMouseMotion).position
		var hover := scene.pick_unit(hover_at, scene.player_side)
		scene.markers.world_hover = hover if hover >= 0 else scene.pick_unit(hover_at, scene.enemy_side)
		# CB-M2 : curseur contextuel (recalculé une fois par image au plus) ; aperçu du trajet
		# pendant le clic droit maintenu.
		scene.note_mouse(hover_at)
		if _right_press.x >= 0.0:
			_preview_right(hover_at, false, (event as InputEventMouseMotion).shift_pressed)
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
			# CB0 : sélection rapide, double clic gauche (350 ms) sur une troupe = même type.
			var now := Time.get_ticks_msec()
			var double_click := picked == _last_left_click_unit and now - _last_left_click_ms < DOUBLE_CLICK_MS
			_last_left_click_ms = now
			_last_left_click_unit = picked
			if double_click:
				select_same_type_of(picked)
			elif additive and scene.selected.has(picked):
				scene.selected.erase(picked)
			elif not scene.selected.has(picked):
				scene.selected.append(picked)
	selection_changed.emit(scene.selected)


## `queued` (Maj, CB-M3) : l'ordre s'ajoute à la file des régiments au lieu de remplacer l'ordre
## en cours (`queue: true`) ; file pleine : rien n'est envoyé, message.
func _finish_right(position: Vector2, queued: bool = false) -> void:
	var press := _right_press
	_right_press = Vector2(-1, -1)
	if scene.selected.is_empty() or scene.replay_mode:  # EP13 : aucun ordre pendant un rejeu
		return
	if scene.deployment != null and scene.deployment.active:
		if _formation_active():
			scene.formation_picker.deploy_click(_right_press_ground, scene.ground_point(position), scene.camera_rig.camera.global_position, press.distance_to(position) > 20.0)
			return
		_deploy_selection(press, position)
		return
	if queued and BattlePathPreview.queue_full(scene.units, scene.selected):
		scene.hud.show_toast(queue_full_text())
		return
	var now := Time.get_ticks_msec()
	var double_click := now - _last_right_click_ms < DOUBLE_CLICK_MS
	_last_right_click_ms = now
	if _formation_active() and scene.pick_unit(position, scene.enemy_side) < 0:
		# CB6 : préréglage actif = places de la formation de groupe, puis groupe verrouillé.
		for command in scene.formation_picker.battle_orders(scene.selected, _right_press_ground, scene.ground_point(position), scene.camera_rig.camera.global_position, press.distance_to(position) > 20.0, double_click, queued):
			command_requested.emit(command)
		return
	var lock := _selected_lock()
	if press.distance_to(position) > 20.0:
		# Glisser-droit : ligne de p0 à p1, front tourné à l'opposé de la caméra ; CB1 : la longueur
		# du glisser donne la largeur du front (répartie au prorata des effectifs par le cœur), sauf
		# pour un groupe verrouillé, tourné et déplacé d'un bloc.
		var line := FormationDrag.drag_line(_right_press_ground, scene.ground_point(position), scene.camera_rig.camera.global_position)
		if not bool(line["ok"]):
			return
		var mid: Vector3 = line["mid"]
		var facing := float(line["facing"])
		if lock != 0:
			_issue_locked(lock, mid, facing, double_click, queued)
			return
		var width := float(line["width"])
		if not _path_allowed(mid, facing, queued, width):
			return
		command_requested.emit(_queued({"type": "move", "units": scene.selected.duplicate(), "x": mid.x, "z": mid.z, "run": double_click, "facing": facing, "width": width}, queued))
		return
	var enemy := scene.pick_unit(position, scene.enemy_side)
	if enemy >= 0:
		command_requested.emit(_queued({"type": "attack", "units": scene.selected.duplicate(), "target": enemy, "run": true}, queued))
		return
	var point := scene.ground_point(position)
	if lock != 0:
		_issue_locked(lock, point, NAN, double_click, queued)
		return
	if not _path_allowed(point, NAN, queued):
		return
	command_requested.emit(_queued({"type": "move", "units": scene.selected.duplicate(), "x": point.x, "z": point.z, "run": double_click}, queued))


## CB6 : un préréglage de formation de groupe est actif.
func _formation_active() -> bool:
	return scene.formation_picker != null and scene.formation_picker.is_active()


## CB1 : étiquette du groupe verrouillé qui forme toute la sélection (0 : aucun).
func _selected_lock() -> int:
	if scene.hud == null or scene.hud.groups == null:
		return 0
	return scene.hud.groups.locked_group_for(scene.selected)


## CB1 : ordres `move` individuels d'un groupe verrouillé déplacé en `point` et tourné vers
## `facing` (NaN : orientation du verrouillage), à l'allure du plus lent et sous l'étiquette du
## groupe ; rien d'envoyé si aucun régiment n'a de chemin.
func _issue_locked(tag: int, point: Vector3, facing: float, run: bool, queued: bool) -> void:
	var places: Array = scene.hud.groups.lock_places(tag, Vector2(point.x, point.z), facing)
	if places.is_empty():
		return
	var preview: BattlePathPreview = scene.path_preview
	if preview != null and scene.battle != null:
		preview.compute_places(places, scene.units, Time.get_ticks_msec() / 1000.0, queued)
		if not preview.reachable():
			scene.hud.show_toast("Ordre impossible : %s." % preview.refusal().to_lower())
			return
	for command in locked_orders(places, tag, run, queued):
		command_requested.emit(command)


## CB1 : les ordres d'un groupe verrouillé pour ses `places` ({id, x, z, facing}) : un `move`
## par régiment, `match_speed` et `group_tag` communs (fonction pure, testée).
static func locked_orders(places: Array, tag: int, run: bool, queued: bool) -> Array:
	var out: Array = []
	for place in places:
		var command := {"type": "move", "units": [int(place["id"])], "x": float(place["x"]), "z": float(place["z"]), "run": run, "facing": float(place["facing"]), "match_speed": true, "group_tag": tag}
		if queued:
			command["queue"] = true
		out.append(command)
	return out


## CB1 : Ctrl/Cmd+G — verrouille la sélection en groupe (ou la déverrouille) ; message court.
func toggle_lock() -> void:
	if scene.hud == null or scene.selected.is_empty():
		return
	var tag: int = scene.hud.groups.toggle_lock(scene.selected, scene.units)
	if tag != 0:
		scene.hud.show_toast("Groupe verrouillé : il se déplace d'un bloc, à l'allure du plus lent (Ctrl+G : déverrouiller).")
	else:
		scene.hud.show_toast("Groupe déverrouillé.")
	scene.hud.update_cards(scene.units, scene.player_side, scene.selected)


## CB-M3 : `queue: true` seulement pour un ordre en file (les ordres simples restent identiques).
func _queued(command: Dictionary, queued: bool) -> Dictionary:
	if queued:
		command["queue"] = true
	return command


## CB-M3 : texte de la file pleine (borne lue dans `data/rules/battle_queue.json` par RuleValues).
static func queue_full_text() -> String:
	return "File d'ordres pleine : %d ordres en attente au plus par régiment." % int(RuleValues.value("battle_queue_max", 8.0))


## CB-M2 : point visé et orientation d'un clic droit (ou glisser-droit) en cours, comme
## `_finish_right` les enverra ; aperçu du trajet étranglé (`final` : sans étranglement).
## Rien sur un ennemi (attaque : la cible bouge), en déploiement ni pendant un rejeu.
func _preview_right(position: Vector2, final: bool, queued: bool = false) -> void:
	var preview: BattlePathPreview = scene.path_preview
	if preview == null or scene.selected.is_empty() or scene.replay_mode:
		return
	if scene.deployment != null and scene.deployment.active and _formation_active():
		# CB6 : la proposition de formation suit le clic droit maintenu.
		scene.formation_picker.deploy_preview(_right_press_ground, scene.ground_point(position), scene.camera_rig.camera.global_position, _right_press.x >= 0.0 and _right_press.distance_to(position) > 20.0)
		return
	if scene.deployment != null and scene.deployment.active:
		# CB1 : fantômes du glisser-droit en déploiement (places et largeurs de `place`).
		if _right_press.x >= 0.0 and _right_press.distance_to(position) > 20.0:
			preview.show_ghosts(scene.deployment.plan(scene.selected, _right_press_ground, scene.ground_point(position), scene.camera_rig.camera.global_position), scene.units)
		else:
			preview.clear_live()
		return
	if scene.pick_unit(position, scene.enemy_side) >= 0:
		preview.clear_live()
		return
	if _formation_active():
		scene.formation_picker.preview(scene.selected, _right_press_ground, scene.ground_point(position), scene.camera_rig.camera.global_position, _right_press.x >= 0.0 and _right_press.distance_to(position) > 20.0, queued, final)
		return
	var point := scene.ground_point(position)
	var facing := NAN
	var width := 0.0
	if _right_press.x >= 0.0 and _right_press.distance_to(position) > 20.0:
		var line := FormationDrag.drag_line(_right_press_ground, point, scene.camera_rig.camera.global_position)
		if bool(line["ok"]):
			point = line["mid"]
			facing = float(line["facing"])
			width = float(line["width"])
	var now := Time.get_ticks_msec() / 1000.0
	# CB1 : groupe verrouillé : fantômes à ses places rigides (pas de largeur).
	var lock := _selected_lock()
	if lock != 0:
		var places: Array = scene.hud.groups.lock_places(lock, Vector2(point.x, point.z), facing)
		if final or preview.should_recompute(point, facing, now, queued):
			preview.compute_places(places, scene.units, now, queued, point, facing)
		return
	if final:
		preview.compute(scene.selected, scene.units, point, facing, now, queued, width)
	else:
		preview.request(scene.selected, scene.units, point, facing, now, queued, width)


## CB-M2 : l'ordre de déplacement vers `point` n'est envoyé que si au moins un régiment y a un
## chemin (verdict du cœur, `preview_paths`) ; sinon message et rien d'envoyé.
func _path_allowed(point: Vector3, facing: float, queued: bool = false, width: float = 0.0) -> bool:
	var preview: BattlePathPreview = scene.path_preview
	if preview == null or scene.battle == null:
		return true
	preview.compute(scene.selected, scene.units, point, facing, Time.get_ticks_msec() / 1000.0, queued, width)
	if preview.reachable():
		return true
	scene.hud.show_toast("Ordre impossible : %s." % preview.refusal().to_lower())
	return false


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
		"run", "guard", "skirmish", "melee", "breach":
			_toggle_mode(command, ids)


## CB2 : action d'une ligne `dispatch` de `BattleHotkeys` (touche pressée).
func handle_action(action: String) -> void:
	if action == "lock_group":
		toggle_lock()
		return
	_on_command(action)


## CB2 : ordre `set_mode` qui bascule `mode` sur les régiments de `ids` qui peuvent le prendre
## (`modes` de `get_units`, verdict du cœur) : activé si l'un d'eux ne l'a pas, sinon désactivé ;
## message si aucun ne le peut. Rien pendant un rejeu.
func _toggle_mode(mode: String, ids: Array[int]) -> void:
	if scene.replay_mode:
		return
	var command := mode_command(scene.units, ids, mode)
	if command.is_empty():
		scene.hud.show_toast("Aucune unité sélectionnée ne peut passer en mode %s." % BattleModeIcons.label_of(mode).to_lower())
		return
	command_requested.emit(command)


## CB2 : l'ordre `set_mode` de `mode` pour `ids` (fonction pure, testée) ; vide si aucun régiment
## de `ids` ne peut prendre le mode.
static func mode_command(units: Array, ids: Array, mode: String) -> Dictionary:
	var able: Array[int] = []
	var enable := false
	var field := BattleModeIcons.field_of(mode)
	for unit in units:
		if not ids.has(int(unit["id"])) or not Array(unit.get("modes", [])).has(mode):
			continue
		able.append(int(unit["id"]))
		enable = enable or not bool(unit.get(field, false))
	if able.is_empty():
		return {}
	return {"type": "set_mode", "units": able, "mode": mode, "enabled": enable}


## CB4 : Alt+`slot` : la capacité n° `slot` de la première unité sélectionnée qui en a autant,
## employée (ou levée) par toutes les unités sélectionnées qui l'ont ; message sinon. Rien
## pendant un rejeu.
func use_ability_slot(slot: int) -> void:
	if scene.replay_mode:
		return
	var command := ability_command(scene.units, _available_selection(), slot)
	if command.is_empty():
		scene.hud.show_toast("Aucune unité sélectionnée n'a de capacité en Alt+%d." % slot)
		return
	command_requested.emit(command)


## CB4 : bouton de capacité d'une carte : cette unité seule.
func use_card_ability(unit_id: int, ability: String) -> void:
	if scene.replay_mode:
		return
	command_requested.emit({"type": "use_ability", "units": [unit_id], "ability": ability})


## CB4 : l'ordre `use_ability` d'Alt+`slot` pour les unités `ids` (dans l'ordre de la sélection ;
## fonction pure, testée) : la capacité n° `slot` de la première qui en a autant, pour toutes
## celles de `ids` qui l'ont ; vide si aucune.
static func ability_command(units: Array, ids: Array, slot: int) -> Dictionary:
	var by_id := {}
	for unit in units:
		by_id[int(unit["id"])] = unit
	var ability := ""
	for id in ids:
		var abilities := Array((by_id.get(int(id), {}) as Dictionary).get("abilities", []))
		if abilities.size() >= slot:
			ability = str((abilities[slot - 1] as Dictionary).get("id", ""))
			break
	if ability == "":
		return {}
	var having: Array[int] = []
	for id in ids:
		var abilities := Array((by_id.get(int(id), {}) as Dictionary).get("abilities", []))
		if BattleAbilityIcons.slot_of(abilities, ability) > 0:
			having.append(int(id))
	return {"type": "use_ability", "units": having, "ability": ability}


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


## CB0 : Ctrl/Cmd+A = toutes les troupes du joueur présentes, hors déroute.
func _select_all_player_units() -> void:
	var ids: Array[int] = []
	for unit in scene.units:
		if str(unit["side"]) == scene.player_side and bool(unit["present"]) and str(unit["state"]) != "routing":
			ids.append(int(unit["id"]))
	scene.selected = ids
	selection_changed.emit(scene.selected)


## CB0 : double clic (gauche sur le terrain, ou carte via `BattleScene._on_card_double_clicked`)
## = même `type` parmi les troupes présentes du joueur.
func select_same_type_of(unit_id: int) -> void:
	var kind := ""
	for unit in scene.units:
		if int(unit["id"]) == unit_id:
			kind = str(unit["type"])
			break
	if kind == "":
		return
	var ids: Array[int] = []
	for unit in scene.units:
		if str(unit["side"]) == scene.player_side and bool(unit["present"]) and str(unit["type"]) == kind:
			ids.append(int(unit["id"]))
	scene.selected = ids
	selection_changed.emit(scene.selected)
