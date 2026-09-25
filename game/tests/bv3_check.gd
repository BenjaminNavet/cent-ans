extends SceneTree

## Lot BV3 : vérifications sans rendu (headless).
##   godot --headless --path game --script res://tests/bv3_check.gd
## Herbe couchée (empreintes, plafonds), vent et discours déterministes, choix des jeux
## d'imposteurs, données des duels et des étendards.

var _failures := 0


func _init() -> void:
	await process_frame
	_check_flatten()
	_check_wind()
	_check_speech()
	_check_impostor_sets()
	_check_finish_data()
	if _failures == 0:
		print("bv3_check OK")
	quit(0 if _failures == 0 else 1)


func _expect(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		push_error("bv3_check: " + message)


func _check_flatten() -> void:
	var flatten := BattleGrassFlatten.new()
	flatten.setup()
	var marching := {"id": 1, "present": true, "state": "marching", "x": 500.0, "z": 400.0, "facing": 0.0, "width": 20.0, "depth": 6.0}
	flatten.update([marching], 0.5)
	for i in 40:
		marching["z"] = 400.0 + float(i + 1) * 2.0
		flatten.update([marching], 0.5)
	var trodden := flatten.flatten_at(500.0, 450.0)
	_expect(trodden > 0.2 and trodden <= float(BattleGrassFlatten.MARCH_CAP) / 255.0 + 0.001, "marching troops tread the grass without mowing it (%.2f)" % trodden)
	flatten.on_corpse(Vector3(700.0, 0.0, 300.0), "infantry", 1.0)
	_expect(flatten.flatten_at(700.0, 300.0) > 0.99, "grass lies flat under a corpse")
	_expect(flatten.blood_at(700.0, 300.0) > 0.5, "blood stains the grass under a corpse")
	_expect(flatten.flatten_at(705.0, 300.0) == 0.0, "a corpse only flattens its own spot")
	var melee := {"id": 2, "present": true, "state": "melee", "x": 300.0, "z": 300.0, "facing": 0.0, "width": 20.0, "depth": 6.0}
	for i in 10:
		flatten.update([melee], 0.5)
	_expect(flatten.flatten_at(300.0, 303.0) > 0.99, "the melee front is trampled flat")
	_expect(flatten.flatten_at(-50.0, 300.0) == 0.0, "outside the map reads as untouched")


func _check_wind() -> void:
	var a := BattleStandards.wind_for("clear", 42)
	var b := BattleStandards.wind_for("clear", 42)
	_expect((a["dir"] as Vector2).is_equal_approx(b["dir"]), "wind direction is deterministic")
	_expect(is_equal_approx((a["dir"] as Vector2).length(), 1.0), "wind direction is a unit vector")
	_expect(float(BattleStandards.wind_for("fog", 42)["strength"]) < float(BattleStandards.wind_for("rain", 42)["strength"]), "fog is calmer than rain")


func _check_speech() -> void:
	var setup := {
		"attacker": {"faction": "fac_france", "general": {"name": "Philippe VI de Valois"}},
		"defender": {"faction": "fac_england"},
	}
	var strong := BattleSpeech.compose(setup, "attacker", 2.0, "hills", "rain", 7)
	var again := BattleSpeech.compose(setup, "attacker", 2.0, "hills", "rain", 7)
	_expect(not strong.is_empty(), "a speech is composed")
	_expect(strong == again, "the speech is deterministic")
	var lines: Array = strong.get("lines", [])
	_expect(lines.size() == 6, "opening, general, odds, terrain, weather, closing (%d lines)" % lines.size())
	_expect(str(lines[1]).contains("Philippe VI de Valois"), "the general names himself")
	_expect(str(strong["cry"]) == "Montjoie ! Saint-Denis !", "French war cry from the battle orders")
	var captain := BattleSpeech.compose(setup, "defender", 0.5, "plains", "clear", 7)
	_expect((captain.get("lines", []) as Array).size() == 4, "no general, no weather line: 4 lines")
	_expect(str(captain["speaker"]) != "", "someone speaks without a general")
	_expect(str(captain["cry"]) == "Saint George !", "English war cry")


func _check_impostor_sets() -> void:
	_expect(BattleImpostors.state_set("idle", false) == 0, "idle set")
	_expect(BattleImpostors.state_set("marching", false) == 1, "marching set")
	_expect(BattleImpostors.state_set("marching", true) == 2, "running set")
	_expect(BattleImpostors.state_set("melee", false) == 3, "action set")
	_expect(BattleImpostors.SETS.size() * BattleImpostors.FRAMES == 16, "16 atlas rows")


func _check_finish_data() -> void:
	var data := BattleStandards.settings()
	_expect(data.has("duels") and data.has("standards") and data.has("wind"), "battle_finish.json sections")
	var duels: Dictionary = data.get("duels", {})
	for exchange in duels.get("sequence", []):
		var clips: Dictionary = BattleSkinned.rig("infantry", 0).get("clips", {})
		_expect(clips.has(str(exchange["a"])) and clips.has(str(exchange["b"])), "duel clips %s / %s" % [exchange["a"], exchange["b"]])
	var names: Array = BattleDuels._names(duels.get("sequence", []), "b")
	_expect(str(names[-1]) == "knockdown", "the foot loser ends knocked down")
