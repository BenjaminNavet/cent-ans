extends TestCase

## CB4 : capacités actives des régiments.
## 1. Fonctions pures : `BattleHotkeys.ability_slot` (Alt/Option+1…4 seulement), aide F1 qui cite
##    Alt+1…4, `BattleInput.ability_command` (capacité n° n de la première unité, pour toutes
##    celles qui l'ont), `BattleAbilityIcons` (infobulle chiffrée, état, cadran de recharge).
## 2. Intégration sur la démo autonome (`battle.tscn`) : `get_units` porte `abilities`, le
##    catalogue ses textes ; boutons sous les cartes ; Alt+1 envoie `use_ability` et le cœur la
##    prend ; un clic de bouton de carte aussi ; après un mouvement, le bouton est grisé et
##    l'infobulle donne la raison (recharge).
##
## Usage : godot --headless --path game --script res://tests/cb4_abilities_test.gd

var _scene: Node = null


func _init() -> void:
	await process_frame
	_check_hotkeys()
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


func _check_hotkeys() -> void:
	check(BattleHotkeys.ability_slot(_key(KEY_1, false, true)) == 1, "Alt+1 = first ability")
	check(BattleHotkeys.ability_slot(_key(KEY_4, false, true)) == 4, "Alt+4 = fourth ability")
	check(BattleHotkeys.ability_slot(_key(KEY_5, false, true)) == 0, "Alt+5 is no ability")
	check(BattleHotkeys.ability_slot(_key(KEY_1)) == 0, "1 stays the selection group")
	check(BattleHotkeys.ability_slot(_key(KEY_1, true)) == 0, "Ctrl+1 stays the group save")
	check(BattleHotkeys.ability_slot(_key(KEY_1, false, true, true)) == 0, "Alt+Shift+1 left to CB6")
	for row in BattleHotkeys.BINDINGS:
		check(not bool(row.get("pending", false)), "no pending row left (%s)" % row["action"])
	var help := BattleHotkeys.help_bbcode({"orders": "Z X V B"})
	check(help.contains("[b]Alt+1…4[/b] capacités"), "F1 help lists Alt+1…4")


func _check_pure() -> void:
	var pavise := {"id": "ability_pavise", "kind": "pavise", "available": true, "active": false}
	var pikes := {"id": "ability_planted_pikes", "kind": "planted_pikes", "available": true, "active": false}
	var close := {"id": "ability_close_ranks", "kind": "close_ranks", "available": true, "active": false}
	var units := [
		{"id": 1, "abilities": [pavise]},
		{"id": 2, "abilities": []},
		{"id": 3, "abilities": [pikes, close]},
		{"id": 4, "abilities": [close]},
	]
	var command := BattleInput.ability_command(units, [1, 2, 3], 1)
	check(command == {"type": "use_ability", "units": [1], "ability": "ability_pavise"}, "Alt+1: the first unit's first ability, for those that have it (%s)" % command)
	command = BattleInput.ability_command(units, [2, 3, 4], 2)
	check(command.get("ability", "") == "ability_close_ranks" and command["units"] == [3, 4], "Alt+2: close ranks for the militia and the men-at-arms (%s)" % command)
	check(BattleInput.ability_command(units, [2], 1).is_empty(), "no ability: no order")
	check(BattleInput.ability_command(units, [1], 2).is_empty(), "no second ability: no order")
	check(BattleAbilityIcons.slot_of([pikes, close], "ability_close_ranks") == 2, "slot of an ability")

	var entry := {"id": "ability_pavise", "name": "Dresser les pavois", "description": "Les valets plantent les pavois."}
	var tip := BattleAbilityIcons.tip(entry, pavise, 1)
	check(tip.begins_with("[b]Dresser les pavois[/b] (Alt+1)"), "tooltip title and key (%s)" % tip)
	check(tip.contains("Recharge :") and tip.contains("Prête."), "tooltip timing and state (%s)" % tip)
	var greyed := {"id": "ability_pavise", "available": false, "active": false, "reason": "recharge, encore 7 s", "cooldown": 10.0, "cooldown_remaining": 7.0}
	check(BattleAbilityIcons.tip(entry, greyed, 1).contains("Indisponible : recharge, encore 7 s."), "greyed tooltip gives the reason")
	check(is_equal_approx(BattleAbilityIcons.cooldown_fraction(greyed), 0.7), "cooldown dial fraction")
	check(BattleAbilityIcons.status_text({"active": true, "setup_remaining": 2.5}).contains("mise en place : 3 s"), "setup time shown")
	check(BattleAbilityIcons.status_text({"available": true, "ended_reason": "pavois levés"}).contains("pavois levés"), "last end reason shown")


func _check_integration() -> void:
	root.size = Vector2i(1440, 900)  # headless : la fenêtre par défaut est minuscule (64x64)
	_scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	_scene.autoplay = true
	_scene.log_orders_for_test = true
	root.add_child(_scene)
	for _i in 6:
		await process_frame
	if not check(_scene.battle != null and (_scene.units as Array).size() > 0, "battle demo failed to stage (run core/build.sh?)"):
		return
	if not check(_scene.battle.has_method("get_ability_catalog") and _scene.battle.has_method("use_ability"), "bridge without CB4 (run core/build.sh)"):
		return
	var catalog: Dictionary = _scene.battle.call("get_ability_catalog")
	check(catalog.size() == 5 and catalog.has("ability_pavise"), "catalogue of five abilities (%s)" % [catalog.keys()])
	check(_scene.hud.ability_catalog.size() == 5, "the HUD holds the catalogue")
	_scene.battle.call("set_ai", _scene.player_side, false)
	_scene.paused = true
	var user := -1
	var plain := -1
	for unit in _scene.battle.call("get_units"):
		if not bool(unit["present"]) or str(unit["side"]) != _scene.player_side:
			continue
		var abilities := Array(unit.get("abilities", []))
		if user < 0 and not abilities.is_empty() and bool(abilities[0]["available"]):
			user = int(unit["id"])
		elif plain < 0 and abilities.is_empty():
			plain = int(unit["id"])
	if not check(user >= 0, "a player regiment with an ability"):
		return
	var first: Dictionary = Array(_unit(user)["abilities"])[0]
	var ability := str(first["id"])
	for key in ["id", "kind", "available", "reason", "active", "effective", "remaining", "cooldown", "cooldown_remaining", "ended_reason"]:
		check(first.has(key), "ability state has %s" % key)

	# Boutons sous la carte.
	_scene._refresh_view(true)
	_scene.hud.update_cards(_scene.units, _scene.player_side, _scene.selected)
	var card: UnitCard = _scene.hud._cards.get(user)
	if not check(card != null, "card of the regiment"):
		return
	var buttons := card.ability_buttons()
	check(buttons.size() == Array(_unit(user)["abilities"]).size(), "one button per ability (%d)" % buttons.size())
	check(buttons.size() > 0 and not buttons[0].disabled and str(buttons[0].tooltip_text).contains("(Alt+1)"), "enabled button, tooltip with its key")
	check(buttons.size() > 0 and str(buttons[0].tooltip_text).contains(str(catalog[ability]["name"])), "tooltip names the ability")
	check(card.custom_minimum_size.y == UnitCard.HEIGHT and UnitCard.HEIGHT + 30.0 <= BattleHud.BAND_HEIGHT + 2.0, "card and band heights")
	if plain >= 0 and _scene.hud._cards.has(plain):
		check((_scene.hud._cards[plain] as UnitCard).ability_buttons().is_empty(), "no button without ability")

	# Alt+1 sur la sélection : `use_ability`, le cœur la prend.
	_scene.selected.assign([user] + ([plain] if plain >= 0 else []))
	_scene.issued_log.clear()
	_press(_key(KEY_1, false, true))
	var log: Array = _scene.issued_log
	check(log.size() == 1 and str(log[0].get("type", "")) == "use_ability" and log[0]["units"] == [user] and str(log[0]["ability"]) == ability, "Alt+1 sends use_ability for the regiment that has it (%s)" % [log])
	check(_scene.selected.has(user), "Alt+1 does not recall a selection group")
	_scene._refresh_view(true)
	check(bool(Array(_unit(user)["abilities"])[0]["active"]), "the core takes the ability")
	_scene.hud.update_cards(_scene.units, _scene.player_side, _scene.selected)
	check(bool((buttons[0].get_meta("state", {}) as Dictionary).get("active", false)), "the button shows it active")

	# Clic du bouton : la lève (second emploi) ; puis grisé en recharge, raison dans l'infobulle.
	_scene.issued_log.clear()
	buttons[0].pressed.emit()
	log = _scene.issued_log
	check(log.size() == 1 and log[0] == {"type": "use_ability", "units": [user], "ability": ability}, "card button sends use_ability for its regiment (%s)" % [log])
	_scene._refresh_view(true)
	_scene.hud.update_cards(_scene.units, _scene.player_side, _scene.selected)
	var state: Dictionary = Array(_unit(user)["abilities"])[0]
	check(not bool(state["active"]) and not bool(state["available"]), "lifted, then recharging (%s)" % state)
	check(buttons[0].disabled, "greyed while it recharges")
	check(str(buttons[0].tooltip_text).contains("Indisponible : recharge"), "the tooltip gives the reason (%s)" % buttons[0].tooltip_text)

	# Aide F1 : générée par la table, avec les capacités.
	var help := ""
	for child in _scene.hud.help_panel.get_children():
		if child is RichTextLabel:
			help = (child as RichTextLabel).get_parsed_text()
	check(help.contains("Alt+1…4") and help.contains("capacités"), "F1 help lists the abilities")


func _unit(id: int) -> Dictionary:
	for unit in _scene.battle.call("get_units"):
		if int(unit["id"]) == id:
			return unit
	return {}


func _press(event: InputEventKey) -> void:
	for pressed in [true, false]:
		var copy := event.duplicate() as InputEventKey
		copy.pressed = pressed
		_scene.get_viewport().push_input(copy)
