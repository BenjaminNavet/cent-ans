extends SceneTree

## NT2 : bataille personnalisée — roster chargé, achat/retrait dans le budget, configuration
## valide, composition mémorisée, entrée du menu, lancement jusqu'au premier tick de bataille.
## Usage : godot --headless --path game --script res://tests/nt2_custom_battle_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

var _failures := 0


func _init() -> void:
	await process_frame
	if not ClassDB.class_exists("BattleSim"):
		print("nt2: extension absente, test ignoré")
		quit(0)
		return
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "custom_battle/last", "", false)
	await _check_screen()
	await _check_menu_entry()
	await _check_launch()
	if _failures == 0:
		print("nt2 custom battle OK")
	quit(1 if _failures > 0 else 0)


func _check_screen() -> void:
	var screen: CustomBattleScreen = (load("res://scenes/ui/custom_battle_screen.tscn") as PackedScene).instantiate()
	root.add_child(screen)
	await process_frame
	# L'écran tient dans la plus petite fenêtre prise en charge (1280×720, hauteur logique 800).
	var needed := screen.get_combined_minimum_size()
	print("nt2: screen minimum size %s" % str(needed))
	_check(needed.x <= 1280.0 and needed.y <= 720.0, "screen fits 1280×720: %s" % str(needed))
	_check(screen.factions.size() > 20, "playable factions listed (%d)" % screen.factions.size())
	_check(int(screen.rules.get("default_budget", 0)) == 12000, "default budget 12000")
	var french: Array = screen.roster("fac_france")
	_check(not french.is_empty(), "French roster loaded")
	_check(screen.roster_entry("fac_france", "unit_crossbowmen").get("cost", 0) == 450, "crossbowmen cost 450")
	_check(screen.roster_entry("fac_france", "unit_akinci").is_empty(), "no akinci for France")
	_check(not bool(screen.report.get("ok", true)) and screen.launch_button.disabled, "empty armies: cannot launch")
	# Achat jusqu'au budget : 2000 points (minimum des règles) = 4 arbalétriers (1800), le 5e est refusé.
	screen.config["attacker"]["budget"] = 2000
	screen.refresh()
	for i in 4:
		_check(screen.buy("attacker", "unit_crossbowmen"), "buy %d" % (i + 1))
	_check(not screen.buy("attacker", "unit_crossbowmen"), "fifth crossbowmen over budget refused")
	_check(int(screen.report["attacker"]["cost"]) == 1800, "attacker cost 1800 (%s)" % str(screen.report.get("attacker")))
	_check(not screen.buy("attacker", "unit_akinci"), "unit outside the roster refused")
	screen.remove("attacker", 0)
	_check((screen.config["attacker"]["units"] as Array).size() == 3 and int(screen.report["attacker"]["cost"]) == 1350, "remove refunds")
	_check(screen.buy("defender", "unit_crossbowmen"), "defender buys")
	_check(bool(screen.report.get("ok", false)), "valid composition: %s" % str(screen.report.get("errors")))
	_check(not screen.launch_button.disabled, "launch enabled")
	# Budget abaissé sous la dépense : le cœur refuse, le bouton se grise.
	screen.set_budget("attacker", 100)
	_check(not bool(screen.report.get("ok", true)) and screen.launch_button.disabled, "budget below spending refused")
	screen.set_budget("attacker", 6000)
	# Changer de faction retire les unités hors du nouveau roster.
	screen.set_faction("defender", "fac_ottoman")
	_check(bool(screen.report.get("ok", false)), "crossbowmen kept for the Ottomans? %s" % str(screen.report.get("errors")))
	screen.set_faction("defender", "fac_england")
	# NT11 : année (roster d'époque) et engins du siège.
	_check(int(screen.config["year"]) == 1337 and int(screen.year_spin.value) == 1337, "default year 1337")
	_check(screen.roster_entry("fac_france", "unit_francs_archers").is_empty(), "no francs-archers in 1337")
	screen.set_year(1450)
	# LR-12 : les francs-archers exigent une technologie que la France n'a pas au départ.
	_check(screen.roster_entry("fac_france", "unit_francs_archers").is_empty(), "francs-archers need their technology")
	_check(screen.grantable_technologies("attacker").any(func(t: Dictionary) -> bool: return str(t["id"]) == "tech_francs_archers"), "tech_francs_archers offered")
	screen.toggle_technology("attacker", "tech_francs_archers")
	_check(not screen.roster_entry("fac_france", "unit_francs_archers", ["tech_francs_archers"]).is_empty(), "francs-archers in 1450 with the technology")
	_check(screen.buy("attacker", "unit_francs_archers"), "buy francs-archers in 1450")
	_check(screen.tech_buttons["attacker"].text == "Technologies (1)", "technology button summary")
	_check(bool(screen.report.get("ok", false)), "1450 composition valid: %s" % str(screen.report.get("errors")))
	screen.set_year(1337)
	_check(not (screen.config["attacker"]["units"] as Array).has("unit_francs_archers"), "francs-archers dropped back in 1337")
	screen.toggle_technology("attacker", "tech_francs_archers")
	_check((screen.config["attacker"]["units"] as Array).size() == 1, "other units kept")
	_check(not bool(screen.battle_config().get("hold_opponent", true)), "hold opponent off by default")
	screen.hold_check.button_pressed = true
	_check(bool(screen.battle_config().get("hold_opponent", false)), "hold opponent option set")
	screen.hold_check.button_pressed = false
	var engines: Dictionary = screen.config["engines"]
	_check(bool(engines["ladders"]) and bool(engines["ram"]) and int(engines["towers"]) == 0, "default engines: ladders and ram")
	_check(screen.engines_button.disabled, "engines menu greyed out without siege")
	screen.set_engines(false, true, 9)
	var max_towers := int(screen.rules.get("max_siege_towers", 2))
	_check(int(screen.config["engines"]["towers"]) == max_towers, "towers capped at %d" % max_towers)
	_check(screen.engines_button.text.contains("bélier") and not screen.engines_button.text.contains("échelles"), "engines summary: %s" % screen.engines_button.text)
	screen.set_engines(true, true, 0)
	var config: Dictionary = screen.battle_config()
	_check(config["attacker"]["budget"] is int and config["fortification"] is int, "integer budgets in the config")
	var sim: Object = ClassDB.instantiate("BattleSim")
	var report: Dictionary = sim.call("validate_custom", CustomBattleScreen.data_dir(), config)
	_check(bool(report.get("ok", false)), "config valid in the core")
	# Composition mémorisée à la fermeture, relue par un nouvel écran.
	screen.close()
	await process_frame
	var saved := CustomBattleScreen.saved_config()
	_check((saved.get("attacker", {}) as Dictionary).get("units", []).size() == 1, "composition saved: %s" % str(saved))
	var again: CustomBattleScreen = (load("res://scenes/ui/custom_battle_screen.tscn") as PackedScene).instantiate()
	root.add_child(again)
	await process_frame
	_check((again.config["defender"]["units"] as Array).size() == 1 and str(again.config["defender"]["faction"]) == "fac_england", "composition restored")
	_check(bool(again.report.get("ok", false)), "restored composition valid")
	again.queue_free()
	await process_frame


func _check_menu_entry() -> void:
	var menu: Control = (load("res://scenes/start_menu.tscn") as PackedScene).instantiate()
	root.add_child(menu)
	await process_frame
	var button: Button = menu.get("custom_battle_button")
	_check(button != null and button.text == "Bataille personnalisée", "menu entry")
	menu.call("open_custom_battle")
	await process_frame
	_check(bool(menu.call("overlay_open")), "custom battle screen opened from the menu")
	menu.queue_free()
	await process_frame


func _check_launch() -> void:
	var config := {
		"attacker": {"faction": "fac_france", "budget": 6000, "units": ["unit_crossbowmen", "unit_crossbowmen"]},
		"defender": {"faction": "fac_england", "budget": 6000, "units": ["unit_crossbowmen"]},
		"terrain": "hills", "season": "autumn", "weather": "rain", "hour": "morning",
		"siege": false, "fortification": 2, "player_side": "", "seed": 7,
	}
	BattleScene.custom_config = config
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	root.add_child(scene)
	await process_frame
	await process_frame
	var battle: Object = scene.get("battle")
	_check(battle != null, "battle built from the custom config")
	if battle != null:
		var units: Array = battle.call("get_units")
		_check(units.size() == 3, "3 regiments (%d)" % units.size())
		_check(str((battle.call("get_weather") as Dictionary).get("key", "")) == "rain", "forced weather: %s" % str(battle.call("get_weather")))
		battle.call("tick", 0.1)
		_check(float(battle.call("get_elapsed")) > 0.0, "first battle tick")
	_check(BattleScene.custom_config.is_empty(), "custom config consumed")
	scene.queue_free()
	await process_frame
	# NT11 : siège avec un beffroi choisi, sans bélier.
	config["siege"] = true
	config["place"] = "castle"
	config["engines"] = {"ladders": true, "ram": false, "towers": 1}
	BattleScene.custom_config = config
	scene = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	root.add_child(scene)
	await process_frame
	await process_frame
	battle = scene.get("battle")
	_check(battle != null, "siege built from the custom config")
	if battle != null:
		var towers := (battle.call("get_units") as Array).filter(func(u: Dictionary) -> bool: return str(u.get("type", "")) == "unit_siege_tower")
		_check(towers.size() == 1, "one siege tower chosen (%d)" % towers.size())
	scene.queue_free()
	await process_frame


func _check(cond: bool, msg: String) -> bool:
	if not cond:
		_failures += 1
		push_error("nt2: " + msg)
	return cond
