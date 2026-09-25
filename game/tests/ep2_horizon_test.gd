extends SceneTree

## EP2 : horizon des batailles (relief réel, raccord, choix du panorama).
## Usage : godot --headless --path game --script res://tests/ep2_horizon_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

var _failures := 0


func _init() -> void:
	await process_frame
	var field := Vector2(1200, 800)
	# Béarn : tuile chargée, Pyrénées au sud, relief réel absent au bord du champ.
	var bearn := BattleHorizon.new()
	_check(bearn.setup("prov_bearn", field, 20.0, "", "hills", "summer"), "Béarn tile loads")
	_check(bearn.panorama_id == "pyrenees", "Béarn -> pyrenees (got %s)" % bearn.panorama_id)
	_check(is_equal_approx(bearn.blend(600.0, -10.0, 7.0), 7.0), "no real relief at the field edge")
	_check(bearn.weight(600.0, -5000.0) == 1.0, "only real relief far away")
	_check(bearn.weight(600.0, -1800.0) > 0.0 and bearn.weight(600.0, -1800.0) < 1.0, "blend zone")
	var south := bearn.real_height(600.0, 400.0 + 12000.0)
	var north := bearn.real_height(600.0, 400.0 - 12000.0)
	_check(south > north + 300.0, "mountains to the south (%.0f vs %.0f)" % [south, north])
	# Hauteur de référence : le relief réel au centre est recalé sur la hauteur moyenne du champ.
	_check(absf(bearn.real_height(600.0, 400.0) - 20.0) < 60.0, "real relief pinned on the field (%.0f)" % bearn.real_height(600.0, 400.0))
	bearn.free()
	# Ponthieu côtier, côte à l'ouest : la mer réelle est tournée vers l'ouest et au niveau 0.
	var coast := BattleHorizon.new()
	_check(coast.setup("prov_ponthieu", field, 3.0, "west", "plains", "summer"), "Ponthieu tile loads")
	_check(is_equal_approx(coast.offset_y, BattleVillage.SEA_LEVEL), "coastal: real sea at the field sea level")
	var west_sea := 0
	var east_sea := 0
	for k in 20:
		var z := -4000.0 + k * 440.0
		west_sea += 1 if coast.is_sea(600.0 - 11000.0, z) else 0
		east_sea += 1 if coast.is_sea(600.0 + 11000.0, z) else 0
	_check(west_sea > east_sea, "sea on the western flank (%d vs %d)" % [west_sea, east_sea])
	coast.free()
	# Hiver en plaine : panorama d'hiver ; province inconnue : repli (inactif).
	var winter := BattleHorizon.new()
	winter.setup("prov_champagne", field, 10.0, "", "plains", "winter")
	_check(winter.panorama_id == "winter_lowlands", "winter lowlands (got %s)" % winter.panorama_id)
	winter.free()
	var none := BattleHorizon.new()
	_check(not none.setup("prov_nowhere", field, 0.0, "", "plains", "summer"), "unknown province -> fallback")
	_check(is_equal_approx(none.blend(600.0, -9000.0, 5.0), 5.0), "fallback keeps generated relief")
	none.free()
	print("ep2_horizon_test: %s" % ("OK" if _failures == 0 else "%d échec(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, label: String) -> void:
	if not condition:
		_failures += 1
		push_error("ep2_horizon_test: FAILED %s" % label)
