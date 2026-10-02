extends SceneTree

## Test headless du lot HC1 (ADR 0161, arbres généralisés de la carte de campagne) sur les vraies
## données :
##  1. réglages : style par défaut `generalised`, hauteur monde d'un feuillu adulte, arbre nettement
##     plus petit qu'un village ;
##  2. style `generalised`, forêt d'Orléans à la distance de rig 300 : tuiles semées, instances
##     visibles, pas de forêt dense ni de cartes proches 1:1, pas de buisson de haie ; aucune
##     instance dans une emprise de lieu (`SettlementLayer.vegetation_exclusions()`), dans un
##     fleuve, un lac ou la mer ; encore visibles à 700, éteints au-delà de la portée ;
##  3. style `real` : portée de 30, pas du semis et échelle 1:1 inchangés, forêt dense présente.
## Usage : godot --headless --path game --script res://tests/hc_forest_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const ORLEANS_FOREST := Vector2(2180.0, 3333.0)
const STRIDE := VegetationTileJob.FLOATS_PER_INSTANCE

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	MapPropScale.set_tree_style("")
	TownMaquetteData.clear_cache()
	ModelLibrary.clear_cache()
	print("hc_forest_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("hc_forest_test: " + message)
	return condition


func _run() -> void:
	# 1. Réglages.
	var props := MapPropScale.shared()
	_check(MapPropScale.tree_style() == MapPropScale.TREE_STYLE_GENERALISED, "default tree style should be generalised, got %s" % MapPropScale.tree_style())
	_check(is_equal_approx(props.generalised_scale() * props.generalised_reference_height, props.generalised_tree_height), "world height of an adult broadleaf")
	_check(props.generalised_tree_height >= 0.5 and props.generalised_tree_height <= 1.0, "tree height %.2f units" % props.generalised_tree_height)
	_check(props.generalised_tree_height < 0.5 * TownMaquetteData.width("village"), "a tree stays well below a village (%.2f vs %.2f)" % [props.generalised_tree_height, TownMaquetteData.width("village")])
	_check(props.generalised_max_distance >= 700.0 and props.generalised_max_distance <= 900.0, "range %.0f" % props.generalised_max_distance)
	_check(is_equal_approx(props.map_tree_scale(), props.generalised_scale()), "map tree scale follows the style")
	_check(is_equal_approx(props.generalised_density(100.0), 1.0), "no thinning at play height")

	var data_dir := MAP_PATHS.default_data_dir()
	var map_dir := data_dir.path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not _check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var settlements := SettlementData.load_from(data_dir, map_dir)
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var layer := SettlementLayer.new()
	world.add_child(layer)
	layer.setup(map_data, terrain, settlements, ZoomTiers.load_default())
	var exclusions := layer.vegetation_exclusions()
	var main_roads: Array = []
	for road: Dictionary in settlements.roads:
		if road["main"]:
			main_roads.append(road["points"])

	# 2. Style généralisé.
	var vegetation := _make_vegetation(world, terrain, map_data, exclusions, main_roads)
	_check(vegetation.generalised, "generalised style active")
	_check(vegetation.forest_detail == null, "no 1:1 dense forest layer in generalised style")
	_check(not vegetation.near_cards_active(), "no 1:1 near cards in generalised style")
	_check(vegetation.clearance != null and int(vegetation.clearance.stats["lakes"]) > 100, "lakes loaded for clearance")
	var focus := Vector3(ORLEANS_FOREST.x, map_data.surface_world_at(ORLEANS_FOREST.x, ORLEANS_FOREST.y), ORLEANS_FOREST.y)
	await _view(vegetation, focus, 300.0)
	var census := vegetation.visible_census()
	print("hc_forest_test: rig 300 %s stats %s" % [JSON.stringify(census), JSON.stringify(vegetation.stats)])
	_check(vegetation.visible and int(census["tiles"]) > 0, "trees drawn at rig 300")
	_check(int(census["instances"]) > 1000, "instances at rig 300 on the forest of Orléans (%d)" % int(census["instances"]))
	_check(int(census["shadow_parts"]) == 0, "no tree shadow at rig 300")
	_check(int(vegetation.stats.get("cleared", 0)) > 0, "some trees removed by the water / road clearance")
	var crown := props.generalised_crown_radius()
	var tiles: Dictionary = vegetation.get("_tiles")
	var near_forest := 0
	var in_place := 0
	var in_water := 0
	var hedges := 0
	var heights: Array[float] = []
	var chunk := float(vegetation.chunk_px)
	for index: int in tiles:
		var entry: Dictionary = tiles[index]
		var rect := Rect2((index % vegetation.chunks_x) * chunk, (index / vegetation.chunks_x) * chunk, chunk, chunk)
		var local := PackedVector3Array()
		for e in exclusions:
			if rect.grow(e.z).has_point(Vector2(e.x, e.y)):
				local.append(e)
		var buffers: Array = entry["buffers"]
		for slot in buffers.size():
			var buffer: PackedFloat32Array = buffers[slot]
			if slot % VegetationTileJob.KIND_COUNT == VegetationTileJob.Kind.HEDGE:
				hedges += buffer.size() / STRIDE
				continue
			var k := 0
			while k < buffer.size():
				var x := buffer[k + 3]
				var y := buffer[k + 11]
				if Vector2(x, y).distance_to(ORLEANS_FOREST) < 15.0:
					near_forest += 1
				for e in local:
					if Vector2(x - e.x, y - e.y).length() < e.z:
						in_place += 1
				if map_data.river_sd_at(x, y) < 0.0 or vegetation.clearance.coast_distance(x, y) < 0.0 or vegetation.clearance.in_lake(x, y):
					in_water += 1
				if heights.size() < 5000:
					heights.append(Vector3(buffer[k + 1], buffer[k + 5], buffer[k + 9]).length() * props.generalised_scale())
				k += STRIDE * 3
	_check(near_forest > 100, "forest of Orléans planted (%d sampled trees within 15 px)" % near_forest)
	_check(in_place == 0, "%d tree(s) inside a place footprint" % in_place)
	_check(in_water == 0, "%d tree(s) in a river, a lake or the sea" % in_water)
	_check(hedges == 0, "%d hedge bush(es) on the field grid in generalised style" % hedges)
	heights.sort()
	if _check(heights.size() > 100, "tree heights sampled"):
		var median := heights[heights.size() / 2]
		print("hc_forest_test: crown radius %.2f, world heights p10 %.2f median %.2f p90 %.2f" % [crown, heights[heights.size() / 10], median, heights[heights.size() * 9 / 10]])
		_check(median > 0.4 and median < 1.1, "median world height %.2f" % median)
	# Paris : la clairière du lieu suit l'emprise renvoyée par le calque, élargie d'un houppier.
	var paris: int = settlements.index_by_id["set_paris"]
	var paris_px: Vector2 = settlements.settlements[paris]["px"]
	var paris_rect := Rect2(paris_px - Vector2(1, 1), Vector2(2, 2))
	var found := false
	for e in vegetation.exclusions_for(paris_rect):
		if Vector2(e.x, e.y).distance_to(paris_px) < 0.01 and is_equal_approx(e.z, exclusions[paris].z + crown):
			found = true
	_check(found, "Paris clearing = layer exclusion + one crown radius")
	print("hc_forest_test: Paris exclusion radius %.1f px (layer) + crown %.2f" % [exclusions[paris].z, crown])
	await _view(vegetation, focus, 700.0)
	_check(vegetation.visible and int(vegetation.visible_census()["instances"]) > 1000, "trees still drawn at rig 700")
	print("hc_forest_test: rig 700 %s" % JSON.stringify(vegetation.visible_census()))
	await _view(vegetation, focus, props.generalised_max_distance + 50.0)
	_check(not vegetation.visible, "no tree beyond the generalised range")
	vegetation.clear()
	world.remove_child(vegetation)
	vegetation.free()

	# 3. Style réel inchangé.
	MapPropScale.set_tree_style(MapPropScale.TREE_STYLE_REAL)
	_check(is_equal_approx(props.map_tree_scale(), props.tree_ratio), "real style keeps the 1:1 scale")
	var real := _make_vegetation(world, terrain, map_data, exclusions, main_roads)
	_check(not real.generalised and real.clearance == null, "real style: no generalised path")
	_check(is_equal_approx(real.effective_max_distance(), props.tree_max_distance), "real style range %.0f" % real.effective_max_distance())
	_check(not real.has_native() or real.forest_detail != null, "real style keeps the dense forest layer")
	await _view(real, focus, 300.0)
	_check(not real.visible, "real style: no tree at rig 300")
	await _view(real, focus, 10.0)
	_check(real.visible and real.tile_count() > 0, "real style: trees at rig 10")
	var paris_real := real.exclusions_for(paris_rect)
	var same := false
	for e in paris_real:
		if Vector2(e.x, e.y).distance_to(paris_px) < 0.01 and is_equal_approx(e.z, exclusions[paris].z):
			same = true
	_check(same, "real style: exclusions unchanged")
	real.clear()
	world.queue_free()
	await process_frame


func _make_vegetation(world: Node3D, terrain: TerrainBuilder, map_data: MapData, exclusions: PackedVector3Array, roads: Array) -> Vegetation:
	var vegetation := Vegetation.new()
	vegetation.camera_rig_path = NodePath("")
	world.add_child(vegetation)
	vegetation.set_process(false)
	vegetation.quality_max_distance = -1.0  # portée des réglages, pas celle du préréglage de qualité
	vegetation.extra_exclusions = exclusions
	vegetation.clearance_roads = roads
	vegetation.bind_terrain(terrain)
	vegetation.build(map_data)
	return vegetation


## Place la vue (caméra en arrière et au-dessus du point visé) et attend la fin des semis.
func _view(vegetation: Vegetation, focus: Vector3, distance: float) -> void:
	var camera := focus + Vector3(0.0, 0.56 * distance, 0.83 * distance)
	var guard := 0
	while guard < 3000:
		guard += 1
		vegetation.update_view(camera, distance, focus)
		if vegetation.pending_jobs() == 0 and guard > 3:
			break
		await process_frame
	vegetation.update_view(camera, distance, focus)
