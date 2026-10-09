extends SceneTree

## Test headless du lot DN-FLEUVE : glb générés des eaux de la carte de campagne.
##  0. table livrée : chaque identifiant des règles existe dans le registre ; glb présents et
##     mesurés (saut avec message quand le paquet de modèles n'est pas installé, ADR 0212) ;
##  1. règles : flotte (culture avant bassin), lignes, fleuves, ponts (`by_id` avant structure),
##     filtrage par année ;
##  2. pose : proue sur +X, longueur du registre, pied à y = 0 ;
##  3. couche `WaterPropsLayer` sur les vraies données : moulins, ports, navires amarrés, chantiers.
## Usage : godot --headless --path game --script res://tests/dn_water_models_test.gd -- --no-tb3

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	DnWaterModels.clear_cache()
	ModelLibrary.clear_cache()
	print("dn_water_models_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("dn_water_models_test: " + message)
	return condition


func _referenced_ids() -> Array:
	var ids: Array = []
	var doc := DnWaterModels.document()
	var fleet: Dictionary = doc["fleet"]
	for list: Array in (fleet["by_basin"] as Dictionary).values() + (fleet["by_culture"] as Dictionary).values() + [fleet["default"]]:
		ids.append_array(list)
	var lanes: Dictionary = doc["sea_lanes"]
	for list: Array in (lanes["by_basin"] as Dictionary).values() + [lanes["default"]]:
		ids.append_array(list)
	for rule: Dictionary in doc["river_boats"]["rules"]:
		ids.append_array(rule["ships"])
	ids.append_array(doc["river_boats"]["default"])
	for list: Array in (doc["bridges"]["by_structure"] as Dictionary).values():
		ids.append_array(list)
	ids.append_array((doc["bridges"]["by_id"] as Dictionary).values())
	var mills: Dictionary = doc["mills"]
	for key in ["default", "tidal", "floating"]:
		ids.append_array(mills[key])
	for list: Array in (mills.get("by_family", {}) as Dictionary).values():
		ids.append_array(list)
	var ports: Dictionary = doc["ports"]
	for rule: Dictionary in (ports["by_kind"] as Dictionary).values():
		ids.append_array(rule["props"])
	for list: Array in (ports["moored_by_basin"] as Dictionary).values() + [ports["moored_default"]]:
		ids.append_array(list)
	ids.append(doc["wrecks"]["model"])
	ids.append("port_arsenal")
	ids.append("shipyard_slip")
	return ids


func _run() -> void:
	DnWaterModels.clear_cache()
	# 0. Table livrée.
	if not _check(not DnWaterModels.is_empty(), "shipped table is not empty"):
		return
	for id in _referenced_ids():
		_check(not DnWaterModels.model_entry(str(id)).is_empty(), "rule references unknown model '%s'" % id)
	var installed := not DnWaterModels.info("cog", 1).is_empty()
	if not installed:
		print("dn_water_models_test: generated models package not installed, glb checks skipped")
	else:
		for id: String in DnWaterModels.document()["models"]:
			_check(not DnWaterModels.info(id, 1).is_empty(), "glb lod1 imported for '%s'" % id)
			_check(not DnWaterModels.info(id, 2).is_empty(), "glb lod2 imported for '%s'" % id)

	# 1. Règles (table en mémoire).
	var previous := DnWaterModels.document()
	DnWaterModels.set_document({
		"defaults": {"year": 1340},
		"models": {"a": {"path": "dn/ships/x"}, "b": {"path": "dn/ships/y"}, "late": {"path": "dn/ships/z", "from_year": 1430}, "br": {"path": "dn/buildings/br"}, "av": {"path": "dn/buildings/av"}},
		"fleet": {"default": ["a"], "by_basin": {"baltic": ["b"]}, "by_culture": {"cul_x": ["late", "a"]}},
		"sea_lanes": {"default": ["a"], "by_basin": {"baltic": ["b"]}},
		"river_boats": {"default": ["a"], "rules": [{"rivers": ["Volga"], "ships": ["b"]}]},
		"bridges": {"by_structure": {"stone": ["br"]}, "by_id": {"avignon": "av"}},
	})
	_check(DnWaterModels.fleet_ids("baltic", "cul_x") == ["late", "a"], "culture before basin")
	_check(DnWaterModels.fleet_ids("baltic", "") == ["b"], "basin rule")
	_check(DnWaterModels.fleet_ids("", "") == ["a"], "fleet default")
	_check(DnWaterModels.pick(["late", "a"], 0) == "a", "ship not yet in service is skipped (1340)")
	DnWaterModels.year_override = 1450
	_check(DnWaterModels.pick(["late", "a"], 0) == "late", "ship in service in 1450")
	DnWaterModels.year_override = 0
	_check(DnWaterModels.lane_ids("baltic") == ["b"] and DnWaterModels.lane_ids("") == ["a"], "lane rules")
	_check(DnWaterModels.river_ids("Volga") == ["b"] and DnWaterModels.river_ids("Seine") == ["a"], "river rules")
	_check(DnWaterModels.bridge_id("stone", "x_1", "Pont d'Avignon", 3) == "av", "bridge by_id before structure")
	_check(DnWaterModels.bridge_id("stone", "x_1", "Pont", 3) == "br", "bridge by structure")
	_check(DnWaterModels.bridge_id("ford", "x_1", "Gué", 0) == "", "no model for a ford")
	_check(DnWaterModels.pick(null, 0) == "" and DnWaterModels.pick(["nope"], 0) == "", "unknown ids give no model")
	DnWaterModels.set_document(previous)

	if not installed:
		return

	# 2. Pose : longueur du registre sur +X, pied à y = 0.
	var info := DnWaterModels.info("cog", 1)
	var transform := DnWaterModels.pose(info, Vector3(10, 2, 20), Vector2.RIGHT)
	var box := transform * (info["mesh"] as Mesh).get_aabb()
	_check(absf(box.size.x - float(info["length"])) < 0.05 or absf(box.size.z - float(info["length"])) < 0.05, "long side fits the registry length: %s" % box.size)
	_check(absf(box.position.y - 2.0) < 0.01, "model rests on the given height: %s" % box.position.y)
	_check(box.size.x >= box.size.z - 0.01, "long axis along +X for heading +X: %s" % box.size)
	var turned := DnWaterModels.pose(info, Vector3.ZERO, Vector2.DOWN) * (info["mesh"] as Mesh).get_aabb()
	_check(turned.size.z > turned.size.x, "heading +Z turns the long axis: %s" % turned.size)
	# La longueur d'un pont (modèle sans `length`) vaut 1.
	var bridge := DnWaterModels.info("bridge_wood", 1)
	_check(absf(float(bridge["length"]) - 1.0) < 1e-4, "bridge unit length")

	# 3. Couche de décors sur les vraies données.
	var data_dir := MAP_PATHS.default_data_dir()
	var map_dir := data_dir.path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not _check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var data := SettlementData.load_from(data_dir, map_dir)
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var layer := WaterPropsLayer.new()
	world.add_child(layer)
	var started := Time.get_ticks_msec()
	layer.setup(map_data, terrain, data)
	print("dn_water_models_test: setup %d ms, %s" % [Time.get_ticks_msec() - started, layer.stats])
	_check(layer.place_count("mill") > 50, "water mills placed: %d" % layer.place_count("mill"))
	_check(layer.place_count("port") > 100, "port props placed: %d" % layer.place_count("port"))
	_check(layer.place_count("moored") > 100, "moored ships placed: %d" % layer.place_count("moored"))
	_check(layer.place_count("shipyard") > 5, "shipyards placed: %d" % layer.place_count("shipyard"))
	_check(layer.place_count("wreck") > 5, "wrecks placed: %d" % layer.place_count("wreck"))
	for place in layer.places():
		if place["kind"] == "moored":
			var px: Vector2 = place["px"]
			_check(not map_data.is_land_px(int(px.x), int(px.y)), "moored ship at sea: %s" % [px])
			break
	layer.update_view(0.0, 0.0)
	_check(not layer.visible, "props hidden at the strategic tier")
	layer.update_view(1.0, 0.0)
	_check(layer.visible, "props visible near")
	world.queue_free()
	await process_frame
