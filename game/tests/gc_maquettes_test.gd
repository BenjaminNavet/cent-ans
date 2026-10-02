extends SceneTree

## Test headless du lot GC2 (ADR 0158, maquettes stylisées des lieux) sur les vraies données :
##  1. `TownMaquetteData` : style par défaut `maquette`, familles d'architecture de quelques
##     provinces (culture > région > religion > Ouest), nom de modèle et repli sur l'Ouest ;
##  2. collisions : réduction déterministe, plancher `min_scale`, lieu emblématique jamais réduit ;
##  3. `SettlementLayer` en style `maquette` : pas de calque 1:1, une instance de MultiMesh par
##     lieu du kit + un `LandmarkModel` grossi par ville emblématique, pose au sol, bannières à
##     la couleur du contrôleur ;
##  4. emprises : rayon de clic et anneau = demi-largeur de la maquette, picking écran d'Amiens ;
##  5. style `real` : calques 1:1 présents, pas de maquette.
## Usage : godot --headless --path game --script res://tests/gc_maquettes_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	ModelLibrary.clear_cache()
	TownMaquetteData.clear_cache()
	print("gc_maquettes_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("gc_maquettes_test: " + message)
	return condition


func _run() -> void:
	# 1. Données, familles, modèles.
	var doc := TownMaquetteData.document()
	if not _check(not doc.is_empty(), "town_maquettes.json missing"):
		return
	_check(TownMaquetteData.style() == TownMaquetteData.STYLE_MAQUETTE, "default style should be maquette, got %s" % TownMaquetteData.style())
	_check(is_equal_approx(TownMaquetteData.width("city"), 11.0) and is_equal_approx(TownMaquetteData.width("village"), 3.2), "sizes from data")
	_check(is_equal_approx(TownMaquetteData.model_scale("town"), 6.5 / 2.0), "town model scale %.3f" % TownMaquetteData.model_scale("town"))
	var expected := {
		"prov_ile_de_france": "west", "prov_firenze": "med", "prov_constantinople": "byz",
		"prov_novgorod": "rus", "prov_tunis": "isl", "prov_saray": "steppe",
	}
	for province: String in expected:
		_check(FileAccess.file_exists(MAP_PATHS.default_data_dir().path_join("provinces/%s.json" % province)), "province %s missing" % province)
		_check(TownMaquetteData.family_of_province(province) == expected[province], "%s family %s, expected %s" % [province, TownMaquetteData.family_of_province(province), expected[province]])
	# Priorité : culture, puis région, puis religion, sinon Ouest.
	_check(TownMaquetteData.family_for("cul_greek", "anatolie", "rel_islam") == "byz", "culture first")
	_check(TownMaquetteData.family_for("cul_unknown", "anatolie", "rel_orthodox") == "isl", "then region")
	_check(TownMaquetteData.family_for("cul_unknown", "nowhere", "rel_orthodox") == "byz", "then religion")
	_check(TownMaquetteData.family_for("cul_unknown", "nowhere", "rel_unknown") == "west", "west by default")
	_check(TownMaquetteData.family_of_province("prov_nowhere") == "west", "unknown province is west")
	_check(TownMaquetteData.model_name_raw("city", "west", "a") == "city_a", "west model name")
	_check(TownMaquetteData.model_name_raw("city", "byz", "b") == "city_byz_b", "family model name")
	# Repli : une famille sans fichier retombe sur l'Ouest ; un fichier présent est pris.
	_check(TownMaquetteData.model_name("castle", "nofamily", "a") == "castle_a", "missing family model falls back to west")
	var byz := TownMaquetteData.model_name("city", "byz", "a")
	var byz_present := ResourceLoader.exists(TownMaquetteData.MODELS_DIR + "city_byz_a.glb")
	_check(byz == ("city_byz_a" if byz_present else "city_a"), "byz city model %s (file present: %s)" % [byz, byz_present])
	_check(TownMaquetteData.variant_of("set_amiens") == TownMaquetteData.variant_of("set_amiens") and TownMaquetteData.variant_of("set_amiens") in ["a", "b"], "deterministic variant")
	_check(TownMaquetteData.yaw_of("set_amiens") == TownMaquetteData.yaw_of("set_amiens"), "deterministic yaw")

	# 2. Collisions.
	var centers := PackedVector2Array([Vector2(0, 0), Vector2(8, 0), Vector2(0, 30), Vector2(1, 0), Vector2(100, 100), Vector2(3, 0)])
	var radii := PackedFloat32Array([5.0, 5.0, 5.0, 2.0, 2.0, 4.0])
	var fixed := PackedByteArray([0, 0, 0, 0, 0, 1])
	var scales := TownMaquetteData.solve_collisions(centers, radii, fixed, 0.6, 0.5)
	_check(is_equal_approx(scales[0], 1.0), "the most important place keeps its size")
	_check(scales[1] < 1.0 and scales[1] >= 0.6, "touching neighbour reduced (%.2f)" % scales[1])
	_check(8.0 - 5.0 - radii[1] * scales[1] >= 0.5 - 0.01 or is_equal_approx(scales[1], 0.6), "reduced neighbour clears the margin or hits the floor")
	_check(is_equal_approx(scales[2], 1.0) and is_equal_approx(scales[4], 1.0), "distant places untouched")
	_check(is_equal_approx(scales[3], 0.6), "a place inside a bigger one sits at min_scale (%.2f)" % scales[3])
	_check(is_equal_approx(scales[5], 1.0), "a fixed place is never reduced")
	_check(scales == TownMaquetteData.solve_collisions(centers, radii, fixed, 0.6, 0.5), "collisions deterministic")

	# 3. Calque en style maquette.
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
	var amiens: int = data.index_by_id["set_amiens"]
	var amiens_px: Vector2 = data.settlements[amiens]["px"]
	var camera := Camera3D.new()
	world.add_child(camera)
	var focus := Vector3(amiens_px.x, map_data.surface_world_at(amiens_px.x, amiens_px.y), amiens_px.y)
	camera.look_at_from_position(focus + Vector3(0.0, 60.0, 60.0), focus, Vector3.UP)
	camera.current = true
	var tiers := ZoomTiers.load_default()
	var layer := SettlementLayer.new()
	world.add_child(layer)
	layer.setup(map_data, terrain, data, tiers)
	var rig_distance := 85.0
	layer.update_view(rig_distance)
	layer.flush()
	await process_frame
	var maquettes := layer.maquettes
	if not _check(maquettes != null, "no TownMaquetteLayer in maquette style"):
		return
	_check(layer.towns == null and layer.landmark_cities == null and layer.town_far == null, "1:1 layers must not exist in maquette style")
	var count := data.settlements.size()
	var landmarks := int(maquettes.stats.get("landmarks", 0))
	_check(landmarks == 7, "expected the 7 landmark cities, got %d" % landmarks)
	_check(maquettes.instance_count() + landmarks == count, "instances %d + landmarks %d != places %d" % [maquettes.instance_count(), landmarks, count])
	_check(int(maquettes.stats.get("multimeshes", 0)) < maquettes.instance_count(), "places are not grouped by tile: %s" % maquettes.stats)
	print("gc_maquettes_test: %s" % JSON.stringify(maquettes.stats))
	# Taille, pose et réductions.
	var reduced := 0
	var floating := 0
	var families := {}
	for i in count:
		var kind := TownMaquetteData.kind_of(data.settlements[i])
		var base := TownMaquetteData.width(kind) * 0.5
		families[maquettes.family_of(i)] = true
		if maquettes.is_landmark(i):
			continue
		var factor := maquettes.factor_of(i)
		if factor < 0.999:
			reduced += 1
		if factor < 0.6 - 0.001 or factor > 1.001 or not is_equal_approx(maquettes.radius_of(i), base * factor):
			_check(false, "%s: factor %.3f radius %.3f (base %.3f)" % [data.settlements[i]["id"], factor, maquettes.radius_of(i), base])
			break
		var xf := maquettes.instance_transform(i)
		var px := layer.model_px(i)
		if absf(xf.basis.get_scale().x - TownMaquetteData.model_scale(kind) * factor) > 0.01:
			_check(false, "%s: instance scale %.3f" % [data.settlements[i]["id"], xf.basis.get_scale().x])
			break
		# Le sol du modèle (origine du nœud) ne dépasse pas le relief au centre.
		var ground := xf.origin
		if absf(ground.x - px.x) > 0.01 or absf(ground.z - px.y) > 0.01:
			_check(false, "%s: instance off its place (%s vs %s)" % [data.settlements[i]["id"], ground, px])
			break
		if maquettes.pose_height(i) > map_data.surface_world_at(px.x, px.y) + 0.05:
			floating += 1
	_check(floating == 0, "%d maquette(s) posed above the ground at their centre" % floating)
	_check(reduced > 0 and reduced < count, "collision reduction should touch some places only (%d)" % reduced)
	for family in ["west", "med", "byz", "rus", "isl", "steppe"]:
		_check(families.has(family), "no place resolved to family %s" % family)
	_check(maquettes.family_of(data.index_by_id["set_constantinople"]) == "byz", "Constantinople is byz")
	_check(maquettes.family_of(data.index_by_id["set_novgorod"]) == "rus", "Novgorod is rus")
	_check(maquettes.family_of(data.index_by_id["set_tunis"]) == "isl", "Tunis is isl")
	_check(maquettes.family_of(data.index_by_id["set_florence"]) == "med", "Florence is med")
	_check(maquettes.family_of(data.index_by_id["set_sarai"]) == "steppe", "Sarai is steppe")
	# Ville emblématique grossie : zone, cœur et drapé à l'échelle.
	var paris: int = data.index_by_id["set_paris"]
	var paris_model := maquettes.landmark_model(paris)
	if _check(paris_model != null, "Paris keeps its LandmarkModel"):
		var k := TownMaquetteData.landmark_scale()
		_check(is_equal_approx(paris_model.scale.x, k), "Paris model scale %.2f" % paris_model.scale.x)
		_check(is_equal_approx(paris_model.zone_radius, 6.8 * k) and is_equal_approx(layer.model_radius(paris), 6.0 * k), "Paris zone %.2f, footprint %.2f" % [paris_model.zone_radius, layer.model_radius(paris)])
		_check(layer.model_top(paris) > 0.5, "Paris top %.2f" % layer.model_top(paris))
		var material := paris_model.find_children("*", "MeshInstance3D", true, false)[0] as MeshInstance3D
		var shader_material := material.get_surface_override_material(0) as ShaderMaterial
		_check(is_equal_approx(float(shader_material.get_shader_parameter("drape_scale")), 1.0 / k), "drape scale follows the model scale")
		_check(is_equal_approx(float(shader_material.get_shader_parameter("map_extent")), 6.8 * k * 2.2), "baked heights cover the magnified zone")
		_check(layer.landmark_zones().has(Vector3(paris_model.position.x, paris_model.position.z, paris_model.zone_radius)), "camera zone follows the magnified model")
	# Bannières : couleur du contrôleur, mise à jour quand il change.
	var palette := {"fac_a": Color(0.8, 0.1, 0.1), "fac_b": Color(0.1, 0.2, 0.8)}
	var controller := str(data.settlements[amiens]["controller"])
	var color_of := func(faction: String) -> Color:
		return palette.get(faction, Color(0.3, 0.6, 0.3))
	maquettes.refresh(null, color_of)
	_check(maquettes.banner_color(amiens) == color_of.call(controller), "Amiens banner takes its controller's colour")
	data.settlements[amiens]["controller"] = "fac_b"
	maquettes.refresh(null, color_of)
	_check(maquettes.banner_color(amiens) == palette["fac_b"], "Amiens banner follows a new controller")
	data.settlements[amiens]["controller"] = controller

	# 4. Emprises : la maquette (clic, anneau, étiquette).
	_check(is_equal_approx(layer.model_radius(amiens), maquettes.radius_of(amiens)), "footprint radius = maquette half-width")
	_check(layer.model_radius(amiens) > 1.0, "Amiens footprint %.2f units" % layer.model_radius(amiens))
	_check(is_equal_approx(layer.model_top(amiens), maquettes.top_of(amiens)) and layer.model_top(amiens) > 0.5, "footprint top = maquette height (%.2f)" % layer.model_top(amiens))
	_check(layer.real_radius(amiens) > 0.0 and layer.real_radius(amiens) < layer.model_radius(amiens), "real radius kept (%.2f)" % layer.real_radius(amiens))
	var center := Vector3(amiens_px.x, terrain.surface_height_at(amiens_px.x, amiens_px.y), amiens_px.y)
	var inside := camera.unproject_position(center + camera.global_transform.basis.x * layer.model_radius(amiens) * 0.8)
	var outside := camera.unproject_position(center + camera.global_transform.basis.x * layer.model_radius(amiens) * 6.0)
	var picked := layer.pick_screen_scored(inside)
	_check(str(picked.get("id", "")) != "" and float(picked.get("score", INF)) <= 1.0, "a click inside the maquette of Amiens picks a place, got %s" % [picked])
	_check(layer._model_pick_score(amiens, camera, camera.global_transform.basis.x, camera.global_position, tiers.model_range * tiers.model_range, inside, layer._focal_px(camera)) < 1.0, "Amiens pickable at 0.8 radius")
	_check(layer._model_pick_score(amiens, camera, camera.global_transform.basis.x, camera.global_position, tiers.model_range * tiers.model_range, outside, layer._focal_px(camera)) == INF, "Amiens not pickable at 6 radii")
	layer.select("set_amiens")
	layer.update_view(rig_distance)
	var ring := layer._selection_ring
	_check(ring != null and ring.visible and is_equal_approx(ring.scale.x, layer.model_radius(amiens) * 1.1), "selection ring around the maquette")
	layer.select("")
	# Exclusions de végétation : au moins la maquette.
	var exclusions := layer.vegetation_exclusions()
	_check(exclusions[amiens].z >= layer.model_radius(amiens), "vegetation cleared under the maquette")
	# Portée par type : un village n'est plus cliquable par son emprise au-delà de sa portée.
	var village := -1
	for i in count:
		if str(data.settlements[i]["kind"]) == "village":
			village = i
			break
	_check(village >= 0 and is_equal_approx(maquettes.range_of(village), TownMaquetteData.visibility("village")), "village range from data")
	world.queue_free()
	await process_frame

	# 5. Style réel : calques 1:1, pas de maquette.
	TownMaquetteData.set_style(TownMaquetteData.STYLE_REAL)
	var world_real := Node3D.new()
	root.add_child(world_real)
	var terrain_real := TerrainBuilder.new()
	world_real.add_child(terrain_real)
	terrain_real.build(map_data)
	var layer_real := SettlementLayer.new()
	world_real.add_child(layer_real)
	layer_real.setup(map_data, terrain_real, data, tiers)
	_check(layer_real.maquettes == null and layer_real.towns != null and layer_real.landmark_cities != null and layer_real.town_far != null, "real style keeps the 1:1 layers")
	_check(layer_real.model_radius(amiens) < 2.0, "real footprint of Amiens %.2f" % layer_real.model_radius(amiens))
	world_real.queue_free()
	await process_frame
