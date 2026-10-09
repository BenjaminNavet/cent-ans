extends TestCase

## Test headless du lot NT6d : réaffectation des touches (`KeyBindings`, onglet Commandes des
## réglages) : réaffecter, conflit (échange), sauvegarde / rechargement, défaut.
## Usage : godot --headless --path game --script res://tests/nt6cd_keys_test.gd

const TEST_FILE := "user://settings_nt6cd.cfg"


func _init() -> void:
	await process_frame
	var settings := root.get_node("/root/Settings")
	settings.call("use_test_file", TEST_FILE)
	DirAccess.remove_absolute(ProjectSettings.globalize_path(TEST_FILE))
	_test_rebind(settings)
	_test_conflict(settings)
	_test_persistence(settings)
	_test_menu(settings)
	_test_defaults(settings)
	DirAccess.remove_absolute(ProjectSettings.globalize_path(TEST_FILE))
	finish()


func _key(code: Key) -> InputEventKey:
	var event := InputEventKey.new()
	event.physical_keycode = code
	event.pressed = true
	return event


func _test_rebind(settings: Node) -> void:
	check(KeyBindings.all_actions().size() >= 25, "actions listed: %d" % KeyBindings.all_actions().size())
	var before: Array = KeyBindings.codes("map_toggle_court")
	check(before == KeyBindings.default_codes("map_toggle_court") and before.size() >= 1, "court on its default")
	var other := KeyBindings.rebind("map_toggle_court", 0, KEY_F15, settings)
	check(other == "", "no conflict on a free key")
	check(KeyBindings.codes("map_toggle_court")[0] == KEY_F15, "court is now F9")
	check(InputMap.action_has_event("map_toggle_court", _key(KEY_F15)), "InputMap updated")
	check(ShortcutSheet.action_keys("map_toggle_court") == "F15", "sheet follows: %s" % ShortcutSheet.action_keys("map_toggle_court"))
	check((settings.call("get_value", "input/bindings") as Dictionary).has("map_toggle_court"), "stored in settings")


func _test_conflict(settings: Node) -> void:
	# « Agents » prend la touche de « Technologies » : échange.
	var tech_before: int = KeyBindings.codes("map_toggle_tech")[0]
	var agents_before: int = KeyBindings.codes("map_toggle_agents")[0]
	var other := KeyBindings.rebind("map_toggle_agents", 0, tech_before, settings)
	check(other == "map_toggle_tech", "conflict reported: %s" % other)
	check(KeyBindings.codes("map_toggle_agents")[0] == tech_before, "agents took the key")
	check(KeyBindings.codes("map_toggle_tech")[0] == agents_before, "tech got the old key (swap)")
	check(KeyBindings.action_using(tech_before, "map_toggle_agents") == "", "no double binding left")


func _test_persistence(settings: Node) -> void:
	var expected_court: Array = KeyBindings.codes("map_toggle_court")
	var expected_agents: Array = KeyBindings.codes("map_toggle_agents")
	check(settings.call("save_settings") == OK, "saved")
	# Simule un redémarrage : InputMap d'origine, fichier relu, réglages réappliqués.
	InputMap.load_from_project_settings()
	check(KeyBindings.codes("map_toggle_court") == KeyBindings.default_codes("map_toggle_court"), "InputMap back to defaults")
	settings.call("load_settings", TEST_FILE)
	KeyBindings.apply_saved(settings.call("get_value", "input/bindings"))
	check(KeyBindings.codes("map_toggle_court") == expected_court, "court reloaded")
	check(KeyBindings.codes("map_toggle_agents") == expected_agents, "agents reloaded")


func _test_menu(settings: Node) -> void:
	var menu := SettingsMenu.new()
	root.add_child(menu)
	await process_frame
	var button := menu.find_child("Change_map_toggle_units", true, false) as Button
	if not check(button != null, "Change button exists"):
		menu.queue_free()
		return
	button.pressed.emit()
	check(button.text != "Changer", "button waits for a key")
	var event := _key(KEY_F16)
	menu.call("_input", event)
	check(KeyBindings.codes("map_toggle_units")[0] == KEY_F16, "menu capture rebinds the action")
	# Conflit par le menu : avertissement visible.
	var court_key: int = KeyBindings.codes("map_toggle_court")[0]
	(menu.find_child("Change_map_toggle_units", true, false) as Button).pressed.emit()
	menu.call("_input", _key(court_key as Key))
	var notice := menu.find_child("KeyNotice", true, false) as Label
	check(notice != null and notice.visible and notice.text.contains("échangé"), "swap warning shown")
	# Rétablir par défaut.
	(menu.find_child("RestoreKeys", true, false) as Button).pressed.emit()
	check(KeyBindings.codes("map_toggle_units") == KeyBindings.default_codes("map_toggle_units"), "restore button resets")
	menu.queue_free()


func _test_defaults(settings: Node) -> void:
	KeyBindings.reset("", settings)
	for action in KeyBindings.all_actions():
		if KeyBindings.codes(action) != KeyBindings.default_codes(action):
			check(false, "not default after reset: " + action)
	check((settings.call("get_value", "input/bindings") as Dictionary).is_empty(), "stored bindings cleared")
	# Un seul retour par défaut.
	KeyBindings.rebind("map_toggle_court", 0, KEY_F14, settings)
	KeyBindings.reset("map_toggle_court", settings)
	check(KeyBindings.codes("map_toggle_court") == KeyBindings.default_codes("map_toggle_court"), "single action reset")
