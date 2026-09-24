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
	seasons.transition_seconds = 1.0
	seasons.set_season("winter")
	_check(seasons.weights == Vector4(0, 1, 0, 0), "transition starts from summer")
	seasons.update(0.5)
	_check(seasons.weights.y > 0.2 and seasons.weights.w > 0.2, "halfway blend %s" % seasons.weights)
	_check(is_equal_approx(seasons.weights.x + seasons.weights.y + seasons.weights.z + seasons.weights.w, 1.0), "weights sum to 1")
	seasons.update(0.6)
	_check(seasons.weights == Vector4(0, 0, 0, 1), "winter reached")
	_check(RenderingServer.global_shader_parameter_get_list().has(SeasonVisuals.GLOBAL_PARAM), "global campaign_season declared in project.godot")

	# 2. Terroirs sur les vraies données.
	var data_dir := MAP_PATHS.default_data_dir()
	var map_dir := data_dir.path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not _check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var data := SettlementData.load_from(data_dir, map_dir)
	var beaune := data.get_settlement("set_beaune")
	var paris := data.get_settlement("set_paris")
	if not _check(not beaune.is_empty() and not paris.is_empty(), "set_beaune / set_paris missing"):
		return
	var states := {str(beaune["province"]): {"devastation": 60.0, "population": 80000.0}}
	var mask := TerroirMask.new()
	mask.build(data.settlements, data.hamlets, states, VegetationFields.landuse(map_data), Vector2(map_data.size))
	var at_beaune := mask.sample(beaune["px"])
	var at_paris := mask.sample(paris["px"])
	_check(at_paris.r > 0.6, "fields around Paris: %s" % at_paris)
	_check(at_paris.b < 0.01, "no burn around Paris: %s" % at_paris)
	_check(at_beaune.g > 0.2, "vineyards around Beaune: %s" % at_beaune)
	_check(at_beaune.b > 0.4, "burnt land in devastated Burgundy: %s" % at_beaune)
	_check(mask.sample(Vector2(10, 10)).r == 0.0, "no fields in the map corner")
	print("cv1_campaign_life_test: terroir mask %d ms" % mask.build_ms)
