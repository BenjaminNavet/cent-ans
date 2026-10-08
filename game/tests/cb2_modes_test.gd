extends SceneTree

## CB2 : modes d'unité, pastilles d'état, remappage des touches.
## 1. Fonctions pures : table unique `BattleHotkeys` (aiguillage F/T/R/G/K/M, Ctrl+G ; pas de
##    doublon ; libellés ; aide F1 sans les lots en attente), `BattleInput.mode_command`
##    (bascule sur les seuls régiments qui peuvent prendre le mode), `BattleHud.mode_button_state`,
##    `BattleUnitMarkers.state_badges` (priorité déroute > hésite > sous le feu > charge > mode,
##    trois au plus).
## 2. Intégration sur la démo autonome (`battle.tscn`) : `get_units` porte les modes et états du
##    cœur ; les touches K, G, R, F, T envoient `set_mode`, `fire_at_will` et `formation` ; le cœur
##    prend les modes ; boutons de mode dans la barre d'ordres ; aide F1 générée par la table.
##
## Usage : godot --headless --path game --script res://tests/cb2_modes_test.gd

var _failures := 0
var _scene: Node = null


func _init() -> void:
	await process_frame
	_check_hotkeys()
	_check_pure()
	await _check_integration()
	if _scene != null and is_instance_valid(_scene):
		_scene.queue_free()
	print("cb2_modes_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("cb2_modes_test: " + message)
	return condition


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
	var expected := {KEY_F: "fire_at_will", KEY_T: "formation", KEY_R: "run", KEY_G: "guard", KEY_K: "skirmish", KEY_M: "melee"}
	for code in expected:
		_check(BattleHotkeys.action_for(_key(code)) == expected[code], "%s -> %s" % [OS.get_keycode_string(code), expected[code]])
	_check(BattleHotkeys.action_for(_key(KEY_G, true)) == "lock_group", "Ctrl+G locks the group")
	_check(BattleHotkeys.action_for(_key(KEY_G, false, true)) == "", "Alt+G is not guard (Alt: abilities)")
	_check(BattleHotkeys.action_for(_key(KEY_H)) == "", "H stays with the battle input match")
	_check(BattleHotkeys.action_for(_key(KEY_1, false, true, true)) == "", "Alt+Shift+1 left to CB6")
	_check(BattleHotkeys.key_label("guard") == "G" and BattleHotkeys.key_label("lock_group") == "Ctrl+G", "labels (%s, %s)" % [BattleHotkeys.key_label("guard"), BattleHotkeys.key_label("lock_group")])
	_check(BattleHotkeys.key_label("fire_at_will") == "F" and BattleHotkeys.key_label("formation") == "T", "remapped F and T")
	# Aucune touche (avec ses modificateurs) n'est prise deux fois.
	var seen := {}
	for row in BattleHotkeys.BINDINGS:
		if not row.has("key"):
			continue
		var slot := "%d/%s/%s" % [int(row["key"]), str(row.get("mods", "")), str(row.get("physical", false))]
		_check(not seen.has(slot), "key taken twice: %s (%s, %s)" % [slot, seen.get(slot, ""), row["action"]])
		seen[slot] = row["action"]
	var help := BattleHotkeys.help_bbcode({"orders": "Z X V B N"})
	for needle in ["[b]F[/b] tir à volonté", "[b]T[/b] changer de formation", "[b]K[/b] escarmouche", "[b]Ctrl+G[/b]", "Alt+Maj+1…6", "Z X V B N", "Tab"]:
		_check(help.contains(needle), "help mentions %s" % needle)
	_check(help.contains("Alt+1…4"), "CB4 abilities in the help once merged")


func _check_pure() -> void:
	var units := [
		{"id": 1, "modes": PackedStringArray(["run", "guard", "skirmish", "melee"]), "skirmish": false, "guard": true},
		{"id": 2, "modes": PackedStringArray(["run", "guard"]), "guard": true},
		{"id": 3, "modes": PackedStringArray(["run", "guard", "skirmish", "melee"]), "skirmish": true},
	]
	var command := BattleInput.mode_command(units, [1, 2], "skirmish")
	_check(command == {"type": "set_mode", "units": [1], "mode": "skirmish", "enabled": true}, "skirmish for the shooters only (%s)" % command)
	command = BattleInput.mode_command(units, [1, 2], "guard")
	_check(command.get("enabled", true) == false and (command["units"] as Array).size() == 2, "all on guard: toggled off (%s)" % command)
	command = BattleInput.mode_command(units, [1, 3], "skirmish")
	_check(bool(command.get("enabled", false)), "one off: turned on for all (%s)" % command)
	_check(BattleInput.mode_command(units, [2], "melee").is_empty(), "no one can: no order")

	var state := BattleHud.mode_button_state(units, [1, 2])
	_check(bool(state["guard"]["on"]) and bool(state["guard"]["able"]), "guard shown on")
	_check(not bool(state["skirmish"]["on"]) and bool(state["skirmish"]["able"]), "skirmish available, off")
	_check(not bool(state["breach"]["able"]), "no engine: breach greyed")

	# RJ-a : état on/off des boutons d'ordre, cycle T, menu et infobulle des formations.
	var orders := [
		{"id": 1, "present": true, "can_shoot": true, "fire_at_will": true, "formation": "herse", "formation_short": "Herse", "formations": PackedStringArray(["line", "column", "thin_line", "herse"]), "reforming": true, "reform_progress": 0.4},
		{"id": 2, "present": true, "can_shoot": false, "fire_at_will": false, "formation": "line", "formation_short": "Ligne", "formations": PackedStringArray(["line", "column", "square"])},
		{"id": 3, "present": true, "can_shoot": true, "fire_at_will": false, "formation": "line", "formation_short": "Ligne", "formations": PackedStringArray(["line", "column", "herse"])},
	]
	var buttons := BattleHud.command_button_state(orders, [1, 2])
	_check(bool(buttons["fire_at_will"]["on"]) and bool(buttons["fire_at_will"]["able"]), "fire at will shown on (every chosen shooter)")
	_check(bool(buttons["formation"]["on"]) and is_equal_approx(float(buttons["formation"]["progress"]), 0.4), "formation shown on while reforming (%s)" % [buttons])
	_check(str(buttons["formation"]["label"]) == "", "mixed formations: no common label")
	buttons = BattleHud.command_button_state(orders, [2, 3])
	_check(not bool(buttons["fire_at_will"]["on"]) and not bool(buttons["formation"]["on"]), "held fire, default line: both off (%s)" % [buttons])
	_check(str(buttons["formation"]["label"]) == "Ligne", "common formation label")
	_check(not bool(BattleHud.command_button_state(orders, [2])["fire_at_will"]["able"]), "no shooter: fire at will greyed")
	_check(BattleInput.next_formation(orders[1]) == "column" and BattleInput.next_formation(orders[0]) == "line", "T cycles the allowed formations")
	_check(BattleInput.next_formation({"formation": "line", "formations": PackedStringArray(["line"])}) == "", "a single formation: no order")
	var core: Object = ClassDB.instantiate("BattleSim") if ClassDB.class_exists("BattleSim") else null
	var catalog: Array = core.call("unit_formations") if core != null else []
	_check(catalog.size() >= 8, "core formation catalogue (%d)" % catalog.size())
	var menu := BattleFormationMenu.menu_state(catalog, orders, [1, 3])
	_check(bool(menu.get("herse", {}).get("able", false)) and not bool(menu.get("herse", {}).get("current", true)), "herse able, not common (%s)" % [menu.get("herse")])
	_check(not bool(menu.get("wedge", {}).get("able", true)), "no rider: wedge greyed")
	for entry in catalog:
		if str(entry["key"]) == "square":
			var spec := RichTooltip.spec_for(TooltipHost.tooltip_key("formation", "square", ""), BattleFormationMenu.tooltip_live(entry, core.call("formation_reform_rules")))
			_check(str(spec.get("title", "")) == "Schiltron" and (spec.get("effects", []) as Array).size() >= 4, "schiltron tooltip: name, history and effects (%s)" % [spec.get("effects")])
	if core is Node:
		(core as Node).free()

	var base := {"state": "idle", "fatigue": 0.0}
	var badges := BattleUnitMarkers.state_badges(base.merged({"wavering": true, "under_fire": true, "charging": true, "guard": true}))
	_check(badges == ["wavering", "under_fire", "charge"], "priority and cap of three (%s)" % [badges])
	badges = BattleUnitMarkers.state_badges(base.merged({"under_fire": true, "mode_run": true}))
	_check(badges == ["under_fire", "mode_run"], "mode after the states (%s)" % [badges])
	badges = BattleUnitMarkers.state_badges({"state": "routing", "wavering": true, "guard": true, "fatigue": 0.0})
	_check(badges.size() >= 1 and badges[0] == "rout" and not badges.has("wavering"), "rout first, alone (%s)" % [badges])
	badges = BattleUnitMarkers.state_badges({"state": "melee", "fatigue": 0.0})
	_check(badges == ["melee"], "state fallback without core fields (%s)" % [badges])
	_check(BattleModeIcons.active_modes({"mode_run": true, "breach": true}) == ["run", "breach"], "active modes in button order")


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
	var first: Dictionary = (_scene.battle.call("get_units") as Array)[0]
	if not _check(first.has("modes") and first.has("wavering") and first.has("under_fire"), "bridge without CB2 fields (run core/build.sh)"):
		return
	_scene.battle.call("set_ai", _scene.player_side, false)
	_scene.paused = true
	var shooter := -1
	var foot := -1
	for unit in _scene.battle.call("get_units"):
		if not bool(unit["present"]) or str(unit["side"]) != _scene.player_side:
			continue
		if shooter < 0 and bool(unit["can_shoot"]) and str(unit["category"]) != "siege":
			shooter = int(unit["id"])
		elif foot < 0 and str(unit["category"]) == "infantry":
			foot = int(unit["id"])
	if not _check(shooter >= 0 and foot >= 0, "a shooter and a foot regiment of the player"):
		return
	_check(Array(_unit(shooter)["modes"]).has("skirmish") and not Array(_unit(foot)["modes"]).has("skirmish"), "skirmish for shooters only (core verdict)")

	# K : escarmouche sur la sélection (seul le tireur peut) ; le cœur la prend.
	_scene.selected.assign([shooter, foot])
	_scene.issued_log.clear()
	_press(KEY_K)
	var log: Array = _scene.issued_log
	_check(log.size() == 1 and str(log[0].get("type", "")) == "set_mode" and log[0]["units"] == [shooter] and bool(log[0]["enabled"]), "K sends set_mode skirmish to the shooter (%s)" % [log])
	_scene._refresh_view(true)
	_check(bool(_unit(shooter).get("skirmish", false)), "the core takes skirmish")

	# G : garde pour les deux ; R : course ; F : tir à volonté ; T : formation.
	_scene.issued_log.clear()
	_press(KEY_G)
	_press(KEY_R)
	_press(KEY_F)
	_press(KEY_T)
	log = _scene.issued_log
	var types: Array = []
	for entry in log:
		types.append("%s:%s" % [entry.get("type", ""), entry.get("mode", entry.get("kind", ""))])
	_check(types.size() >= 4 and types[0] == "set_mode:guard" and types[1] == "set_mode:run", "G then R toggle guard and run (%s)" % [types])
	_check(types.has("fire_at_will:"), "F is fire at will now (%s)" % [types])
	var formation := false
	for t in types:
		formation = formation or str(t).begins_with("formation:")
	_check(formation, "T cycles the formation (%s)" % [types])
	_scene._refresh_view(true)
	_check(bool(_unit(foot).get("guard", false)) and bool(_unit(foot).get("mode_run", false)), "the core takes guard and run")

	# Barre d'ordres : boutons de mode, état de la sélection.
	_scene.hud.update_cards(_scene.units, _scene.player_side, _scene.selected)
	var guard_button: Button = _scene.hud._mode_buttons.get("guard")
	var breach_button: Button = _scene.hud._mode_buttons.get("breach")
	_check(guard_button != null and not guard_button.disabled, "guard button enabled for the selection")
	_check(breach_button != null and breach_button.disabled, "breach greyed outside a siege")
	_check(bool(_scene.hud._mode_state.get("guard", {}).get("on", false)), "guard button shown on")
	_check(guard_button != null and str(guard_button.tooltip_text).contains("(G)"), "guard tooltip gives its key")

	# Aide F1 : générée par la table.
	var help := ""
	for child in _scene.hud.help_panel.get_children():
		if child is RichTextLabel:
			help = (child as RichTextLabel).get_parsed_text()
	_check(help.contains("escarmouche") and help.contains("Ctrl+G") and help.contains("tir à volonté"), "F1 help lists the new keys")


func _unit(id: int) -> Dictionary:
	for unit in _scene.battle.call("get_units"):
		if int(unit["id"]) == id:
			return unit
	return {}


func _press(code: Key) -> void:
	for pressed in [true, false]:
		var event := _key(code)
		event.pressed = pressed
		_scene.get_viewport().push_input(event)
