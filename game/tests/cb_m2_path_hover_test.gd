extends SceneTree

## CB-M2 : aperçu du trajet et curseur contextuel.
## 1. Curseurs : une image 32 px par contexte (fichier DA5 ou substitut), `none` = flèche.
## 2. Étranglement de l'aperçu : pas de recalcul sous 5 m ni plus de 10 fois par seconde
##    (valeurs de `data/rules/battle_hover.json` via RuleValues), compteur d'appels au cœur.
## 3. Intégration sur la démo autonome (`battle.tscn`) : clic droit maintenu = trajet et fantôme,
##    rafale de mouvements = un seul appel, ordre refusé sans chemin, trajet et fantôme gardés
##    après l'ordre, flèche d'attaque, curseur `move` / attaque selon le survol (un appel au cœur
##    par changement), survol d'une carte du HUD = contour pâle.
##
## Usage : godot --headless --path game --script res://tests/cb_m2_path_hover_test.gd

var _failures := 0
var _scene: Node = null


func _init() -> void:
	await process_frame
	_check_cursors()
	await _check_integration()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	print("cb_m2_path_hover_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("cb_m2_path_hover_test: " + message)
	return condition


func _check_cursors() -> void:
	for context in BattleCursor.CONTEXTS:
		var image := BattleCursor.image(context)
		if not _check(image != null, "no cursor image for %s" % context):
			continue
		_check(image.get_size() == Vector2i(32, 32), "%s cursor is 32 px" % context)
		var opaque := 0
		for y in 32:
			for x in 32:
				if image.get_pixel(x, y).a > 0.5:
					opaque += 1
		_check(opaque > 40, "%s cursor has a visible shape (%d px)" % [context, opaque])
	_check(BattleCursor.image("none") == null, "none = system arrow")
	_check(BattleCursor.image("melee") != BattleCursor.image("move"), "distinct cursors")
	var cursor := BattleCursor.new()
	cursor.apply("move")
	cursor.apply("move")
	cursor.apply("bogus")
	_check(cursor.changes == 2 and cursor.context == "none", "cursor set only on change, unknown = none")


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
	_scene.battle.call("set_ai", _scene.player_side, false)
	_scene.paused = true
	var preview: BattlePathPreview = _scene.path_preview
	if not _check(preview != null and _scene.cursor != null, "no path preview / cursor node"):
		return
	_check_throttle(preview)

	var own := -1
	var spare := -1
	var foe := -1
	for unit in _scene.units:
		if not bool(unit["present"]):
			continue
		if str(unit["side"]) == _scene.player_side:
			if own < 0:
				own = int(unit["id"])
			elif spare < 0:
				spare = int(unit["id"])
		elif foe < 0:
			foe = int(unit["id"])
	if not _check(own >= 0 and spare >= 0 and foe >= 0, "no present regiments"):
		return
	_scene.selected.assign([own])
	var unit := _unit(own)
	var goal := Vector3(float(unit["x"]) + 40.0, 0.0, float(unit["z"]) + 25.0)
	_scene.camera_rig.look_at_point(goal, 260.0, 0.0)
	for _i in 3:
		await process_frame
	var camera: Camera3D = _scene.camera_rig.camera
	var screen := camera.unproject_position(Vector3(goal.x, _scene.terrain.height_at(goal.x, goal.z), goal.z))

	# Clic droit maintenu : trajet en direct et fantôme ; une rafale de mouvements dans la même
	# image ne coûte qu'un appel au cœur.
	var before := preview.recompute_count
	_mouse_button(screen, MOUSE_BUTTON_RIGHT, true)
	for k in 60:
		_mouse_motion(screen + Vector2(k % 3, 0))
	_check(preview.live_active and preview.live_mesh().visible, "live path shown while the right button is held")
	_check(preview.ghost_count() >= 1, "arrival ghost shown")
	_check(preview.recompute_count - before <= 2, "throttled: %d core calls for 60 motions" % (preview.recompute_count - before))
	_check(preview.reachable(), "open ground reachable")
	_scene.issued_log.clear()
	_mouse_button(screen, MOUSE_BUTTON_RIGHT, false)
	_check(not preview.live_active and not preview.live_mesh().visible, "live path cleared on release")
	_check(_scene.issued_log.size() == 1 and str(_scene.issued_log[0]["type"]) == "move", "move sent: %s" % [_scene.issued_log])

	# Après l'ordre : trajet et fantôme tant que le régiment reste sélectionné.
	_scene.battle.call("tick", 0.2)
	_scene._refresh_view(true)
	preview._orders_key = ""
	preview.update_orders(_scene.units, _scene.selected, 1000.0)
	_check(preview.orders_mesh().visible and preview.ghost_count() >= 1, "ordered path and ghost kept while selected")
	_scene.selected.clear()
	preview.update_orders(_scene.units, _scene.selected, 1001.0)
	_check(not preview.orders_mesh().visible and preview.ghost_count() == 0, "nothing once deselected")

	# Destination sans chemin (hors du champ) : pas d'ordre, curseur interdit.
	_scene.selected.assign([own])
	_scene.issued_log.clear()
	var allowed: bool = _scene.input._path_allowed(Vector3(-400.0, 0.0, -400.0), NAN)
	_check(not allowed and _scene.issued_log.is_empty(), "no order without a path")
	_check(not preview.reachable() and preview.refusal() != "", "refusal reason given")
	_scene.note_mouse(screen)
	_scene._update_hover_cursor()
	_check(_scene.cursor.context == "forbidden", "forbidden cursor while the preview has no path (got %s)" % _scene.cursor.context)
	preview.clear_live()

	# Curseur : sol = move ; ennemi = une forme d'attaque ; un seul appel par changement.
	_scene.markers.world_hover = -1
	_scene.note_mouse(screen + Vector2(1, 0))
	_scene._update_hover_cursor()
	_check(_scene.cursor.context == "move", "ground cursor move (got %s, %s)" % [_scene.cursor.context, _scene.last_hover])
	var calls: int = _scene.hover_calls
	_scene.note_mouse(screen + Vector2(2, 0))
	_scene._update_hover_cursor()
	_check(_scene.hover_calls == calls, "same 2 m cell: no new core call")
	_scene._update_hover_cursor()
	_check(_scene.hover_calls == calls, "no mouse motion: no core call")
	_scene.markers.world_hover = foe
	_scene.note_mouse(screen + Vector2(3, 0))
	_scene._update_hover_cursor()
	var attack := ["melee", "ranged", "ranged_blocked"]
	_check(attack.has(_scene.cursor.context), "enemy cursor (got %s)" % _scene.cursor.context)
	_check(int(_scene.last_hover.get("target", -1)) == foe, "hovered enemy named")
	_check(_scene.last_hover.has("compare"), "one regiment selected: comparison given")
	var compare: Dictionary = _scene.last_hover.get("compare", {})
	_check((compare.get("advantages", {}) as Dictionary).size() == 9, "nine compared lines")
	_scene.markers.world_hover = spare
	_scene.note_mouse(screen + Vector2(4, 0))
	_scene._update_hover_cursor()
	_check(_scene.cursor.context == "none", "friend: no action cursor (got %s)" % _scene.cursor.context)
	_scene.markers.world_hover = -1

	# Attaque : flèche vers la cible tant que l'attaquant reste sélectionné.
	_scene.issue({"type": "attack", "units": [own], "target": foe, "run": false})
	_scene.battle.call("tick", 0.1)
	_scene._refresh_view(true)
	preview._orders_key = ""
	preview.update_orders(_scene.units, _scene.selected, 2000.0)
	_check(preview.orders_mesh().visible, "attack arrow shown")

	# Carte du HUD survolée : contour pâle sur le régiment (non sélectionné).
	_scene.hud.card_hovered.emit(spare)
	_scene._refresh_view(true)
	_check(_scene.outlines.state_of(spare) == BattleFormationOutline.State.HOVERED, "card hover lights the outline")
	_scene.hud.card_hovered.emit(-1)
	_scene._refresh_view(true)
	_check(_scene.outlines.state_of(spare) == BattleFormationOutline.State.NONE, "card hover cleared")


func _check_throttle(preview: BattlePathPreview) -> void:
	_check(is_equal_approx(preview.recompute_distance, RuleValues.value("hover_preview_recompute_m", -1.0)), "recompute distance from RuleValues")
	_check(is_equal_approx(preview.min_interval, 1.0 / RuleValues.value("hover_preview_max_per_s", -1.0)), "rate from RuleValues")
	preview.clear_live()
	var ids: Array = [int(_scene.units[0]["id"])]
	var p := Vector3(float(_scene.units[0]["x"]) + 30.0, 0.0, float(_scene.units[0]["z"]))
	var count := preview.recompute_count
	_check(preview.request(ids, _scene.units, p, NAN, 100.0), "first request computes")
	_check(not preview.request(ids, _scene.units, p + Vector3(20, 0, 0), NAN, 100.05), "not twice within 0.1 s")
	_check(not preview.request(ids, _scene.units, p + Vector3(3, 0, 0), NAN, 100.5), "not for a 3 m move")
	_check(preview.request(ids, _scene.units, p + Vector3(8, 0, 0), NAN, 100.6), "a 8 m move recomputes")
	_check(preview.request(ids, _scene.units, p + Vector3(8, 0, 0), 1.0, 100.8), "a new facing recomputes")
	_check(preview.recompute_count - count == 3, "three core calls counted")
	preview.clear_live()


func _unit(id: int) -> Dictionary:
	for unit in _scene.battle.call("get_units"):
		if int(unit["id"]) == id:
			return unit
	return {}


func _mouse_button(pos: Vector2, button: int, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.button_index = button
	event.pressed = pressed
	event.position = pos
	event.global_position = pos
	_scene.get_viewport().push_input(event)


func _mouse_motion(pos: Vector2) -> void:
	var event := InputEventMouseMotion.new()
	event.position = pos
	event.global_position = pos
	_scene.get_viewport().push_input(event)
