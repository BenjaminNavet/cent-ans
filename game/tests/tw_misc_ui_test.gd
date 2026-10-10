extends TestCase

## TW misc-ui : raccourcis de bataille reconfigurables (BattleHotkeys), compositions nommées de
## la bataille personnalisée, carte du site dans l'avant-bataille.
## Usage : godot --headless --path game --script res://tests/tw_misc_ui_test.gd


func _key(code: Key, ctrl: bool = false, shift: bool = false) -> InputEventKey:
	var event := InputEventKey.new()
	event.keycode = code
	event.physical_keycode = code
	event.ctrl_pressed = ctrl
	event.shift_pressed = shift
	event.pressed = true
	return event


func _init() -> void:
	await process_frame
	_check_hotkeys()
	_check_presets()
	await _check_menu()
	if ClassDB.class_exists("BattleSim"):
		await _check_custom_screen()
		await _check_site_map()
	finish()


func _check_hotkeys() -> void:
	BattleHotkeys.overrides = {}
	check(BattleHotkeys.action_for(_key(KEY_T)) == "formation", "T = formation by default")
	check(BattleHotkeys.action_for(_key(KEY_H)) == "halt" and BattleHotkeys.action_for(_key(KEY_P)) == "pursue", "H halt, P pursue dispatched through the table")
	check(BattleHotkeys.action_for(_key(KEY_A, true)) == "select_all", "Ctrl+A select_all")
	var row := BattleHotkeys.row_of("formation")
	var binding := BattleHotkeys.capture(row, _key(KEY_Y))
	check(BattleHotkeys.rebind("formation", binding) == "", "rebind T -> Y without conflict")
	check(BattleHotkeys.action_for(_key(KEY_Y)) == "formation" and BattleHotkeys.action_for(_key(KEY_T)) == "", "Y now formation, T free")
	check(BattleHotkeys.key_label("formation") == "Y", "label follows the override: %s" % BattleHotkeys.key_label("formation"))
	# Conflit : H (halte) pris par la formation -> échange, Y rend la halte.
	var swapped := BattleHotkeys.rebind("formation", BattleHotkeys.capture(row, _key(KEY_H)))
	check(swapped == "halt", "conflict swaps with halt (%s)" % swapped)
	check(BattleHotkeys.action_for(_key(KEY_H)) == "formation" and BattleHotkeys.action_for(_key(KEY_Y)) == "halt", "keys swapped")
	# Touches fixes : refus, combinaisons inutilisables.
	check(BattleHotkeys.rebind("halt", {"key": KEY_ESCAPE, "mods": ""}).begins_with("!") or BattleHotkeys.action_using(KEY_ESCAPE, "", "halt") == "deselect", "Escape stays reserved")
	check(BattleHotkeys.capture(row, _key(KEY_Y, true, true)).get("mods", "") == "ctrl_shift", "ctrl+shift captured")
	var alt := _key(KEY_Y)
	alt.ctrl_pressed = true
	alt.alt_pressed = true
	check(BattleHotkeys.capture(row, alt).is_empty(), "ctrl+alt refused")
	check(BattleHotkeys.capture(row, _key(KEY_SHIFT)).is_empty(), "lone modifier refused")
	# Persistance : aller-retour par le dictionnaire enregistré.
	var stored := BattleHotkeys.overrides.duplicate(true)
	BattleHotkeys.apply_saved(stored)
	check(BattleHotkeys.action_for(_key(KEY_H)) == "formation", "overrides restored from the stored dictionary")
	BattleHotkeys.apply_saved({"formation": {"key": KEY_G, "mods": ""}})  # G = garde : doublon, repli sur les défauts
	check(BattleHotkeys.action_for(_key(KEY_T)) == "formation", "duplicate stored binding falls back to defaults")
	BattleHotkeys.reset()
	check(BattleHotkeys.overrides.is_empty() and BattleHotkeys.action_for(_key(KEY_T)) == "formation", "reset restores defaults")
	# Aucun doublon dans la table par défaut.
	var seen := {}
	for r in BattleHotkeys.BINDINGS:
		if r.has("key"):
			var id := "%d/%s" % [int(r["key"]), str(r.get("mods", ""))]
			check(not seen.has(id), "default key unique: %s" % id)
			seen[id] = true
	check(BattleHotkeys.help_bbcode().contains("poursuivre"), "help still lists the pursue shortcut")


func _check_presets() -> void:
	CustomBattlePresets.path = "user://tw_misc_ui_presets_test.json"
	DirAccess.remove_absolute(CustomBattlePresets.path)
	check(CustomBattlePresets.load_all().is_empty(), "no presets at first")
	var army := {"faction": "fac_france", "budget": 6000, "technologies": ["tech_x"], "units": ["unit_a", "unit_b"]}
	check(CustomBattlePresets.save("  Ma garde  ", army), "save trims the name")
	check(not CustomBattlePresets.save("   ", army), "empty name refused")
	check(not CustomBattlePresets.save("x", {"budget": 1}), "army without faction refused")
	var all := CustomBattlePresets.load_all()
	check(all.has("Ma garde") and (all["Ma garde"]["units"] as Array).size() == 2 and int(all["Ma garde"]["budget"]) == 6000, "round trip: %s" % str(all))
	army["units"] = ["unit_a"]
	CustomBattlePresets.save("Ma garde", army)
	check((CustomBattlePresets.load_all()["Ma garde"]["units"] as Array).size() == 1, "same name replaces")
	var file := FileAccess.open(CustomBattlePresets.path, FileAccess.WRITE)
	file.store_string("pas du json")
	file.close()
	check(CustomBattlePresets.load_all().is_empty(), "corrupt file reads as empty")
	CustomBattlePresets.save("A", army)
	check(CustomBattlePresets.remove("A") and not CustomBattlePresets.remove("A"), "remove once")
	DirAccess.remove_absolute(CustomBattlePresets.path)


func _check_custom_screen() -> void:
	CustomBattlePresets.path = "user://tw_misc_ui_presets_test.json"
	DirAccess.remove_absolute(CustomBattlePresets.path)
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "custom_battle/last", "", false)
	var screen: CustomBattleScreen = CustomBattleScreen.new()
	root.add_child(screen)
	await process_frame
	var needed := screen.get_combined_minimum_size()
	check(needed.x <= 1280.0 and needed.y <= 720.0, "screen still fits 1280x720: %s" % str(needed))
	screen.config["attacker"]["budget"] = 4000
	screen.refresh()
	check(screen.buy("attacker", "unit_crossbowmen") and screen.buy("attacker", "unit_crossbowmen"), "buy two crossbowmen")
	check(screen.save_preset("attacker", "Arbalétriers"), "save_preset")
	check((screen.preset_options["attacker"] as OptionButton).item_count == 2, "preset listed in both sides' menus")
	screen.clear_army("attacker")
	screen.set_budget("attacker", 2000)
	check(screen.load_preset("defender", "Arbalétriers"), "load into the other side")
	check((screen.config["defender"]["units"] as Array).size() == 2 and int(screen.config["defender"]["budget"]) == 4000, "army and budget restored on the defender")
	check(str(screen.config["defender"]["faction"]) == "fac_france", "faction restored")
	check(not screen.load_preset("defender", "inconnue"), "unknown preset refused")
	check(screen.delete_preset("Arbalétriers") and CustomBattlePresets.names().is_empty(), "delete")
	screen.queue_free()
	await process_frame
	DirAccess.remove_absolute(CustomBattlePresets.path)


func _check_site_map() -> void:
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not sim.has_method("debug_stage_siege"):
		return
	var data_dir := CustomBattleScreen.data_dir()
	check(sim.call("new_campaign", data_dir, "fac_france", 1337), "new_campaign")
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	if armies.is_empty():
		return
	sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	var dialog: PreBattleDialog = (load("res://scenes/battle/pre_battle_dialog.tscn") as PackedScene).instantiate()
	root.add_child(dialog)
	await process_frame
	dialog.show_battle(sim, (sim.call("get_pending_battles") as Array)[0])
	await process_frame
	check(dialog.site_map != null and dialog.site_map.visible, "site map shown")
	check(dialog.site_map.size.x >= 100.0 and dialog.site_map.dot_count() > 0, "site map sized (%s) with %d army dots" % [str(dialog.site_map.size), dialog.site_map.dot_count()])
	dialog.queue_free()


func _check_menu() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings == null:
		return
	settings.call("use_test_file")
	BattleHotkeys.reset("", settings)
	var menu := SettingsMenu.new()
	root.add_child(menu)
	await process_frame
	var button := menu.find_child("Change_battle_formation", true, false) as Button
	if not check(button != null, "battle Change button exists"):
		menu.queue_free()
		return
	button.pressed.emit()
	menu.call("_input", _key(KEY_Y))
	check(BattleHotkeys.action_for(_key(KEY_Y)) == "formation", "menu capture rebinds the battle key")
	var stored: Dictionary = settings.call("get_value", BattleHotkeys.SETTING_KEY)
	check(stored.has("formation") and int(stored["formation"]["key"]) == KEY_Y, "override persisted in the settings: %s" % str(stored))
	(menu.find_child("RestoreKeys", true, false) as Button).pressed.emit()
	check(BattleHotkeys.action_for(_key(KEY_T)) == "formation" and (settings.call("get_value", BattleHotkeys.SETTING_KEY) as Dictionary).is_empty(), "restore button resets the battle keys")
	menu.queue_free()
