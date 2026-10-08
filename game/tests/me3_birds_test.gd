extends SceneTree

## Test headless du lot ME3 (oiseaux de la carte, rendu seulement) :
##  1. fonctions pures : poids de saison, places du V, biomes, zones humides ;
##  2. espèces des données : cigognes à Strasbourg l'été (pas l'hiver), flamants en Camargue,
##     grues aux marais de l'Est, oies en automne, mouettes à la côte, rapaces en montagne,
##     étourneaux ; rien en mer pour les espèces terrestres.
## Usage : godot --headless --path game --script res://tests/me3_birds_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const SPRING := Vector4(1, 0, 0, 0)
const SUMMER := Vector4(0, 1, 0, 0)
const AUTUMN := Vector4(0, 0, 1, 0)
const WINTER := Vector4(0, 0, 0, 1)

var _failures := 0


func _init() -> void:
	await process_frame
	_run()
	print("me3_birds_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("me3_birds_test: " + message)
	return condition


func _species(id: String, species: Array) -> Dictionary:
	for spec: Dictionary in species:
		if spec["id"] == id:
			return spec
	return {}


func _run() -> void:
	# 1. Fonctions pures.
	var spec := {"seasons": {"spring": 0.5, "summer": 0.0, "autumn": 1.0, "winter": 0.2}}
	_check(is_equal_approx(MapBirdFlocks.season_weight(spec, AUTUMN), 1.0), "autumn weight")
	_check(is_equal_approx(MapBirdFlocks.season_weight(spec, SUMMER), 0.0), "summer weight")
	_check(is_equal_approx(MapBirdFlocks.season_weight(spec, Vector4(0.5, 0, 0.5, 0)), 0.75), "blended weight")
	var slots := MapBirdFlocks.v_slots(7, 2.0, 30.0)
	_check(slots.size() == 7 and slots[0] == Vector2.ZERO, "V leader at the origin")
	_check(slots[1].x < 0.0 and slots[2].x < 0.0 and is_equal_approx(slots[1].y, -slots[2].y), "V wings symmetric and behind: %s" % slots)
	_check(slots[5].x < slots[1].x, "V rows go further back")
	_check(MapBirdFlocks.biome_name(6) == "mountain" and MapBirdFlocks.biome_name(99) == "sea", "biome names")
	var sites := {"a": {"px": [100, 100], "radius_px": 10}, "b": {"px": [500, 500], "radius_px": 10}}
	_check(MapBirdFlocks.sites_near(sites, ["a", "b", "zz"], Vector2(120, 100), 15).size() == 1, "sites near")

	# 2. Données réelles.
	var data_dir := MAP_PATHS.default_data_dir()
	var map_dir := data_dir.path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not _check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var settlements := SettlementData.load_from(data_dir, map_dir)
	var world := Node3D.new()
	root.add_child(world)
	var birds := MapBirdFlocks.new()
	world.add_child(birds)
	birds.setup(map_data, null, settlements)
	_check(int(birds.stats.get("total", 0)) > 60, "birds instanced: %s" % birds.stats)
	_check(birds.has_biomes(), "biomes.png loaded")
	for id in ["crow", "goose", "starling", "gull", "flamingo", "crane", "stork", "raptor"]:
		_check((birds.stats["species"] as Dictionary).has(id), "species in data: " + id)

	var strasbourg: Vector2 = settlements.get_settlement("set_strasbourg")["px"]
	_settle(birds, strasbourg, SUMMER)
	_check(birds.placed_count_of("stork") > 0, "storks on Strasbourg roofs in summer")
	_settle(birds, strasbourg, WINTER)
	_check(birds.placed_count_of("stork") == 0, "no storks in winter")

	var camargue := Vector2(2379, 4042)
	_settle(birds, camargue, SUMMER)
	_check(birds.placed_count_of("flamingo") > 0, "flamingos in the Camargue")
	_settle(birds, Vector2(2750, 3300), SUMMER)
	_check(birds.placed_count_of("flamingo") == 0, "no flamingos in Alsace")

	var paris: Vector2 = settlements.get_settlement("set_paris")["px"]
	_settle(birds, paris, AUTUMN)
	_check(birds.placed_count_of("goose") > 0, "geese in autumn")
	_check(birds.placed_count_of("starling") > 0, "starlings in autumn")
	_settle(birds, paris, SUMMER)
	_check(birds.placed_count_of("goose") == 0, "no geese in summer")
	_check(birds.placed_count_of("gull") == 0, "no gulls inland at Paris")

	var port := Vector2.ZERO
	for entry in settlements.settlements:
		if bool(entry.get("port", false)) and str(entry["id"]) == "set_la_rochelle":
			port = entry["px"]
	if _check(port != Vector2.ZERO, "La Rochelle port found"):
		_settle(birds, port, SUMMER)
		_check(birds.placed_count_of("gull") > 0, "gulls at the coast")

	var peak := _find_peak(map_data)
	_settle(birds, peak, SUMMER)
	_check(birds.placed_count_of("raptor") > 0, "raptors over the mountains %s" % peak)

	# Les vols hors de portée sont libérés.
	_settle(birds, Vector2(100, 100), SUMMER)
	_check(birds.placed_count_of("stork") == 0 and birds.placed_count_of("flamingo") == 0, "far flocks released")
	world.queue_free()
	await process_frame


## Plusieurs passages de placement (le semis est aléatoire) autour de `focus`.
func _settle(birds: MapBirdFlocks, focus: Vector2, season: Vector4) -> void:
	for _i in 60:
		birds.retarget(focus, season)


func _find_peak(map_data: MapData) -> Vector2:
	var best := Vector2.ZERO
	var best_height := -1.0
	for y in range(0, map_data.size.y, 16):
		for x in range(0, map_data.size.x, 16):
			var h := map_data.height_m_at(x, y)
			if h > best_height:
				best_height = h
				best = Vector2(x, y)
	return best
