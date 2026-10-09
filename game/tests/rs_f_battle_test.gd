extends TestCase

## RS-F : ordre « Incendier » et sélecteur de formations repliable.
## 1. Fonctions pures : `LeaderOrdersBar.burn_view` (vue d'ordre, raison, cible), raccourci I dans
##    la table des touches.
## 2. Démo autonome (`battle.tscn`, bataille rangée) : pas de bouton « Incendier » ; le sélecteur
##    CB6 se replie par son en-tête (corps caché, préréglage actif rappelé dans l'en-tête, raccourci
##    Alt+Maj+n toujours actif), l'état tient pendant la bataille, puis se déplie.
## 3. Siège de la Guyenne : bouton « Incendier » en bout de barre des ordres, grisé tant qu'aucun
##    régiment n'est à portée (raison du cœur dans l'infobulle) ; un régiment mené au pied d'une
##    maison du faubourg rend l'ordre possible ; clic puis touche I envoient la commande `burn`.
##
## Usage : godot --headless --path game --script res://tests/rs_f_battle_test.gd

const BAR := preload("res://scripts/battle/leader_orders_bar.gd")

var _scene: Node = null


func _init() -> void:
	await process_frame
	# Réglages par défaut : la taille d'interface du joueur changerait l'échelle de la fenêtre.
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("_apply_ui_scale")
	_check_pure()
	await _check_picker()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
		_scene = null
		await process_frame
	await _check_burn()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	finish()


func _key(code: Key, alt: bool = false, shift: bool = false) -> InputEventKey:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.alt_pressed = alt
	event.shift_pressed = shift
	event.pressed = true
	return event


func _check_pure() -> void:
	var off: Dictionary = BAR.burn_view({"siege": true, "available": false, "reason": "aucune maison à portée"})
	check(str(off["id"]) == "burn" and str(off["name"]) == "Incendier", "burn view: id and name")
	check(not bool(off["available"]) and str(off["reason"]) == "aucune maison à portée", "burn view: unavailable with the core's reason")
	var on: Dictionary = BAR.burn_view({"siege": true, "available": true, "target": "la porte", "distance_m": 4.4})
	check(bool(on["available"]) and str(on["description"]).contains("Cible : la porte, à 4 m"), "burn view: target named (%s)" % on["description"])
	check(BattleHotkeys.key_label("burn") == "I", "burn shortcut in the key table (%s)" % BattleHotkeys.key_label("burn"))
	check(BattleHotkeys.help_bbcode({"orders": "Z X V B"}).contains("incendier"), "burn line in the F1 help")


# ----- sélecteur de formations repliable ----------------------------------------------------

func _check_picker() -> void:
	root.size = Vector2i(1440, 900)
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.autoplay = true
	_scene.paused = true
	root.add_child(_scene)
	for _i in 6:
		await process_frame
	if not check(_scene.battle != null and (_scene.units as Array).size() > 0, "battle demo failed to stage (run core/build.sh?)"):
		return
	var bar: CanvasLayer = _scene._leader_bar
	if check(bar != null, "leader orders bar"):
		bar.refresh()
		check(not bool(bar.burn_order.get("siege", true)), "field battle: no siege fire (%s)" % bar.burn_order)
		check(not bar._buttons.has("burn"), "field battle: no burn button")
	var picker: BattleFormationPicker = _scene.formation_picker
	if not check(picker != null and picker.visible and picker.header != null and picker.body != null, "picker with a header"):
		return
	check(not picker.collapsed and picker.body.visible and picker.header.text.begins_with("▾"), "expanded at first (%s)" % picker.header.text)
	var tall := picker.get_combined_minimum_size().y
	picker.header.pressed.emit()
	await process_frame
	check(picker.collapsed and not picker.body.visible, "the header collapses the picker")
	check(picker.header.text.begins_with("▸"), "collapsed arrow (%s)" % picker.header.text)
	check(picker.get_combined_minimum_size().y < tall * 0.6, "collapsed picker is short (%.0f < %.0f)" % [picker.get_combined_minimum_size().y, tall])
	var rect := picker.get_global_rect()
	check(rect.end.x <= 1440.0 and rect.end.y <= 900.0 - BattleHud.BAND_HEIGHT, "collapsed picker stays in its corner (%s)" % rect)
	check(rect.size.y < 60.0, "collapsed picker shrinks to its header (%s)" % rect)
	# Replié, le raccourci garde son effet et l'en-tête rappelle le préréglage.
	_scene.get_viewport().push_input(_key(KEY_2, true, true))
	check(picker.active_id != "" and picker.header.text.contains(picker.name_of(picker.active_id)), "active preset shown in the header (%s)" % picker.header.text)
	_scene.paused = false
	for _i in 10:
		await process_frame
	check(picker.collapsed and not picker.body.visible, "state kept during the battle")
	picker.toggle(picker.active_id)
	picker.header.pressed.emit()
	await process_frame
	check(not picker.collapsed and picker.body.visible and picker.header.text.begins_with("▾"), "the header expands it again")
	await process_frame
	rect = picker.get_global_rect()
	check(rect.end.x <= 1440.0 and rect.end.y <= 900.0 - BattleHud.BAND_HEIGHT and rect.size.y >= tall - 1.0, "expanded picker back in its corner (%s)" % rect)


# ----- incendier (siège) --------------------------------------------------------------------

func _check_burn() -> void:
	if not check(ClassDB.class_exists("CampaignSim"), "GDExtension missing (run core/build.sh)"):
		return
	var data_dir: String = preload("res://scripts/map/map_paths.gd").default_data_dir()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not check(sim.call("new_campaign", data_dir, "fac_france", 1337), "new_campaign failed"):
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.configure(sim, index, 7)
	_scene.log_orders_for_test = true
	root.add_child(_scene)
	for _i in 20:
		await process_frame
	var battle: Object = _scene.battle
	if not check(battle != null and battle.has_method("get_burn_order"), "bridge without get_burn_order (run core/build.sh)"):
		return
	if battle.call("is_deploying"):
		battle.call("start_battle")
	battle.call("set_ai", "attacker", false)
	battle.call("set_ai", "defender", false)
	var bar: CanvasLayer = _scene._leader_bar
	var side := str(_scene.player_side)
	_scene.selected.clear()
	bar.refresh()
	if not check(bool(bar.burn_order.get("siege", false)) and bar._buttons.has("burn"), "siege: burn button in the orders bar (%s)" % bar.burn_order):
		return
	var entry: Dictionary = bar._buttons["burn"]
	var button: Button = entry["button"]
	check(bar.visible and button.visible, "burn button shown")
	check(str(entry["key"]) == "I", "burn button shortcut I (%s)" % entry["key"])
	check(button.get_index() == button.get_parent().get_child_count() - 1, "burn button at the end of the bar")
	check(button.disabled == not bool(bar.burn_order.get("available", false)), "greyed exactly when the core says no")
	# Un régiment du joueur (hors engins) mené au pied de la maison du faubourg la plus proche.
	var siege: Dictionary = battle.call("get_siege")
	var unit_id := -1
	var from := Vector2.ZERO
	for unit in battle.call("get_units"):
		if str(unit["side"]) == side and str(unit["category"]) != "siege" and bool(unit["present"]):
			unit_id = int(unit["id"])
			from = Vector2(float(unit["x"]), float(unit["z"]))
			break
	if not check(unit_id >= 0, "a regiment of the player"):
		return
	_scene.selected.assign([unit_id])
	bar.refresh()
	if not bool(bar.burn_order.get("available", false)):
		check(button.disabled and str(bar.burn_order.get("reason", "")) != "", "greyed with a reason (%s)" % bar.burn_order)
		check(str(button.order.get("reason", "")) == str(bar.burn_order.get("reason", "")), "reason in the tooltip")
		var best := Vector2.INF
		var radius := 0.0
		for house in siege.get("houses", []):
			if bool(house.get("suburb", false)):
				var p := Vector2(float(house["x"]), float(house["z"]))
				if best == Vector2.INF or from.distance_to(p) < from.distance_to(best):
					best = p
					radius = float(house["radius"])
		if not check(best != Vector2.INF, "suburbs around Guyenne"):
			return
		var goal := best + (from - best).normalized() * (radius + 4.0)
		var moved: Dictionary = battle.call("issue_command", {"type": "move", "units": [unit_id], "x": goal.x, "z": goal.y, "run": true})
		check(bool(moved.get("ok", false)), "move to the suburb: %s" % moved)
		for _i in 6000:
			battle.call("tick", 0.1)
			if _i % 50 == 0 and bool(battle.call("get_burn_order", side, PackedInt32Array([unit_id])).get("available", false)):
				break
		bar.refresh()
	if not check(bool(bar.burn_order.get("available", false)), "burn possible at the foot of a house (%s)" % bar.burn_order):
		return
	check(not button.disabled, "burn button enabled")
	check(int(bar.burn_order.get("unit", -1)) == unit_id, "the selected regiment carries the torch")
	_scene.issued_log.clear()
	button.pressed.emit()
	check(_scene.issued_log.size() == 1 and str(_scene.issued_log[0].get("type", "")) == "burn", "click sends a burn command (%s)" % str(_scene.issued_log))
	# Touche I : même commande (vers une autre cible si la première brûle déjà).
	bar.refresh()
	var before: int = _scene.issued_log.size()
	bar._unhandled_input(_key(KEY_I))
	if bool(bar.burn_order.get("available", false)):
		check(_scene.issued_log.size() == before + 1, "key I sends the burn command")
	else:
		check(_scene.issued_log.size() == before, "key I with nothing in reach sends nothing")
	print("rs_f_battle_test: burn order %s" % str(bar.burn_order))
