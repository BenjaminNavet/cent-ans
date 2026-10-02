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
const EXPECTED_STEPS := 5

var _failures := 0
var _completed := false
var _completed_steps := 0


## Simulation factice : bâtiments, fortification, chantier et population posés par le test.
class FakeSim:
	extends RefCounted

	var revision := 1
	var turn := 0
	var buildings: Dictionary = {}  # colonie → Array
	var fortification: Dictionary = {}
	var constructing: Dictionary = {}
	var resources: Dictionary = {}  # province → Array
	var population: Dictionary = {}  # province → habitants
	var devastation: Dictionary = {}
	var details := 0

	func get_state_revision() -> int:
		return revision

	func get_turn() -> int:
		return turn

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
	_check_soot(layer, data)
	await _check_worksite_sign(world, camera)
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


## 5. Suie par ville : règle (dévastation, siège, prise), puis état porté par chaque ville.
func _check_soot(layer: SettlementLayer, data: SettlementData) -> void:
	var out := layer.outbuildings
	var cfg: Dictionary = out.config["soot"]
	_check(TownSoot.from_devastation(cfg, 0.0) == 0.0 and is_equal_approx(TownSoot.from_devastation(cfg, 100.0), float(cfg["devastation_max"])), "soot follows the devastation of the province")
	var rule := TownSoot.new(cfg)
	var places: Array = [
		{"id": "a", "province": "p", "controller": "fac_france", "kind": "city"},
		{"id": "b", "province": "p", "controller": "fac_france", "kind": "town"},
		{"id": "c", "province": "q", "controller": "fac_france", "kind": "city"},
	]
	rule.update(places, {"p": {"devastation": 0.0, "besieged": true}, "q": {"devastation": 0.0, "besieged": true}}, 10)
	_check(is_equal_approx(rule.amount_of("a"), snappedf(float(cfg["siege"]), 0.02)) and rule.amount_of("b") == 0.0, "a besieged city is lightly sooted, not the other places")
	# « a » saccagée (contrôleur changé, dévastation +25), « c » prise d'assaut (sans saccage).
	places[0]["controller"] = "fac_england"
	places[2]["controller"] = "fac_england"
	var changed := rule.update(places, {"p": {"devastation": 25.0, "besieged": false}, "q": {"devastation": 0.0, "besieged": false}}, 11)
	_check(is_equal_approx(rule.amount_of("a"), snappedf(float(cfg["sack"]), 0.02)) and changed.has("a"), "a sacked town is black with soot, got %.2f" % rule.amount_of("a"))
	_check(rule.amount_of("b") < 0.1, "the town next to it, not taken, stays clean, got %.2f" % rule.amount_of("b"))
	_check(is_equal_approx(rule.amount_of("c"), snappedf(float(cfg["storm"]), 0.02)), "a stormed town is sooted, got %.2f" % rule.amount_of("c"))
	rule.update(places, {"p": {"devastation": 25.0, "besieged": false}, "q": {"devastation": 0.0, "besieged": false}}, 12)
	rule.update(places, {"p": {"devastation": 25.0, "besieged": false}, "q": {"devastation": 0.0, "besieged": false}}, 13)
	_check(rule.amount_of("a") < float(cfg["sack"]) - 0.15 and rule.amount_of("a") > 0.0, "soot fades turn after turn, got %.2f" % rule.amount_of("a"))
	# Partie rechargée (le tour saute) : des contrôleurs différents ne sont pas des prises.
	places[1]["controller"] = "fac_england"
	rule.update(places, {"p": {"devastation": 60.0, "besieged": false}, "q": {"devastation": 0.0, "besieged": false}}, 40)
	_check(rule.amount_of("b") < float(cfg["devastation_max"]) + 0.01 and rule.amount_of("c") == 0.0, "a loaded game does not read as a sack, got %.2f" % rule.amount_of("b"))
	rule.update(places, {"p": {"devastation": 0.0, "besieged": false}, "q": {"devastation": 0.0, "besieged": false}}, 41)
	_check(rule.amount_of("a") == 0.0 and rule.amount_of("b") == 0.0, "soot gone after the repairs")
	# État par ville dans le rendu : ville 1:1, maillage lointain, faubourgs.
	var shader := load("res://shaders/town_building.gdshader") as Shader
	_check(shader != null and shader.code.contains("instance uniform float town_soot"), "town_building.gdshader has a per-town soot instance parameter")
	var agen: int = data.index_by_id[TOWN]
	var agen_px: Vector2 = data.settlements[agen]["px"]
	var other := ""
	for entry in data.settlements:
		if str(entry["province"]) != PROVINCE and layer.towns.data.has_town(str(entry["id"])) and (entry["px"] as Vector2).distance_to(agen_px) < 40.0:
			other = str(entry["id"])
			break
	var sim := FakeSim.new()
	sim.resources[PROVINCE] = ["res_wheat", "res_wine"]
	sim.buildings[TOWN] = ["bld_market", "bld_windmill"]
	sim.population[PROVINCE] = out.baseline_population(PROVINCE)
	layer.refresh(sim, Callable())
	layer.update_view(10.0)
	layer.flush()
	out.flush(agen_px)
	layer.set_town_soot(TOWN, 0.8)
	_check(is_equal_approx(layer.towns.soot_of(TOWN), 0.8), "soot stored for the 1:1 town")
	_check(other != "" and layer.towns.soot_of(other) == 0.0, "the neighbouring town keeps clean roofs")
	_check(absf(layer.town_far.soot_of(TOWN) - 0.8) < 0.01 and layer.town_far.soot_of(other) == 0.0, "soot written per town in the far mesh mask, got %.2f" % layer.town_far.soot_of(TOWN))
	if _check(layer.towns.is_shown(TOWN), "the 1:1 town of %s is built near the camera" % TOWN):
		var counts := _soot_nodes(layer.towns.builder_of(TOWN), 0.8)
		_check(counts.x > 0 and counts.y == counts.x, "every node of the sooted town carries its soot (%d of %d)" % [counts.y, counts.x])
		for id: String in layer.towns.built_ids():
			if id != TOWN:
				var clean := _soot_nodes(layer.towns.builder_of(id), 0.0)
				_check(clean.x > 0 and clean.y == clean.x, "no soot on the nodes of %s (%d of %d clean)" % [id, clean.y, clean.x])
	_check(out.instances_of(TOWN, "mill").size() == 1 and is_equal_approx(out.soot_of(TOWN), 0.8) and out.soot_of(other) == 0.0, "the outbuildings of the sooted town carry its soot")
	var packed := TownBuilder.pack_instances([Transform3D(), Transform3D()], [12.0, 30.0], [0.5, 0.5])
	var buffer := OutbuildingLayer.with_soot(packed["buffer"], [0.0, 0.8])
	_check(buffer[14] == 0.0 and is_equal_approx(buffer[30], 0.8) and buffer[12] == 12.0 and buffer[28] == 30.0, "soot written per MultiMesh instance (INSTANCE_CUSTOM.b)")
	# Dévastation de la province lue dans la simulation : suie de toutes ses villes, pas des autres.
	sim.devastation[PROVINCE] = 100
	sim.revision += 1
	sim.turn += 1
	layer.refresh(sim, Callable())
	var second := ""
	for entry in data.settlements:
		if str(entry["province"]) == PROVINCE and str(entry["id"]) != TOWN:
			second = str(entry["id"])
			break
	_check(second != "" and is_equal_approx(layer.town_soot(second), float(cfg["devastation_max"])), "towns of a devastated province are sooted, got %.2f" % layer.town_soot(second))
	_check(layer.town_soot(other) == 0.0, "towns of the next province are not")
	_completed_steps += 1


## (nœuds de géométrie d'une ville, nœuds dont la suie d'instance vaut `amount`).
func _soot_nodes(builder: TownBuilder, amount: float) -> Vector2i:
	var total := 0
	var matching := 0
	if builder == null:
		return Vector2i.ZERO
	for entry in builder.geometry:
		var node: GeometryInstance3D = entry[0]
		if not is_instance_valid(node):
			continue
		total += 1
		var value: Variant = node.get_instance_shader_parameter(&"town_soot")
		if is_equal_approx(float(value) if value != null else 0.0, amount):
			matching += 1
	return Vector2i(total, matching)


## 6. Signe de chantier : la maquette (échafaudage, tas de pierres) à la place du « ⚒ ».
func _check_worksite_sign(world: Node3D, camera: Camera3D) -> void:
	var markers := ConstructionMarkers.new()
	world.add_child(markers)
	var at := camera.global_position + Vector3(0.0, -8.0, -8.0)
	markers.refresh(PackedStringArray(["prov_a", "prov_b", "prov_c"]), func(id: String) -> bool: return id != "prov_b", func(id: String) -> Vector3: return at + (Vector3(4.0, 0.0, 0.0) if id == "prov_c" else Vector3.ZERO))
	await process_frame
	_check(markers.marker_count() == 2, "one worksite per province under construction, got %d" % markers.marker_count())
	var labels := markers.find_children("*", "Label3D", true, false)
	_check(labels.is_empty(), "no hammer glyph left")
	var meshes := 0
	for child in markers.get_children():
		var marker := child as MeshInstance3D
		if marker == null or marker.is_queued_for_deletion():
			continue
		meshes += 1
		_check(marker.mesh != null and marker.mesh.get_faces().size() / 3 >= 100, "the worksite is a real model (scaffold, wall, stone heaps)")
		var distance := camera.global_position.distance_to(marker.global_position)
		_check(is_equal_approx(marker.scale.x, markers.marker_scale(distance)) and marker.scale.x > 0.0, "the sign keeps a constant size on screen")
	_check(meshes == 2, "two worksite models, got %d" % meshes)
	_check(markers.marker_scale(200.0) > markers.marker_scale(20.0) * 9.0, "the sign grows with the camera distance")
	markers.queue_free()
	_completed_steps += 1
