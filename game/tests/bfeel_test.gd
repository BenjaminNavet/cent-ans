extends TestCase

## TW bfeel : ressenti de bataille (anneau d'ordre, barks d'ordre, pastilles munitions/ralliement,
## temps restant, ralenti du général, plan de victoire, infobulle du repère).
## Usage : godot --headless --path game --script res://tests/bfeel_test.gd


func _init() -> void:
	await process_frame
	await _check_flash()
	_check_voices()
	_check_badges()
	_check_hud_text()
	_check_victory_focus()
	await _check_integration()
	finish()


func _check_flash() -> void:
	var preview := BattlePathPreview.new()
	root.add_child(preview)
	preview.setup(null, func(_x: float, _z: float) -> float: return 0.0, Color(0.2, 0.3, 0.9))
	check(preview.flash_count() == 0, "no ring before an order")
	preview.flash_order(Vector3(10, 0, 20), "move")
	preview.flash_order(Vector3(30, 0, 40), "attack")
	check(preview.flash_count() == 2, "one ring per order")
	preview._process(0.3)
	check(preview.flash_count() == 2, "rings live for ~0.6 s")
	preview._process(0.4)
	check(preview.flash_count() == 0, "rings are gone after 0.7 s")
	check(BattlePathPreview.flash_color("attack", Color.BLUE).is_equal_approx(BattlePathPreview.RED), "attack ring is red")
	check(BattlePathPreview.flash_color("move", Color(0.2, 0.3, 0.9)).b > 0.8, "move ring takes the side colour")
	check(is_equal_approx(BattlePathPreview.flash_radius(0.3, 0.6, 1.0, 3.0), 2.0), "ring radius is linear")
	preview.queue_free()


func _check_voices() -> void:
	check(BattleVoices.situation_for_order("halt") == "hold", "halt -> hold")
	check(BattleVoices.situation_for_order("formation") == "formation", "formation bark")
	check(BattleVoices.situation_for_order("withdraw") == "retreat", "withdraw -> retreat")
	check(BattleVoices.situation_for_order("set_mode") == "", "other orders stay silent")
	for situation in ["hold", "formation", "retreat", "rally"]:
		check(not VoiceLines.situation(situation).is_empty(), "situation %s declared" % situation)
		for language in ["fr", "en", "oc", "cy"]:  # les langues régionales se replient sur fr/en
			check(not VoiceLines.bark_candidates(language, situation, "infantry").is_empty(), "%s has lines in %s (fallback)" % [situation, language])


func _check_badges() -> void:
	var base := {"state": "idle", "fatigue": 0.0}
	var low := base.duplicate()
	low["low_ammo"] = true
	check(BattleUnitMarkers.state_badges(low).has("low_ammo"), "low ammunition badge")
	var rallied := base.duplicate()
	rallied["rallied"] = true
	rallied["wavering"] = true
	rallied["under_fire"] = true
	rallied["low_ammo"] = true
	var badges := BattleUnitMarkers.state_badges(rallied)
	check(badges.size() <= BattleUnitMarkers.MAX_BADGES, "badges are capped at %d" % BattleUnitMarkers.MAX_BADGES)
	check(badges[0] == "rallied", "rallied comes first")
	var routing := {"state": "routing", "fatigue": 0.0, "low_ammo": true, "rallied": true}
	var routing_badges := BattleUnitMarkers.state_badges(routing)
	check(routing_badges.size() == 1 and routing_badges[0] == "rout", "a routing unit shows only the rout badge")


func _check_hud_text() -> void:
	check(BattleHud.time_left_text(725.0) == "Nuit dans 12:05", "time left text: %s" % BattleHud.time_left_text(725.0))
	check(BattleHud.time_left_text(-3.0) == "Nuit dans 00:00", "time left floors at zero")
	var thresholds := [300, 60]
	check(BattleHud.time_warning_text(301.0, 299.0, thresholds, false).contains("5 minute"), "warning at 300 s")
	check(BattleHud.time_warning_text(299.0, 298.0, thresholds, false) == "", "no repeat below the threshold")
	check(BattleHud.time_warning_text(61.0, 59.0, thresholds, true).contains("1 minute"), "warning at 60 s")
	check(BattleHud.time_warning_text(61.0, 59.0, thresholds, true).contains("place"), "siege wording")


func _check_victory_focus() -> void:
	var units_now := [
		{"side": "attacker", "present": true, "x": 10.0, "y": 0.0, "z": 10.0, "is_general": false, "category": "infantry"},
		{"side": "attacker", "present": true, "x": 30.0, "y": 0.0, "z": 30.0, "is_general": false, "category": "infantry"},
		{"side": "defender", "present": true, "x": 500.0, "y": 0.0, "z": 500.0, "is_general": true, "category": "infantry"},
	]
	var centre := BattleScene.victory_focus(units_now, "attacker")
	check(not centre.is_empty() and (centre["point"] as Vector3).distance_to(Vector3(20, 0, 20)) < 0.01, "no general: centre of the regiments")
	units_now[1]["is_general"] = true
	var general := BattleScene.victory_focus(units_now, "attacker")
	check((general["point"] as Vector3).distance_to(Vector3(30, 0, 30)) < 0.01, "the winning general is the focus")
	check(BattleScene.victory_focus(units_now, "nobody").is_empty(), "no unit, no shot")


func _check_integration() -> void:
	root.size = Vector2i(1600, 900)
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.autoplay = true
	root.add_child(scene)
	for _i in 6:
		await process_frame
	if not check(scene.battle != null, "battle demo failed to stage (run core/build.sh?)"):
		return
	var left := float(scene.battle.call("get_time_left_s"))
	check(left > 3000.0 and left <= 3600.0, "time_left_s from the core: %f" % left)
	check(scene.hud.time_left_label.text.begins_with("Nuit dans"), "the HUD shows the time left")
	var units: Array = scene.battle.call("get_units")
	check(units[0].has("rallied") and units[0].has("low_ammo"), "get_units exposes rallied and low_ammo")
	# Anneau d'ordre : un ordre de marche valide en pose un.
	var own: Array = units.filter(func(u: Dictionary) -> bool: return str(u["side"]) == scene.player_side)
	scene.autoplay = false
	var before: int = scene.path_preview.flash_count()
	scene.issue({"type": "move", "units": [int(own[0]["id"])], "x": float(own[0]["x"]), "z": float(own[0]["z"]) + 20.0})
	check(scene.path_preview.flash_count() == before + 1, "an accepted move order flashes a ring")
	# Ralenti de la chute du général.
	scene._general_slow_left = 0.0
	scene.autoplay = false
	scene._check_general_slowmo([{"kind": "general_down"}])
	if scene.staging != null and scene.staging.cinematic != null and scene.staging.cinematic.slowmo:
		check(scene._general_slow_factor(0.1) < 1.0, "slow motion after the general falls")
		check(scene._general_slow_factor(2.0) == 1.0, "slow motion ends")
	scene.queue_free()
