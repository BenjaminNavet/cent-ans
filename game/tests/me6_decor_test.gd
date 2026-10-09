extends TestCase

## Test headless des lots DN ME6/ME7/ME9 (décor ponctuel hors les villes) sur les vraies données :
##  1. données : types, règles et sites cohérents ; modèles = ids du catalogue ;
##  2. `DecorPlanner` : déterministe, tout sur la terre, sites historiques au bon endroit, règles
##     par ressource / terrain / route / pont qui produisent ;
##  3. `DecorLayer` : un MultiMesh par type au plus, période (peste après 1348), saison (foires),
##     taille tenue à l'écran, rien au-delà de la portée (vue parchemin), repli procédural.
## Usage : godot --headless --path game --script res://tests/me6_decor_test.gd

const TYPES_MAX_NODES := 40


func _init() -> void:
	await process_frame
	await _run()


func _run() -> void:
	var data_dir := OutbuildingLayer.data_dir()
	var config := DecorPlanner.load_config(data_dir)
	if not check(not config.is_empty(), "map_landmarks_extra.json not read"):
		_finish()
		return
	_check_data(config)
	var map_data := MapData.load_from_dir(data_dir.path_join("map"))
	check(map_data.load_error == "", "map load failed: %s" % map_data.load_error)
	var data := SettlementData.load_from(data_dir, data_dir.path_join("map"))
	var planner := DecorPlanner.new()
	planner.setup(config, map_data, data, DecorPlanner.load_province_info(data_dir), DecorPlanner.load_crossings(data_dir))
	var plan := planner.plan()
	_check_plan(planner, plan, map_data, data, config)
	await _check_layer(map_data, data, config)
	_finish()


func _finish() -> void:
	finish()


func _check_data(config: Dictionary) -> void:
	var types: Dictionary = config["types"]
	check(types.size() >= 40, "expected >= 40 types, got %d" % types.size())
	for rule: Dictionary in config["rules"]:
		for type: String in rule["types"]:
			check(types.has(type), "rule %s uses unknown type %s" % [rule["id"], type])
	for site: Dictionary in config["sites"]:
		check(types.has(site["type"]), "site %s uses unknown type" % site["id"])
	var catalog: Variant = JSON.parse_string(FileAccess.get_file_as_string(OutbuildingLayer.data_dir().path_join("art/dn_catalog_map_extra.json")))
	var ids := {}
	for entry: Dictionary in catalog:
		ids[entry["id"]] = true
	for type: String in types:
		check(ids.has(types[type]["model"]), "type %s: model %s not in the DN catalogue" % [type, types[type]["model"]])


func _check_plan(planner: DecorPlanner, plan: Array, map_data: MapData, data: SettlementData, config: Dictionary) -> void:
	check(plan.size() >= 3000 and plan.size() <= 20000, "plan size %d out of range" % plan.size())
	check(float(planner.stats["plan_ms"]) < 4000.0, "planning too slow: %.0f ms" % planner.stats["plan_ms"])
	var info := DecorPlanner.load_province_info(OutbuildingLayer.data_dir())
	var again := DecorPlanner.new()
	again.setup(config, map_data, data, info, DecorPlanner.load_crossings(OutbuildingLayer.data_dir()))
	var second := again.plan()
	check(second.size() == plan.size() and second[0]["px"] == plan[0]["px"] and second[second.size() - 1]["px"] == plan[plan.size() - 1]["px"], "plan is not deterministic")
	var by_rule := {}
	var wet := 0
	for instance: Dictionary in plan:
		by_rule[instance["rule"]] = int(by_rule.get(instance["rule"], 0)) + 1
		var px: Vector2 = instance["px"]
		if not map_data.is_land_px(int(px.x), int(px.y)):
			wet += 1
	check(wet == 0, "%d instances off the land" % wet)
	for rule: Dictionary in config["rules"]:
		check(int(by_rule.get(rule["id"], 0)) > 0, "rule %s produced nothing" % rule["id"])
	var carnac := _find(plan, "carnac_alignments")
	check(not carnac.is_empty() and (carnac["px"] as Vector2).distance_to(Vector2(1626.3, 3351.0)) < 10.0, "Carnac site misplaced")
	for id in ["pont_du_gard", "arenes_nimes", "montfaucon", "stonehenge", "battle_crecy", "fair_provins", "torre_hercules"]:
		check(not _find(plan, id).is_empty(), "site %s missing from the plan" % id)
	var gard := _find(plan, "pont_du_gard")
	check(not gard.is_empty() and (gard["px"] as Vector2).distance_to(Vector2(2381.5, 3980.3)) < 8.0, "Pont du Gard misplaced")
	# Mines : seulement dans une province de fer ou d'étain.
	for instance: Dictionary in plan:
		if instance["rule"] == "mines":
			var resources: Array = info[instance["province"]]["resources"]
			check(resources.has("res_iron") or resources.has("res_tin"), "mine in %s without iron or tin" % instance["province"])
	# Péages aux ponts : près d'un pont.
	var crossings := DecorPlanner.load_crossings(OutbuildingLayer.data_dir())
	var near := 0
	var tolls := 0
	for instance: Dictionary in plan:
		if instance["rule"] != "bridge_tolls":
			continue
		tolls += 1
		for crossing: Dictionary in crossings:
			if crossing["type"] == "bridge" and (instance["px"] as Vector2).distance_to(Vector2(crossing["px"][0], crossing["px"][1])) < 1.0:
				near += 1
				break
	check(tolls > 0 and near == tolls, "bridge tolls must sit at bridges (%d/%d)" % [near, tolls])
	print("me6_decor_test: %d instances, %.0f ms" % [plan.size(), planner.stats["plan_ms"]])


func _find(plan: Array, site_id: String) -> Dictionary:
	for instance: Dictionary in plan:
		if instance.get("site", "") == site_id:
			return instance
	return {}


func _check_layer(map_data: MapData, data: SettlementData, config: Dictionary) -> void:
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var camera := Camera3D.new()
	world.add_child(camera)
	camera.current = true
	var layer := SettlementLayer.new()
	world.add_child(layer)
	layer.setup(map_data, terrain, data, ZoomTiers.load_default())
	var decor := layer.decor
	if not check(decor != null and decor.enabled, "no decor layer"):
		return
	decor.force_active = true
	# 3a. Alignements de Carnac : un nœud par type, sous le plafond.
	var focus := Vector2(1626.3, 3351.0)
	camera.look_at_from_position(Vector3(focus.x, 8.0, focus.y + 8.0), Vector3(focus.x, 0.0, focus.y), Vector3.UP)
	decor.set_date(1337, "summer")
	decor.update_view(12.0)
	decor.flush(focus)
	var types_here := {}
	for instance: Dictionary in decor.shown_instances():
		types_here[instance["type"]] = true
	check(types_here.has("megalith_row"), "Carnac alignments not shown near Carnac")
	check(decor.node_count() <= TYPES_MAX_NODES and decor.node_count() == types_here.size(), "one node per type (%d nodes, %d types)" % [decor.node_count(), types_here.size()])
	check(decor.instance_count() >= 2, "too few instances around Carnac: %d" % decor.instance_count())
	check(float(decor.stats["build_ms"]) < 250.0, "rebuild too slow: %.1f ms" % decor.stats["build_ms"])
	print("me6_decor_test: Carnac view, %d instances, %d nodes, %.1f ms" % [decor.instance_count(), decor.node_count(), decor.stats["build_ms"]])
	# Voisinage large (zoom moyen) : de l'ordre de dizaines d'instances, en peu de nœuds.
	decor.update_view(45.0)
	decor.flush(focus)
	check(decor.instance_count() > 15 and decor.node_count() <= TYPES_MAX_NODES, "medium zoom: %d instances, %d nodes" % [decor.instance_count(), decor.node_count()])
	print("me6_decor_test: medium view, %d instances, %d nodes, %.1f ms" % [decor.instance_count(), decor.node_count(), decor.stats["build_ms"]])
	# Pire cas de charge : zoom large au-dessus de l'île de France (routes, villes, foires denses).
	var paris := Vector2(2096.5, 3100.0)
	decor.update_view(80.0)
	decor.flush(paris)
	check(decor.node_count() <= TYPES_MAX_NODES and decor.instance_count() <= int(config["render"]["max_instances"]), "wide zoom over Paris: %d instances, %d nodes" % [decor.instance_count(), decor.node_count()])
	print("me6_decor_test: wide view (rig 80), %d instances, %d nodes, %.1f ms" % [decor.instance_count(), decor.node_count(), decor.stats["build_ms"]])
	decor.update_view(12.0)
	decor.flush(focus)
	# Hauteurs : posées sur la surface affichée.
	var batch := decor.get_node("Batches/megalith_row") as MultiMeshInstance3D
	var placed := batch.multimesh.get_instance_transform(0)
	var ground := terrain.surface_height_at(placed.origin.x, placed.origin.z)
	check(absf(placed.origin.y - ground) < 0.01, "instance not on the ground")
	# 3b. Peste : absente en 1337, présente après 1348 (Florence).
	var florence := _site_px(decor, "plague_florence")
	decor.set_date(1337, "summer")
	decor.flush(florence)
	check(not _shown_site(decor, "plague_florence"), "plague pit shown in 1337")
	decor.set_date(1350, "summer")
	decor.flush(florence)
	check(_shown_site(decor, "plague_florence"), "plague pit missing in 1350")
	# 3c. Foires : l'été, pas l'hiver.
	var provins := _site_px(decor, "fair_provins")
	decor.set_date(1337, "summer")
	decor.flush(provins)
	check(_shown_site(decor, "fair_provins"), "fair missing in summer")
	decor.set_date(1337, "winter")
	decor.flush(provins)
	check(not _shown_site(decor, "fair_provins"), "fair shown in winter")
	# 3d. Taille tenue à l'écran et portée.
	var spec: Dictionary = config["types"]["wayside_cross"]
	check(is_equal_approx(decor.factor_of(spec, 5.0), 1.0), "real scale up close")
	check(decor.factor_of(spec, 40.0) > 20.0, "a cross is held on screen at distance 40 (factor %f)" % decor.factor_of(spec, 40.0))
	decor.force_active = false
	decor.update_view(1500.0)
	check(not decor.visible, "decor must be hidden in the parchment view")
	decor.update_view(12.0)
	check(decor.visible, "decor shown at close range")
	# 3d bis. Brancher un modèle = une entrée du manifeste DN (ici un glb existant en bouche-trou).
	decor.set_manifest({})
	check(decor.model_source("wayside_cross") == "procedural", "empty manifest: procedural fallback expected")
	decor.set_manifest({"env_cross_wayside_stone": {"files": ["folk/procession_cross.glb"]}})
	check(decor.model_source("wayside_cross") == "glb", "manifest entry must switch the type to its glb")
	check(decor.model_source("milestone") == "procedural", "other types keep their fallback")
	decor.set_manifest({})
	# 3d ter. Fumée des forges et charbonnières : un panache par instance `smoke` du voisinage.
	var forge := _first_of_type(decor, "charcoal_kiln")
	if check(not forge.is_empty(), "no charcoal kiln planned"):
		decor.flush(forge["px"])
		check(decor.smoke_count() >= 1, "charcoal kilns must smoke")
		check(not decor.smoke_sources(forge["px"], 5.0).is_empty(), "smoke sources exposed")
	decor.flush(focus)
	check(decor.smoke_count() == 0 or decor.smoke_count() < decor.instance_count(), "smoke only for smoking types")
	# 3e. Repli procédural : volumes non vides, pied à y = 0.
	for shape in ["pillar", "hut", "tower", "cone", "slab", "ring", "arch"]:
		var mesh := DecorLayer.procedural_mesh(shape, [10.0, 8.0], Color.WHITE)
		check(mesh.get_surface_count() == 1 and mesh.get_aabb().position.y >= -0.001 and mesh.get_aabb().size.y > 1.0, "procedural %s" % shape)


func _site_px(decor: DecorLayer, site_id: String) -> Vector2:
	for instance: Dictionary in decor.sites():
		if instance.get("site", "") == site_id:
			return instance["px"]
	return Vector2.ZERO


func _shown_site(decor: DecorLayer, site_id: String) -> bool:
	for instance: Dictionary in decor.shown_instances():
		if instance.get("site", "") == site_id:
			return true
	return false


func _first_of_type(decor: DecorLayer, type: String) -> Dictionary:
	for instance: Dictionary in decor.sites():
		if instance["type"] == type:
			return instance
	return {}
