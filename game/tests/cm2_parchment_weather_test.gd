extends TestCase

## Test headless du lot CM2 (carte de campagne : vue parchemin et météo) :
##  1. ornements de la mer des portulans placés en mer (roses, navires, monstres) ;
##  2. poids du parchemin selon la distance caméra (fondu monotone, bornes) ;
##  3. météo du cœur exposée par le pont (`get_campaign_weather`) : toutes les provinces, genres
##     connus, déterministe, et reprise par `CampaignWeatherView` et l'ambiance sonore (AU1).
## Usage : godot --headless --path game --script res://tests/cm2_parchment_weather_test.gd

const KINDS := ["clear", "fog", "rain", "snow", "storm"]


func _init() -> void:
	await process_frame
	_run()
	finish()


func _data_dir() -> String:
	return (load("res://scripts/map/map_paths.gd") as GDScript).call("default_data_dir") as String


func _run() -> void:
	var map_dir := _data_dir().path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	_check_decor(map_data)
	_check_weight()
	_check_weather(map_data)


func _check_decor(map_data: MapData) -> void:
	var decor := ParchmentDecor.build(map_data)
	check(decor.roses.size() >= 2, "at least two compass roses (%d)" % decor.roses.size())
	check(decor.ships.size() >= 3, "ships expected (%d)" % decor.ships.size())
	check(decor.monsters.size() >= 1, "a sea monster expected (%d)" % decor.monsters.size())
	for item in decor.roses + decor.ships + decor.monsters:
		check(not map_data.is_land_px(int(item.x), int(item.y)), "ornament on land at %s" % [item])


func _check_weight() -> void:
	var view := StrategicView.new()
	var previous := -1.0
	for d in [22.0, 500.0, 1000.0, 1800.0, 2100.0, 2400.0, 2600.0]:
		var w := view.weight_at(d)
		check(w >= previous and w >= 0.0 and w <= 1.0, "parchment weight must grow with distance (%s at %s)" % [w, d])
		previous = w
	check(view.weight_at(600.0) == 0.0 and view.weight_at(2600.0) == 1.0, "3D map below the band, parchment at max zoom")
	view.free()


func _check_weather(map_data: MapData) -> void:
	if not ClassDB.class_exists("CampaignSim"):
		print("cm2_parchment_weather_test: CampaignSim missing (core/build.sh), weather skipped")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not check(bool(sim.call("new_campaign", _data_dir(), "fac_france", 7)), "new_campaign failed"):
		return
	var weather: Dictionary = sim.call("get_campaign_weather")
	check(weather.size() == map_data.province_count, "weather for every province (%d / %d)" % [weather.size(), map_data.province_count])
	var kinds := {}
	for id in weather:
		var entry: Dictionary = weather[id]
		check(KINDS.has(str(entry.get("kind", ""))), "unknown weather %s" % [entry])
		kinds[str(entry["kind"])] = true
	check(kinds.size() >= 2, "the map should not have a single sky: %s" % [kinds.keys()])
	check(weather == sim.call("get_campaign_weather"), "weather must be deterministic")
	var one: Dictionary = sim.call("get_province_weather", weather.keys()[0])
	check(one.get("kind") == weather[weather.keys()[0]]["kind"], "get_province_weather mismatch")
	# Vue météo : masque, point de carte → genre, et ambiance sonore qui lit cette source.
	var view := CampaignWeatherView.new()
	view.set("_map_data", map_data)
	view.weather = weather
	var province: Dictionary = map_data.get_province(1)
	var centroid: Vector2 = province["centroid"]
	var expected := str(weather.get(str(province["id"]), {}).get("kind", "clear"))
	check(view.weather_at(centroid) == expected or map_data.province_index_at(centroid.x, centroid.y) != 1, "weather_at should read the province under the point")
	check(view.weather_at(Vector2(-50, -50)) == "clear", "outside the map → clear")
	view.free()
