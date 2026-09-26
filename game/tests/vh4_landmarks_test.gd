extends SceneTree

## Test headless des lots VH0/VH4 (villes emblématiques 1:1, ADR 0078), sans pyramide :
##  1. `LandmarkV2Library` : Rouen v2 chargé, origine EPSG:3035 → unités carte proche de l'ancre
##     de la maquette L1 (même lieu), conversions [dE, dN] → repère local ;
##  2. `LandmarkPlan` : plan déterministe, milliers de maisons, parcelles hors de l'eau (Seine
##     de la carte fine) et à l'intérieur des quartiers, enceinte fermée avec portes et tours,
##     monuments à gabarit réel (cathédrale ≥ 130 m, maillage non vide), pont habité ;
##     éléments datés (beffroi de 1389 absent en 1340, présent en 1400) ;
##  3. `TownBuilder` : construction complète (nœuds de maisons par cellule d'îlots, enceinte
##     polygonale, monuments) ;
##  4. `LandmarkCityLayer` : chargement autour de Rouen, fondu de la maquette ;
##  5. plancher de caméra ZG4b levé au-dessus d'une ville v2.
## Usage : godot --headless --path game --script res://tests/vh4_landmarks_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	var city := LandmarkV2Library.for_settlement("set_rouen")
	if not _check(not city.is_empty(), "Rouen v2 loaded"):
		quit(1)
		return
	_test_library(city)
	var plan := _test_plan(city)
	await _test_builder(plan)
	await _test_layer()
	_test_camera_floor()
	_test_london()
	print("vh4_landmarks_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("vh4_landmarks_test: " + message)
	return condition


## Relief factice (m) : plateau au nord (côte), vallée de la Seine au sud.
static func _relief(x: float, y: float) -> float:
	return 8.0 + clampf(-y - 200.0, 0.0, 1200.0) * 0.05 + clampf(y - 300.0, 0.0, 800.0) * 0.001


func _heights() -> TownPlan.Heights:
	var h := TownPlan.Heights.new()
	h.func_m = _relief
	return h


func _test_library(city: Dictionary) -> void:
	var anchor := LandmarkV2Library.anchor_units(city)
	var v1 := LandmarkLibrary.for_settlement("set_rouen")
	var px: Array = v1.get("anchor", {}).get("px", [0, 0])
	var mpu := LandmarkV2Library.meters_per_unit()
	_check(absf(mpu - 718.98) < 0.1, "meters per unit %.3f" % mpu)
	_check(anchor.distance_to(Vector2(float(px[0]), float(px[1]))) * mpu < 150.0, "v2 origin near the L1 anchor (%.0f m)" % (anchor.distance_to(Vector2(float(px[0]), float(px[1]))) * mpu))
	_check(LandmarkV2Library.local([10.0, 20.0]) == Vector2(10.0, -20.0), "[dE, dN] → local (x east, y south)")
	_check(is_equal_approx(LandmarkV2Library.yaw_of(90.0), -PI * 0.5), "angle → yaw")
	_check(LandmarkV2Library.present({"from_year": 1389}, 1340) == false and LandmarkV2Library.present({"until_year": 1382}, 1340), "dated items")


func _test_plan(city: Dictionary) -> Dictionary:
	var plan := LandmarkPlan.generate(city, 1340, _heights())
	var again := LandmarkPlan.generate(city, 1340, _heights())
	var houses: Dictionary = plan["houses"]
	var n: int = (houses["x"] as PackedFloat32Array).size()
	print("vh4: plan %s" % JSON.stringify(plan["stats"]))
	_check(n > 3000, "houses %d" % n)
	_check(n == (again["houses"]["x"] as PackedFloat32Array).size() and houses["x"] == again["houses"]["x"], "deterministic plan")
	# Maisons dans un quartier, hors de l'eau (hors maisons du pont).
	var districts: LandmarkPlan.Districts = plan["districts"]
	var fixed: Dictionary = plan["fixed_bases"]
	var outside := 0
	var wet := 0
	for i in n:
		if fixed.has(i):
			continue
		var p := Vector2(houses["x"][i], houses["y"][i])
		if districts.at(p) < 0:
			outside += 1
		if LandmarkPlan._in_water(p, plan["waters"]):
			wet += 1
	_check(outside < n / 50, "houses outside districts: %d" % outside)
	_check(wet == 0, "houses in the Seine: %d" % wet)
	_check(fixed.size() >= 20, "houses on the Mathilde bridge: %d" % fixed.size())
	# Enceinte, portes, tours.
	var rings: Array = plan["wall_rings"]
	_check(rings.size() == 1 and (rings[0]["ring"] as PackedVector2Array).size() > 500, "closed wall ring")
	_check((plan["gates"] as Array).size() >= 9, "gates %d" % (plan["gates"] as Array).size())
	_check((plan["towers"] as Array).size() > 40, "towers %d" % (plan["towers"] as Array).size())
	# Monuments.
	var ids := {}
	for m in plan["v2_monuments"]:
		ids[m["id"]] = m
	_check(ids.has("cathedrale") and float(ids["cathedrale"]["length"]) >= 130.0, "cathedral at real size")
	_check(ids.has("cathedrale") and float(ids["cathedrale"]["top"]) > 80.0, "cathedral spire height %.0f" % float(ids.get("cathedrale", {"top": 0})["top"]))
	_check(not ids.has("gros_horloge") and ids.has("beffroi_communal"), "1340: communal belfry, no Gros-Horloge")
	for m in plan["v2_monuments"]:
		var verts: PackedVector3Array = (m["arrays"] as Array)[Mesh.ARRAY_VERTEX]
		_check(verts.size() >= 36 and verts.size() % 3 == 0, "monument %s mesh (%d vertices)" % [m["id"], verts.size()])
	var later := LandmarkPlan.generate(city, 1400, _heights())
	var later_ids := {}
	for m in later["v2_monuments"]:
		later_ids[m["id"]] = true
	_check(later_ids.has("gros_horloge") and not later_ids.has("beffroi_communal"), "1400: Gros-Horloge belfry")
	_check(not (plan["bridge"] as Dictionary).is_empty(), "stone bridge")
	# Pose au sol : base = point le plus bas de l'emprise.
	var k := n / 2
	var expected := TownPlan.footprint_base(_heights(), Vector2(houses["x"][k], houses["y"][k]), houses["yaw"][k], houses["front"][k], houses["depth"][k])
	_check(fixed.has(k) or absf(float(houses["base"][k]) - expected) < 0.01, "house base on the lowest corner")
	return plan


func _test_builder(plan: Dictionary) -> void:
	var root := Node3D.new()
	get_root().add_child(root)
	TownBuilder.prepare(plan)
	var b := TownBuilder.new(plan, Vector2(2096.5, 1819.9), LandmarkV2Library.meters_per_unit(), root)
	FrameBudget.unlimited = true
	var guard := 0
	while not b.step(1 << 30) and guard < 10000:
		guard += 1
	FrameBudget.unlimited = false
	_check(b.done, "builder finished")
	var names := {}
	for child in b.root.get_children():
		names[str(child.name).get_slice("_", 0)] = true
	for expected in ["Detail", "Blocks", "Ground", "Streets", "WallRing", "Monument", "Towers", "Gates", "Bridge"]:
		_check(names.has(expected), "node %s built" % expected)
	var cells := 0
	for child in b.root.get_children():
		if str(child.name).begins_with("Detail_"):
			cells += 1
	_check(cells > 20, "detail nodes by block cell: %d" % cells)
	b.free_nodes()
	root.queue_free()
	await process_frame


func _test_layer() -> void:
	var layer := LandmarkCityLayer.new()
	get_root().add_child(layer)
	layer.force_active = true
	layer.setup(null, null, ZoomTiers.load_default(), ["set_rouen", "set_paris", "set_amiens"])
	_check(layer.has_city("set_rouen") and not layer.has_city("set_amiens"), "layer knows v2 cities only")
	var z := layer.zone_of("set_rouen")
	layer.focus_override = Vector2(z.x, z.y)
	layer.update_view(1.0)
	layer.flush(Vector2(z.x, z.y))
	_check(layer.is_shown("set_rouen"), "Rouen 1:1 shown near the camera")
	layer.update_view(5.0)
	var mid := layer.fade("set_rouen")
	layer.update_view(1.0)
	var near := layer.fade("set_rouen")
	_check(near < 0.05, "maquette faded out at site tier (%.2f)" % near)
	_check(mid >= near, "fade monotonic (%.2f ≥ %.2f)" % [mid, near])
	layer.update_view(60.0)
	_check(not layer.visible and layer.fade("set_rouen") == 1.0, "strategic view: maquette only")
	layer.queue_free()
	await process_frame


func _test_camera_floor() -> void:
	var profile := CloseCameraProfile.new()
	profile.landmark_min_distance = 2.6
	var zones := PackedVector3Array([Vector3(100, 100, 6)])
	_check(profile.landmark_floor(Vector2(100, 100), zones) > 2.0, "ZG4b floor kept for v1 landmarks")
	_check(profile.landmark_floor(Vector2(100, 100), PackedVector3Array()) == 0.0, "no floor without zone (v2 city)")


## VH6 : Londres vers 1340 (mur ouvert sur la Tamise, portes datées, London Bridge habité avec
## chapelle et pont-levis, Old St Paul's et sa flèche, Tour, Westminster ; éléments datés).
func _test_london() -> void:
	var city := LandmarkV2Library.for_settlement("set_londres")
	if not _check(not city.is_empty(), "London v2 loaded"):
		return
	var v1 := LandmarkLibrary.for_settlement("set_londres")
	var px: Array = v1.get("anchor", {}).get("px", [0, 0])
	var offset := LandmarkV2Library.anchor_units(city).distance_to(Vector2(float(px[0]), float(px[1]))) * LandmarkV2Library.meters_per_unit()
	_check(offset < 300.0, "London origin near the L1 anchor (%.0f m)" % offset)
	var plan := LandmarkPlan.generate(city, 1340, _heights())
	print("vh6: plan %s" % JSON.stringify(plan["stats"]))
	var houses: Dictionary = plan["houses"]
	var n: int = (houses["x"] as PackedFloat32Array).size()
	_check(n > 6000, "London houses %d" % n)
	var fixed: Dictionary = plan["fixed_bases"]
	var wet := 0
	for i in n:
		if not fixed.has(i) and LandmarkPlan._in_water(Vector2(houses["x"][i], houses["y"][i]), plan["waters"]):
			wet += 1
	_check(wet == 0, "houses in the Thames: %d" % wet)
	_check(fixed.size() >= 60, "houses on London Bridge: %d" % fixed.size())
	var rings: Array = plan["wall_rings"]
	_check(rings.size() == 1, "one open London wall")
	var gate_names := {}
	for g in plan["gates"]:
		gate_names[str(g.get("name", ""))] = true
	for gate in ["Aldgate", "Bishopsgate", "Cripplegate", "Aldersgate", "Ludgate"]:
		_check(gate_names.has(gate), "gate %s" % gate)
	_check(not gate_names.has("Moorgate (poterne ouverte en 1415)"), "1340: no Moorgate")
	var ids := {}
	for m in plan["v2_monuments"]:
		ids[m["id"]] = m
	for id in ["old_st_pauls", "white_tower", "tower_outer", "tower_inner", "westminster_hall", "westminster_abbey_east", "guildhall", "st_mary_overie", "london_bridge_chapel", "savoy"]:
		_check(ids.has(id), "London monument %s" % id)
	if ids.has("old_st_pauls"):
		_check(float(ids["old_st_pauls"]["top"]) > 140.0, "Old St Paul's spire %.0f m" % float(ids["old_st_pauls"]["top"]))
	if ids.has("westminster_hall"):
		_check(absf(float(ids["westminster_hall"]["length"]) - 73.0) < 1.0, "Westminster Hall 73 m")
	if ids.has("london_bridge_chapel"):
		var bridge: Dictionary = plan["bridge"]
		_check(bool(ids["london_bridge_chapel"].get("fixed", false)) and float(ids["london_bridge_chapel"]["base"]) < float(bridge["deck"]), "bridge chapel on the pier")
	_check(not ids.has("jewel_tower") and not ids.has("westminster_abbey_new_nave"), "1340: no Jewel Tower, Romanesque nave")
	var later := LandmarkPlan.generate(city, 1420, _heights())
	var later_ids := {}
	for m in later["v2_monuments"]:
		later_ids[m["id"]] = true
	_check(later_ids.has("jewel_tower") and not later_ids.has("savoy") and later_ids.has("westminster_abbey_new_nave"), "1420: Jewel Tower, no Savoy, Gothic nave")
	var later_gates := {}
	for g in later["gates"]:
		later_gates[str(g.get("name", ""))] = true
	_check(later_gates.has("Moorgate (poterne ouverte en 1415)"), "1420: Moorgate open")
