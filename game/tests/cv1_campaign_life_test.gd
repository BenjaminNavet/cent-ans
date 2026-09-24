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

	# 3. Croissance : village → bourg → ville → cité ; Paris exclu.
	var village := {"id": "set_x", "kind": "village"}
	_check(SettlementGrowth.level_of(village, {}, 100000.0) == 0, "small village stays a village")
	_check(SettlementGrowth.level_of(village, {"buildings": ["bld_market"]}, 100000.0) == 1, "village with a market becomes a bourg")
	var town := {"id": "set_y", "kind": "town"}
	_check(SettlementGrowth.level_of(town, {"fortification_level": 0, "buildings": []}, 1.0) == 1, "open town is a bourg")
	_check(SettlementGrowth.level_of(town, {"fortification_level": 0, "buildings": ["bld_stone_walls"]}, 1.0) == 2, "walled town is a ville")
	var city := {"id": "set_z", "kind": "city"}
	_check(SettlementGrowth.level_of(city, {"fortification_level": 5, "buildings": ["bld_cathedral"]}, 1.0) == 3, "cathedral city is a cité")
	_check(SettlementGrowth.level_of({"id": "set_paris", "kind": "city"}, {"buildings": ["bld_cathedral"]}, 1.0) == -1, "Paris excluded (landmark L1)")
	_check(SettlementGrowth.level_of({"id": "set_c", "kind": "castle"}, {}, 1.0) == -1, "castles keep their model")
	for level in 4:
		var model := SettlementGrowth.build_model(level, 3, level == 3)
		if _check(model != null, "model for level %d" % level):
			_check(level != 3 or model.find_child("KenneyCastle", true, false) != null, "cité with Kenney castle")
			model.free()

	# 4. Vie ambiante : bateaux en va-et-vient, navires en mer, oiseaux.
	var route := LifeAmbient._route(PackedVector2Array([Vector2(0, 0), Vector2(10, 0), Vector2(10, 10)]), 0.0, 1.0)
	_check(is_equal_approx(float(route["length"]), 20.0), "route length")
	var pose: Array = LifeAmbient.route_pose(route, 15.0)
	_check((pose[0] as Vector2).is_equal_approx(Vector2(10, 5)) and (pose[1] as Vector2).is_equal_approx(Vector2(0, 1)), "boat going down the river: %s" % [pose])
	pose = LifeAmbient.route_pose(route, 25.0)
	_check((pose[0] as Vector2).is_equal_approx(Vector2(10, 5)) and (pose[1] as Vector2).is_equal_approx(Vector2(0, -1)), "boat coming back: %s" % [pose])
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var ambient := LifeAmbient.new()
	world.add_child(ambient)
	ambient.setup(map_data, terrain, data)
	_check(int(ambient.stats["river_boats"]) > 20, "boats on the great rivers: %s" % ambient.stats)
	_check(int(ambient.stats["sea_ships"]) > 3, "ships between ports: %s" % ambient.stats)
	world.queue_free()
	await process_frame
