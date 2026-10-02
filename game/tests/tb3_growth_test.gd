extends SceneTree

## Test headless du lot TB3 (colonies et bâtiments qui poussent) sur les vraies données `data/` :
##  1. correspondance `data/map/building_models.json` : niveaux d'une famille selon les bâtiments
##     construits et les ressources, plafond par colonie, échelle réelle par défaut ;
##  2. couche `OutbuildingLayer` : nombre et niveau des maquettes d'une colonie après
##     construction, maquette qui grandit sur place, hors de l'emprise bâtie, chantier visible ;
##  3. port posé sur une grève ;
##  4. croissance de la ville 1:1 : faubourgs selon la population, enceinte selon la
##     fortification ;
##  5. suie par ville ;
##  6. chantier des signes de carte : maquette à la place du « ⚒ ».
## Usage : godot --headless --path game --script res://tests/tb3_growth_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const TOWN := "set_agen"
const PROVINCE := "prov_agenais"
## Étapes qui doivent aller à leur terme (une erreur de script interrompt la fonction en cours).
const EXPECTED_STEPS := 3

var _failures := 0
var _completed := false
var _completed_steps := 0


## Simulation factice : bâtiments, fortification, chantier et population posés par le test.
class FakeSim:
	extends RefCounted

	var revision := 1
	var buildings: Dictionary = {}  # colonie → Array
	var fortification: Dictionary = {}
	var constructing: Dictionary = {}
	var resources: Dictionary = {}  # province → Array
	var population: Dictionary = {}  # province → habitants
	var devastation: Dictionary = {}
	var details := 0

	func get_state_revision() -> int:
		return revision

	func settlement_detail(id: String) -> Dictionary:
		details += 1
		var out := {"buildings": buildings.get(id, []), "fortification_level": int(fortification.get(id, 0)), "is_city": true}
		if constructing.has(id):
			out["construction"] = {"building": "bld_fair", "turns_left": 2}
		return out

	func get_province_city(province: String) -> Dictionary:
		return {"resources": resources.get(province, [])}

	func get_province_state(province: String) -> Dictionary:
		if not population.has(province):
			return {}
		return {"owner": "fac_france", "controller": "fac_france", "population_total": int(population[province]), "devastation": int(devastation.get(province, 0))}


func _init() -> void:
	await process_frame
	await _run()
	_check(_completed, "the test did not run to its end")
	ModelLibrary.clear_cache()
	print("tb3_growth_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("tb3_growth_test: " + message)
	return condition


func _levels(models: Array) -> Dictionary:
	var out := {}
	for model: Dictionary in models:
		out[str(model["family"])] = int(model["level"])
	return out


func _run() -> void:
	var data_dir := MAP_PATHS.default_data_dir()
	_check_mapping(OutbuildingLayer.load_config(data_dir))
	var map_dir := data_dir.path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not _check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var data := SettlementData.load_from(data_dir, map_dir)
	var town := data.get_settlement(TOWN)
	if not _check(not town.is_empty(), "%s missing" % TOWN):
		return
	var town_px: Vector2 = town["px"]
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var camera := Camera3D.new()
	world.add_child(camera)
	var focus := Vector3(town_px.x, map_data.surface_world_at(town_px.x, town_px.y), town_px.y)
	camera.look_at_from_position(focus + Vector3(0.0, 8.0, 8.0), focus, Vector3.UP)
	camera.current = true
	var layer := SettlementLayer.new()
	world.add_child(layer)
	layer.setup(map_data, terrain, data, ZoomTiers.load_default())
	var out := layer.outbuildings
	if not _check(out != null and out.enabled, "no outbuilding layer"):
		return
	_check(out.manifest.size() >= 25, "expected 24 models and a worksite, got %d" % out.manifest.size())
	await _check_outbuildings(layer, data, town_px)
	_check_growth(layer, data)
	_completed = _completed_steps == EXPECTED_STEPS
	world.queue_free()
	await process_frame


## 1. Correspondance bâtiment → famille → niveau.
func _check_mapping(config: Dictionary) -> void:
	if not _check(not config.is_empty(), "building_models.json not read"):
		return
	var families: Dictionary = config["families"]
	_check(families.size() == 8, "expected 8 families, got %d" % families.size())
	var wheat := ["res_wheat"]
	_check(OutbuildingLayer.family_level(families["market"], [], wheat) == 0, "no market without a building")
	_check(OutbuildingLayer.family_level(families["market"], ["bld_market"], wheat) == 1, "market level 1")
	_check(OutbuildingLayer.family_level(families["market"], ["bld_guild_hall"], wheat) == 2, "guild hall = market level 2")
	_check(OutbuildingLayer.family_level(families["market"], ["bld_fair"], wheat) == 3, "fair = market level 3")
	_check(OutbuildingLayer.family_level(families["mill"], ["bld_water_mill", "bld_forge"], []) == 3, "water mill and forge = mill level 3")
	_check(OutbuildingLayer.family_level(families["vineyard"], ["bld_vineyard_press"], wheat) == 0, "no vineyard without wine")
	_check(OutbuildingLayer.family_level(families["vineyard"], ["bld_vineyard_press"], ["res_wine"]) == 1, "vineyard level 1")
	_check(OutbuildingLayer.family_level(families["saltworks"], ["bld_market", "bld_port"], ["res_salt"]) == 2, "saltworks level 2")
	var everything := ["bld_fair", "bld_herb_garden", "bld_stables", "bld_weaving_workshop", "bld_water_mill", "bld_vineyard_press", "bld_forge", "bld_armoury", "bld_port", "bld_counting_house", "bld_abbey", "bld_scriptorium"]
	var all_resources := ["res_wheat", "res_wine", "res_iron", "res_salt"]
	var capped := OutbuildingLayer.models_for(config, everything, all_resources)
	var cap := int(config["render"]["max_per_settlement"])
	_check(capped.size() == cap, "models capped at %d per settlement, got %d" % [cap, capped.size()])
	for model: Dictionary in capped:
		_check(int(model["level"]) >= 2, "the cap keeps the highest levels (%s level %d)" % [model["family"], model["level"]])
	_check(is_equal_approx(OutbuildingLayer.exaggeration(config, 200.0), float(config["render"]["exaggeration"]["max"])), "exaggeration reaches its maximum far away")
	_check(is_equal_approx(OutbuildingLayer.exaggeration(config, 5.0), 1.0), "real scale up close")
	_completed_steps += 1


## 2 et 3. Maquettes posées autour d'une ville, puis après construction.
func _check_outbuildings(layer: SettlementLayer, data: SettlementData, town_px: Vector2) -> void:
	var out := layer.outbuildings
	var sim := FakeSim.new()
	sim.resources[PROVINCE] = ["res_wheat", "res_wine", "res_wood"]
	sim.buildings[TOWN] = ["bld_market", "bld_parish_church", "bld_windmill", "bld_vineyard_press"]
	layer.refresh(sim, Callable())
	layer.update_view(10.0)
	out.flush(town_px)
	await process_frame
	var levels := _levels(out.models_of(TOWN))
	_check(levels == {"farm": 1, "mill": 1, "vineyard": 1, "market": 1}, "level 1 models after the first buildings, got %s" % levels)
	var placed := out.instances_of(TOWN)
	_check(placed.size() == 4, "4 models placed around %s, got %d" % [TOWN, placed.size()])
	_check(out.visible and out.node_count() >= 4 and out.node_count() <= out.instance_count(), "one render node per distinct model, got %d nodes for %d instances" % [out.node_count(), out.instance_count()])
	var index: int = data.index_by_id[TOWN]
	var built := layer.built_radius(index)
	var spots := {}
	for inst: Dictionary in placed:
		var px: Vector2 = inst["px"]
		spots[str(inst["family"])] = px
		_check(px.distance_to(layer.model_px(index)) > built, "%s stands outside the built area" % inst["family"])
		_check(px.distance_to(layer.model_px(index)) < built + 3.0, "%s stays on the town's land" % inst["family"])
		_check(str(inst["model"]) == "%s_1" % inst["family"], "level 1 model for %s, got %s" % [inst["family"], inst["model"]])
	for a: String in spots:
		for b: String in spots:
			if a < b:
				_check((spots[a] as Vector2).distance_to(spots[b]) * 719.0 > 60.0, "%s and %s do not overlap" % [a, b])
	# Construction : moulin à eau, maison des métiers, haras → niveau 2, au même endroit.
	sim.buildings[TOWN] = ["bld_guild_hall", "bld_parish_church", "bld_water_mill", "bld_vineyard_press", "bld_stables"]
	sim.revision += 1
	layer.refresh(sim, Callable())
	layer.update_view(10.0)
	out.flush()
	levels = _levels(out.models_of(TOWN))
	_check(levels == {"farm": 2, "mill": 2, "vineyard": 2, "market": 2}, "level 2 models after the upgrades, got %s" % levels)
	for inst: Dictionary in out.instances_of(TOWN):
		_check(str(inst["model"]) == "%s_2" % inst["family"], "level 2 model for %s, got %s" % [inst["family"], inst["model"]])
		_check((inst["px"] as Vector2).is_equal_approx(spots[str(inst["family"])]), "%s grows in place" % inst["family"])
	# Niveau 3 et chantier en cours.
	sim.buildings[TOWN] = ["bld_fair", "bld_water_mill", "bld_vineyard_press", "bld_stables", "bld_weaving_workshop", "bld_abbey", "bld_scriptorium", "bld_counting_house"]
	sim.constructing[TOWN] = true
	sim.revision += 1
	layer.refresh(sim, Callable())
	layer.update_view(10.0)
	out.flush()
	levels = _levels(out.models_of(TOWN))
	_check(int(levels.get("farm", 0)) == 3 and int(levels.get("market", 0)) == 3 and int(levels.get("mill", 0)) == 3 and int(levels.get("vineyard", 0)) == 3 and int(levels.get("abbey", 0)) == 3, "level 3 models, got %s" % levels)
	_check(int(levels.get("worksite", 0)) == 1 and out.instances_of(TOWN, "worksite").size() == 1, "a worksite stands by the town under construction")
	var mesh_node := out.get_node_or_null("Batches/out_farm_3") as MultiMeshInstance3D
	_check(mesh_node != null and mesh_node.multimesh.instance_count == 1 and mesh_node.multimesh.mesh != null, "farm_3 drawn as one MultiMesh instance")
	_check(out.get_node_or_null("Batches/out_farm_1") == null, "level 1 farm removed once the farm reached level 3")
	# Hors de portée : rien n'est dessiné.
	layer.update_view(out.view_range() * 2.0)
	_check(not out.visible, "outbuildings hidden beyond their view range")
	layer.update_view(10.0)
	_completed_steps += 1


## 4. Croissance de la ville 1:1 : faubourgs selon la population, enceinte selon la fortification.
func _check_growth(layer: SettlementLayer, data: SettlementData) -> void:
	var out := layer.outbuildings
	var growth: Dictionary = out.config["growth"]
	_check(TownGrowth.quarter_count(growth, 1.0) == 0 and TownGrowth.quarter_count(growth, 0.7) == 0, "no suburb without growth")
	var step := float(growth["suburbs"]["population_step"])
	_check(TownGrowth.quarter_count(growth, 1.0 + step * 2.5) == 2, "one quarter per population step")
	_check(TownGrowth.quarter_count(growth, 9.0) == int(growth["suburbs"]["max_quarters"]), "quarters capped")
	_check(TownGrowth.enclosure_to_add(growth, ["bld_palisade"], [], "none") == "palisade", "a palisade built in game encloses an open town")
	_check(TownGrowth.enclosure_to_add(growth, ["bld_stone_walls"], ["bld_palisade"], "palisade") == "stone", "stone walls replace a palisade")
	_check(TownGrowth.enclosure_to_add(growth, ["bld_stone_walls"], ["bld_stone_walls"], "none") == "", "walls already there in 1337 add nothing")
	_check(TownGrowth.enclosure_to_add(growth, ["bld_castle"], [], "stone") == "", "a town walled in its 1340 plan gets no second wall")
	# Ville ouverte du plan de 1340, sans fortification de départ.
	var open_id := ""
	var towns: Dictionary = layer.towns.data.towns
	for entry in data.settlements:
		var id := str(entry["id"])
		if str(entry["kind"]) == "town" and towns.has(id) and str(towns[id]["walls"]) == "none" and (towns[id]["gates"] as Array).size() >= 2 and TownGrowth.enclosure_rank(growth, entry["initial_buildings"]) == 0:
			open_id = id
			break
	if not _check(open_id != "", "no open town in towns_1340.json"):
		return
	var entry := data.get_settlement(open_id)
	var province := str(entry["province"])
	var index: int = data.index_by_id[open_id]
	var baseline := out.baseline_population(province)
	_check(baseline > 1000.0, "baseline population of %s read from data, got %.0f" % [province, baseline])
	var sim := FakeSim.new()
	sim.population[province] = baseline
	sim.buildings[open_id] = ["bld_market"]
	layer.refresh(sim, Callable())
	layer.update_view(10.0)
	out.flush(entry["px"])
	_check(out.instances_of(open_id, "suburb").is_empty() and out.instances_of(open_id, "enclosure").is_empty(), "no growth at the 1337 population without fortification")
	# Population +20 % : deux quartiers de faubourg hors du bâti de 1340.
	sim.population[province] = baseline * (1.0 + step * 2.5)
	sim.revision += 1
	layer.refresh(sim, Callable())
	layer.update_view(10.0)
	out.flush(entry["px"])
	var houses := out.instances_of(open_id, "suburb")
	var per_quarter := int(growth["suburbs"]["houses_per_quarter"])
	_check(houses.size() == 2 * per_quarter, "two suburb quarters (%d houses) after the population grew, got %d" % [2 * per_quarter, houses.size()])
	var radii := PackedFloat32Array(towns[open_id]["radii"])
	var anchor := layer.towns.data.anchor_of(open_id)
	for inst: Dictionary in houses:
		var local := ((inst["px"] as Vector2) - anchor) * 719.0
		if not _check(not TownPlan.inside(radii, local), "suburb houses stand outside the 1340 town"):
			break
	_check(out.get_node_or_null("Batches/kit_" + str(houses[0]["model"])) != null if not houses.is_empty() else false, "suburb houses drawn with the town kit")
	# Palissade, puis murs de pierre : enceinte, tours, portes.
	sim.buildings[open_id] = ["bld_market", "bld_palisade"]
	sim.revision += 1
	layer.refresh(sim, Callable())
	layer.update_view(10.0)
	out.flush(entry["px"])
	var ring := out.instances_of(open_id, "enclosure")
	var gates := (towns[open_id]["gates"] as Array).size()
	_check(_parts(ring, "wall") == radii.size() and _parts(ring, "tower") == 0 and _parts(ring, "gate") == gates, "palisade: %d wall runs and %d gates, no tower, got %d / %d / %d" % [radii.size(), gates, _parts(ring, "wall"), _parts(ring, "gate"), _parts(ring, "tower")])
	_check(not ring.is_empty() and str(ring[0]["key"]) == "box:Planks", "palisade made of planks")
	sim.buildings[open_id] = ["bld_market", "bld_stone_walls"]
	sim.revision += 1
	layer.refresh(sim, Callable())
	layer.update_view(10.0)
	out.flush(entry["px"])
	ring = out.instances_of(open_id, "enclosure")
	_check(_parts(ring, "wall") == radii.size() and _parts(ring, "tower") >= 4 and _parts(ring, "gate") == gates, "stone enclosure: walls, towers and gates, got %d / %d / %d" % [_parts(ring, "wall"), _parts(ring, "tower"), _parts(ring, "gate")])
	for inst: Dictionary in ring:
		var local := ((inst["px"] as Vector2) - anchor) * 719.0
		if not _check(not TownPlan.inside(radii, local), "the enclosure surrounds the 1340 town"):
			break
	_check(out.get_node_or_null("Batches/tower") != null and out.get_node_or_null("Batches/box_Masonry") != null, "towers and masonry walls drawn")
	# La ville déjà murée de son plan (Agen) ne reçoit pas de seconde enceinte.
	sim.buildings[TOWN] = ["bld_stone_walls", "bld_castle"]
	sim.revision += 1
	layer.refresh(sim, Callable())
	_check(str(towns[TOWN]["walls"]) == "stone" and str(layer.growth_of(TOWN)["enclosure"]) == "", "no second wall around a town walled in 1340")
	var grown := layer.growth_of(open_id)
	_check(str(grown["enclosure"]) == "stone" and int(grown["suburb_houses"]) == 2 * per_quarter and layer.model_holder(index) == null, "growth_of sums up the growth of a town, got %s" % grown)
	_completed_steps += 1


func _parts(instances: Array, part: String) -> int:
	var count := 0
	for inst: Dictionary in instances:
		if str(inst.get("part", "")) == part:
			count += 1
	return count
