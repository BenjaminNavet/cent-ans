extends SceneTree

## Test headless du lot CV1 (campagne vivante, rendu seulement) :
##  1. `SeasonVisuals` : saison lue dans le libellé de date, poids publiés, transition douce ;
##  2. `TerroirMask` : cultures autour des colonies, vigne en Bourgogne, brûlis si dévastation ;
##  3. `SettlementGrowth` : niveaux croissants avec la population et la fortification, Paris exclu ;
##  4. `LifeEffects` : fumées et oiseaux construits.
## Usage : godot --headless --path game --script res://tests/cv1_campaign_life_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	ModelLibrary.clear_cache()
	print("cv1_campaign_life_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("cv1_campaign_life_test: " + message)
	return condition


func _run() -> void:
	# 1. Saisons.
	var seasons := SeasonVisuals.new()
	_check(SeasonVisuals.season_from_label("Hiver 1340") == "winter", "winter label")
	seasons.set_season("summer", true)
	_check(seasons.weights == Vector4(0, 1, 0, 0), "instant summer")
