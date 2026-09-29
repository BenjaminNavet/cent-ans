extends SceneTree

## Test headless du lot C6 (rendu par paliers) sur les vraies données `data/` :
##  1. `SettlementData` : colonies (positions + types), hameaux, routes (principales comprises) ;
##  2. terrain + relief fin : au moins une tuile 8192² construite près de Paris, surface exacte
##     cohérente avec la heightmap 4096 (écart < 3 unités) ;
##  3. `SettlementLayer` : une maquette par colonie posée sous la surface affichée (rien ne
##     flotte), hameaux en `MultiMesh`, étiquettes, picking écran de Paris → `set_paris` et
##     signal `settlement_selected` ;
##  4. `RoadRenderer` : routes principales et rubans drapés construits autour de Paris ;
##  5. paliers : poids cohérents (somme 1) aux trois distances de référence.
## Usage : godot --headless --path game --script res://tests/settlements_render_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0
var _selected := ""


func _init() -> void:
	await process_frame
	await _run()
	ModelLibrary.clear_cache()
	print("settlements_render_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("settlements_render_test: " + message)
	return condition


func _run() -> void:
	var data_dir := MAP_PATHS.default_data_dir()
	var map_dir := data_dir.path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not _check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	# 1. Données.
	var data := SettlementData.load_from(data_dir, map_dir)
	_check(data.settlements.size() >= 500, "expected >= 500 settlements, got %d" % data.settlements.size())
	_check(data.hamlets.size() >= 2000, "expected >= 2000 hamlets, got %d" % data.hamlets.size())
	_check(data.roads.size() > 100, "expected roads, got %d" % data.roads.size())
	var kinds := {}
	for entry in data.settlements:
		kinds[entry["kind"]] = true
	for kind in SettlementData.KINDS:
		_check(kinds.has(kind), "no settlement of kind %s" % kind)
	_check(str(data.settlements[0]["kind"]) == "city", "settlements should be sorted by label priority (city first)")
	var paris := data.get_settlement("set_paris")
	if not _check(not paris.is_empty(), "set_paris missing"):
		return
	var paris_px: Vector2 = paris["px"]
	# Lot C7b : tracés routiers des arêtes du graphe, orientés dans les deux sens.
	_check(data.edge_paths.size() >= 400, "expected >= 400 traced graph edges, got %d" % data.edge_paths.size())
	var some_key: String = data.edge_paths.keys()[0] if not data.edge_paths.is_empty() else "|"
	var ends := some_key.split("|")
	var forward := data.edge_path(ends[0], ends[1])
	var backward := data.edge_path(ends[1], ends[0])
	_check(forward.size() >= 2 and forward.size() == backward.size() and forward[0] == backward[backward.size() - 1], "edge_path orientation")
	var from_entry := data.get_settlement(ends[0])
	_check(not from_entry.is_empty() and forward.size() >= 2 and forward[0].distance_to(from_entry["px"]) < 0.1, "edge path should start on its settlement")
	_check(data.edge_path("set_paris", "set_nowhere").is_empty(), "unknown edge should have no path")

	# 2. Terrain + relief fin.
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var camera := Camera3D.new()
	world.add_child(camera)
	var focus := Vector3(paris_px.x, map_data.surface_world_at(paris_px.x, paris_px.y), paris_px.y + 10.0)
	camera.position = focus + Vector3(0.0, 30.0, 45.0)
	camera.look_at_from_position(camera.position, focus, Vector3.UP)
	camera.current = true
	var tiers := ZoomTiers.load_default()
	for _i in 60:
		terrain.update_lod(camera.global_position, 55.0, focus, tiers.fine_terrain_distance)
		if terrain.fine_ready() and terrain.fine_chunk_count() > 0:
			break
		terrain.wait_fine_jobs()
		await process_frame
	_check(terrain.fine_chunk_count() > 0, "no fine relief chunk near Paris (tiles in %s)" % map_dir.path_join("height"))
	var worst := 0.0
	for k in 20:
		var p := paris_px + Vector2(cos(k) * 30.0, sin(k * 1.7) * 30.0)
		worst = maxf(worst, absf(terrain.surface_height_at(p.x, p.y) - map_data.surface_world_at(p.x, p.y)))
	_check(worst < 3.0, "fine surface too far from the 4096 heightmap near Paris: %.2f" % worst)

	# 3. Colonies.
	var roads := RoadRenderer.new()
	world.add_child(roads)
	roads.build(map_data, data, terrain)
	var layer := SettlementLayer.new()
	world.add_child(layer)
	layer.setup(map_data, terrain, data, tiers)
	layer.settlement_selected.connect(func(id: String) -> void: _selected = id)
	layer.update_view(55.0)
	layer.flush()
	roads.update_view(0.0, 1.0)
	roads.flush(1.0)
	await process_frame
	_check(int(layer.stats.get("models", 0)) == data.settlements.size(), "expected one model per settlement, got %s" % layer.stats)
	var floating := 0
	for i in data.settlements.size():
		var holder: Node3D = layer._models[i]
		if holder == null:
			continue
		var px: Vector2 = data.settlements[i]["px"]
		if holder.position.y > terrain.surface_height_at(px.x, px.y) + 0.01:
			floating += 1
	_check(floating == 0, "%d settlement models float above the displayed surface" % floating)
	_check(layer.hamlet_instance_count() > 0, "no hamlet instances near Paris")
	_check(roads.ribbon_count() > 0, "no road ribbon near Paris")
	_check(int(roads.stats.get("main_roads", 0)) > 0, "no main road")
	var screen := camera.unproject_position(layer.world_position_of("set_paris") + Vector3(0.0, 1.0, 0.0))
	var picked := layer.pick_screen(screen)
	_check(picked == "set_paris", "picking at Paris returned '%s'" % picked)
	layer.select(picked)
	_check(_selected == "set_paris", "settlement_selected not emitted")
	layer.declutter()
	_check(layer.visible_label_count() > 0, "no settlement label visible in county view")

	# 4 bis. Lot C7b : arbres posés sur la surface affichée (relief fin compris) et recalés quand
	# une tuile change de niveau.
	var vegetation := Vegetation.new()
	world.add_child(vegetation)
	vegetation.bind_terrain(terrain)
	vegetation.build(map_data)
	vegetation.update_view(camera.global_position, 55.0)
	for _i in 30:
		if vegetation.pending_jobs() == 0:
			break
		await process_frame
		vegetation.update_view(camera.global_position, 55.0)
	vegetation.flush_ground()
	_check(vegetation.instance_count() > 0, "no tree near Paris")
	var fine_tree_tiles := 0
	for index in vegetation._tiles:
		if terrain.chunk_level(index) == 2:
			fine_tree_tiles += 1
	_check(fine_tree_tiles > 0, "no tree tile on fine relief near Paris")
	var tree_error := vegetation.max_ground_error()
	_check(tree_error < 0.05, "trees off the displayed surface by %.2f" % tree_error)
	# Relief fin désactivé : les tuiles repassent au LOD proche, les arbres suivent.
	terrain.fine_enabled = false
	terrain.update_lod(camera.global_position, 55.0, focus, tiers.fine_terrain_distance)
	_check(vegetation.pending_regrounds() > 0, "no reground queued after a chunk level change")
	vegetation.flush_ground()
	var tree_error_near := vegetation.max_ground_error()
	_check(tree_error_near < 0.05, "trees off the near LOD surface by %.2f" % tree_error_near)
	terrain.fine_enabled = true

	# 5. Paliers (lot DV : vue normale / vue stratégique).
	_check(tiers.strategic_weight(1000.0) < 0.001 and tiers.strategic_weight(1400.0) > 0.999, "strategic fade 1100-1300")
	_check(tiers.tier_at(40.0) == ZoomTiers.Tier.NEAR and tiers.tier_at(1100.0) == ZoomTiers.Tier.NEAR and tiers.tier_at(1400.0) == ZoomTiers.Tier.STRATEGIC, "tier_at thresholds")
	print("settlements_render_test: %s" % JSON.stringify({
		"settlements": data.settlements.size(), "hamlets": data.hamlets.size(), "roads": data.roads.size(),
		"fine_chunks": terrain.fine_chunk_count(), "fine_worst_delta": snappedf(worst, 0.01),
		"hamlet_instances": layer.hamlet_instance_count(), "ribbons": roads.ribbon_count(),
		"tree_error": snappedf(tree_error, 0.001), "regrounds": vegetation.stats["regrounds"], "reground_ms_max": vegetation.stats["reground_ms_max"], "tree_instances": vegetation.instance_count(),
		"load_ms": data.load_ms, "fine_build_ms_max": terrain.build_stats.get("fine_build_ms_max", 0.0),
	}))
	world.queue_free()
	await process_frame
