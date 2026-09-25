extends SceneTree

## Test headless du lot ZG6 (villes ordinaires à l'échelle réelle, ADR 0036), sans pyramide :
##  1. `TownData` : `data/map/towns_1340.json` chargé, villes emblématiques exclues, finage ;
##  2. `TownPlan` : plan déterministe (même graine, même plan), maisons dans le noyau ou le long
##     des faubourgs, parcelles sans chevauchement, enceinte et portes des villes closes ;
##  3. pose au sol : base de chaque maison = point le plus bas de son emprise (relief factice en
##     pente avec une butte), rues drapées ;
##  4. `TownBuilder` : MultiMesh du détail et des blocs (une instance par maison et par niveau),
##     hauteur de base dans les données d'instance, boîtes englobantes à l'échelle verticale ;
##  5. `TownLayer` : chargement autour d'un point, affichage, déchargement au loin.
## Usage : godot --headless --path game --script res://tests/zg6_towns_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	var data := TownData.load_from(MAP_PATHS.default_data_dir().path_join("map"))
	_test_data(data)
	_test_plan(data)
	_test_builder(data)
	await _test_layer(data)
	print("zg6_towns_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("zg6_towns_test: " + message)
	return condition


## Relief factice (m) : pente douce vers l'est et une butte de 30 m au nord-ouest.
static func _relief(x: float, y: float) -> float:
	return 60.0 + 0.04 * x + 30.0 * exp(-((x + 300.0) * (x + 300.0) + (y + 250.0) * (y + 250.0)) / 40000.0)


func _heights() -> TownPlan.Heights:
	var h := TownPlan.Heights.new()
	h.func_m = _relief
	return h


func _test_data(data: TownData) -> void:
	_check(data.towns.size() >= 500, "towns loaded (%d)" % data.towns.size())
	for landmark in ["set_paris", "set_rouen", "set_londres", "set_bruges"]:
		_check(not data.has_town(landmark), "emblematic city %s excluded" % landmark)
	_check(data.has_town("set_amiens") and data.has_town("set_gand"), "ordinary cities present")
	var zones := data.finage_zones()
	_check(zones.size() == data.towns.size() and zones[0].z > 1.0, "finage zones (radius %.2f units)" % zones[0].z)
	_check(data.extent_units("set_gand") > data.extent_units("set_amiens"), "Ghent covers more than Amiens")


func _test_plan(data: TownData) -> void:
	for id in ["set_amiens", "set_poitiers", "set_blois", "set_gand", "set_sully_sur_loire", "set_abbaye_la_couronne"]:
		if not data.has_town(id):
			continue
		var town: Dictionary = data.towns[id]
		var a := TownPlan.generate(town, data.params, _heights())
		var b := TownPlan.generate(town, data.params, _heights())
		var houses: Dictionary = a["houses"]
		var count: int = (houses["x"] as PackedFloat32Array).size()
		print("zg6_towns_test: %s population %d households %d → %d houses, %d streets, %d monuments, %.0f ms" % [id, town["population"], town["households"], count, a["streets"].size(), a["monuments"].size(), a["stats"]["usec"] / 1000.0])
		_check(count == (b["houses"]["x"] as PackedFloat32Array).size() and houses["x"] == b["houses"]["x"] and houses["base"] == b["houses"]["base"], "%s: deterministic plan" % id)
		var wanted: int = a["house_budget"]["core"] + a["house_budget"]["out"]
		var dwellings: int = a["stats"]["dwellings"]
		var share := 0.35 if str(town["kind"]) in ["city", "town"] else 0.2
		_check(dwellings >= wanted * share or id == "set_sully_sur_loire", "%s: enough dwellings (%d for %d wanted)" % [id, dwellings, wanted])
		_check(dwellings <= wanted and count >= dwellings, "%s: not more dwellings than households" % id)
		var radii: PackedFloat32Array = a["radii"]
		var outside_core := 0
		var floating := 0
		for i in count:
			var p := Vector2(houses["x"][i], houses["y"][i])
			if int(houses["zone"][i]) == TownPlan.ZONE_INTRA and not TownPlan.inside(radii, p, 0.0):
				outside_core += 1
			var expected := TownPlan.footprint_base(_heights(), p, houses["yaw"][i], houses["front"][i], houses["depth"][i])
			if absf(float(houses["base"][i]) - expected) > 0.01 or float(houses["base"][i]) > _relief(p.x, p.y):
				floating += 1
		_check(outside_core == 0, "%s: intra-muros houses inside the enclosure (%d outside)" % [id, outside_core])
		_check(floating == 0, "%s: every house sits at the lowest point of its footprint (%d off)" % [id, floating])
		# Pas de chevauchement grossier : centres de maisons à plus de 2 m.
		var grid := {}
		var overlaps := 0
		for i in count:
			var key := Vector2i(int(floor(houses["x"][i] / 2.0)), int(floor(houses["y"][i] / 2.0)))
			if grid.has(key):
				overlaps += 1
			grid[key] = true
		_check(overlaps <= count / 100, "%s: parcels do not overlap (%d)" % [id, overlaps])
		if str(town["walls"]) != "none":
			_check((a["wall_ring"] as PackedVector2Array).size() > 20, "%s: enclosure ring" % id)
			_check(a["gates"].size() == town["gates"].size(), "%s: one gate per access road" % id)
		for s in a["streets"]:
			var pts: PackedVector2Array = s["points"]
			_check((s["bases_l"] as PackedFloat32Array).size() == pts.size(), "%s: draped street edges" % id)
			break


func _test_builder(data: TownData) -> void:
	var town: Dictionary = data.towns["set_amiens"]
	var plan := TownPlan.generate(town, data.params, _heights())
	var parent := Node3D.new()
	root.add_child(parent)
	var builder := TownBuilder.new(plan, Vector2(100, 100), data.meters_per_unit, parent)
	var steps := 0
	while not builder.step(1 << 30):
		steps += 1
	_check(builder.done, "builder completes")
	var detail := 0
	var blocks := 0
	var base_ok := true
	for child in builder.root.get_children():
		if child is MultiMeshInstance3D:
			var mm := (child as MultiMeshInstance3D).multimesh
			if child.name.begins_with("Detail_"):
				detail += mm.instance_count
			elif child.name.begins_with("Blocks_"):
				blocks += mm.instance_count
				# Serveur de rendu factice (--headless) : le tampon n'est pas relu, on vérifie la
				# présence des données d'instance seulement.
				base_ok = base_ok and mm.use_custom_data
			_check((child as GeometryInstance3D).custom_aabb.size.y > 0.0, "custom AABB set on %s" % child.name)
	var houses: int = plan["houses"]["x"].size()
	_check(detail == houses and blocks == houses, "one detail and one block instance per house (%d / %d / %d)" % [detail, blocks, houses])
	_check(base_ok, "base heights (m) carried by the instance data")
	_check(builder.root.get_node_or_null("Streets_0") != null and builder.root.get_node_or_null("Ground_0") != null and builder.root.get_node_or_null("Walls") != null, "streets and walls meshes")
	_check(is_equal_approx(builder.root.scale.x, 1.0 / data.meters_per_unit), "town root in metres")
	builder.free_nodes()
	parent.queue_free()


func _test_layer(data: TownData) -> void:
	var layer := TownLayer.new()
	root.add_child(layer)
	var ids: Array = data.towns.keys()
	ids.append("set_paris")
	layer.setup(null, null, ZoomTiers.new(), ids, data)
	layer.force_active = true
	_check(not layer.town_ids().has("set_paris"), "landmarks never streamed")
	layer.update_view(1.0)
	_check(layer.active, "active at site distance (forced without pyramid)")
	var center := data.anchor_of("set_amiens")
	layer.flush(center)
	_check(layer.is_shown("set_amiens"), "Amiens built around the camera point")
	var built := int(layer.stats.get("built", 0))
	_check(built >= 1 and built < 40, "only nearby towns built (%d)" % built)
	print("zg6_towns_test: layer stats %s" % JSON.stringify(layer.stats))
	layer.flush(center + Vector2(2000, 0))
	_check(not layer.is_shown("set_amiens"), "Amiens unloaded far away")
	layer.update_view(200.0)
	_check(not layer.active, "inactive in strategic view")
	layer.queue_free()
	await process_frame
