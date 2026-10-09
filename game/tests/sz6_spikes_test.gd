extends TestCase

## Test headless du lot SZ6 (pics d'images côté scripts) sur les vraies données et la pyramide de
## relief en cache (sauté, avec un message, sans quadtree) :
##  1. instantané des pages (`surface_snapshot`) : même surface que `TerrainBuilder.surface_height_at`
##     (base des calculs déplacés dans des fils) ;
##  2. `RoadRenderer` : rubans construits dans un fil identiques à la construction synchrone, et
##     construits sans `flush` au fil des images ;
##  3. `LandmarkModel` : cuisson des hauteurs sur instantané identique à la cuisson synchrone ;
##  4. `Vegetation._with_mesh` : même MultiMesh (nombre, format, instances visibles), autre maillage ;
##  5. `LifeEffects._changed_points` : mêmes points que le parcours complet ;
##  6. `TerrainBuilder` : changements de niveau étalés dans une image ouverte, tous hors image.
## Usage : godot --headless --path game --script res://tests/sz6_spikes_test.gd


func _init() -> void:
	await process_frame
	# HC1 (ADR 0161) : ce test porte sur les arbres 1:1 (style `real`, défaut d'avant HC1).
	MapPropScale.set_tree_style(MapPropScale.TREE_STYLE_REAL)
	await _run()
	ModelLibrary.clear_cache()
	finish()


func _run() -> void:
	var data_dir := MAP_PATHS.default_data_dir()
	var map_dir := data_dir.path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var data := SettlementData.load_from(data_dir, map_dir)
	var paris_px: Vector2 = data.get_settlement("set_paris")["px"]
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	_check_changed_points(terrain)
	_check_with_mesh()
	if terrain.quadtree == null:
		print("sz6_spikes_test: no relief pyramid in cache, quadtree checks skipped")
		return
	var camera := Camera3D.new()
	world.add_child(camera)
	var focus := Vector3(paris_px.x, map_data.surface_world_at(paris_px.x, paris_px.y), paris_px.y + 10.0)
	camera.position = focus + Vector3(0.0, 30.0, 45.0)
	camera.look_at_from_position(camera.position, focus, Vector3.UP)
	camera.current = true
	var tiers := ZoomTiers.load_default()
	for _i in 40:
		terrain.update_lod(camera.global_position, 55.0, focus, tiers.fine_terrain_distance)
		terrain.quadtree.wait_jobs()
		if terrain.quadtree.is_settled():
			break
		await process_frame
	check(terrain.quadtree.page_count() > 0, "no relief page loaded near Paris")

	# 1. Instantané des pages.
	var rect := Rect2(paris_px - Vector2(40.0, 40.0), Vector2(80.0, 80.0))
	var snapshot := terrain.quadtree.surface_snapshot(rect, Vector2.ZERO)
	var worst := 0.0
	for k in 400:
		var p := rect.position + Vector2(fposmod(k * 7.31, 80.0), fposmod(k * 3.17, 80.0))
		worst = maxf(worst, absf(maxf(ReliefQuadtree.sample_snapshot(snapshot, p.x, p.y), 0.0) - terrain.surface_height_at(p.x, p.y)))
	check(worst < 1e-5, "snapshot surface differs from surface_height_at by %.6f" % worst)

	# 2. Rubans de route.
	var roads := RoadRenderer.new()
	world.add_child(roads)
	roads.build(map_data, data, terrain)
	var index := terrain.chunk_index_at(paris_px.x, paris_px.y)
	if check(roads._runs_by_chunk.has(index), "no road run in the Paris chunk"):
		var runs: Array = roads._runs_by_chunk[index]
		var sync := RoadRenderer.ribbon_arrays(runs, roads.sample_step, roads.lift, {}, terrain)
		var threaded := RoadRenderer.ribbon_arrays(runs, roads.sample_step, roads.lift, terrain.quadtree.surface_snapshot(roads._run_bounds[index], Vector2.ZERO), null)
		check(_same_arrays(sync, threaded), "threaded road ribbon differs from the synchronous one")
	var roads_deadline := Time.get_ticks_msec() + 10000
	while Time.get_ticks_msec() < roads_deadline:
		roads.update_view(0.0, 1.0)
		if roads.ribbon_count() > 0 and roads.pending_jobs() == 0:
			break
		await process_frame
	check(roads.ribbon_count() > 0, "no road ribbon built by worker threads")

	# 3. Maquette de ville emblématique.
	var plan := LandmarkLibrary.for_settlement("set_paris")
	var landmark := LandmarkModel.create(plan, terrain) if not plan.is_empty() else null
	if landmark == null:
		print("sz6_spikes_test: Paris landmark model not imported, bake check skipped")
	else:
		world.add_child(landmark)
		landmark._start_bake()
		landmark._continue_bake(INF)
		var sync_image: Image = landmark._bake_image.duplicate()
		landmark._bake_row = -1
		landmark._bake_snapshot = terrain.quadtree.surface_snapshot(Rect2(landmark._origin, Vector2(landmark._extent, landmark._extent)).grow(1.0), Vector2.ZERO)
		landmark._bake_rows()
		landmark._bake_snapshot = {}
		var diff := 0.0
		for j in LandmarkModel.HEIGHT_RES:
			for i in LandmarkModel.HEIGHT_RES:
				diff = maxf(diff, absf(sync_image.get_pixel(i, j).r - landmark._bake_image.get_pixel(i, j).r))
		check(diff < 1e-4, "threaded landmark bake differs by %.6f m" % diff)
		landmark._finish_bake()
		landmark._bake_pending = true
		var deadline := Time.get_ticks_msec() + 10000  # fil de travail : machine parfois très chargée
		while Time.get_ticks_msec() < deadline:
			landmark._process(0.0)
			if landmark._bake_task < 0 and not landmark._bake_pending:
				break
			await process_frame
		check(landmark._bake_task < 0 and landmark._bake_row < 0, "landmark rebake did not finish in a worker thread (task %d, row %d, pending %s)" % [landmark._bake_task, landmark._bake_row, landmark._bake_pending])

	# 6. Changements de niveau étalés.
	var far := focus + Vector3(0.0, 400.0, 400.0)
	terrain.update_lod(far, 600.0, focus, tiers.fine_terrain_distance)
	var before := _levels(terrain)
	terrain.level_emit_budget_ms = 0.0
	FrameBudget.begin_frame()
	terrain.update_lod(camera.global_position, 55.0, focus, tiers.fine_terrain_distance)
	var changed := _changed_count(before, _levels(terrain))
	check(changed == 1, "in a frame with no budget, expected exactly one level change, got %d" % changed)
	await process_frame
	terrain.update_lod(camera.global_position, 55.0, focus, tiers.fine_terrain_distance)  # hors image : tout
	var settled := _levels(terrain)
	check(_changed_count(before, settled) > 1, "expected several level changes when zooming back in")
	terrain.update_lod(camera.global_position, 55.0, focus, tiers.fine_terrain_distance)
	check(_changed_count(settled, _levels(terrain)) == 0, "level changes still pending outside a frame")


func _levels(terrain: TerrainBuilder) -> PackedInt32Array:
	var levels := PackedInt32Array()
	for i in terrain.chunks_x * terrain.chunks_y:
		levels.append(terrain.chunk_level(i))
	return levels


func _changed_count(a: PackedInt32Array, b: PackedInt32Array) -> int:
	var count := 0
	for i in a.size():
		if a[i] != b[i]:
			count += 1
	return count


func _same_arrays(a: Array, b: Array) -> bool:
	if a.size() != b.size():
		return false
	if a.is_empty():
		return true
	var va: PackedVector3Array = a[Mesh.ARRAY_VERTEX]
	var vb: PackedVector3Array = b[Mesh.ARRAY_VERTEX]
	if va.size() != vb.size() or a[Mesh.ARRAY_INDEX] != b[Mesh.ARRAY_INDEX] or a[Mesh.ARRAY_TEX_UV] != b[Mesh.ARRAY_TEX_UV]:
		return false
	for n in va.size():
		if va[n].distance_to(vb[n]) > 1e-5:
			return false
	return true


func _check_with_mesh() -> void:
	var old := MultiMesh.new()
	old.transform_format = MultiMesh.TRANSFORM_3D
	old.use_custom_data = true
	old.mesh = BoxMesh.new()
	old.instance_count = 10
	var buffer := PackedFloat32Array()
	buffer.resize(10 * 16)
	for n in 10:
		buffer[n * 16] = 1.0
		buffer[n * 16 + 5] = 1.0
		buffer[n * 16 + 10] = 1.0
		buffer[n * 16 + 3] = float(n)
	old.buffer = buffer
	old.visible_instance_count = 7
	var sphere := SphereMesh.new()
	var swapped := Vegetation._with_mesh(old, sphere, buffer)
	check(swapped.mesh == sphere and swapped.instance_count == 10 and swapped.visible_instance_count == 7 and swapped.use_custom_data and swapped.transform_format == MultiMesh.TRANSFORM_3D, "Vegetation._with_mesh lost MultiMesh settings")


func _check_changed_points(terrain: TerrainBuilder) -> void:
	var fx := LifeEffects.new()
	fx._terrain = terrain
	var points: Array = []
	for k in 300:
		points.append([Vector2(fposmod(k * 97.3, 4096.0), fposmod(k * 61.7, 4096.0)), 0.0, 0.0])
	var changed := {terrain.chunk_index_at(points[5][0].x, points[5][0].y): true, terrain.chunk_index_at(points[77][0].x, points[77][0].y): true}
	var expected: Array = []
	for n in points.size():
		var px: Vector2 = points[n][0]
		if changed.has(terrain.chunk_index_at(px.x, px.y)):
			expected.append(n)
	check(fx._changed_points("test", points, changed) == expected, "LifeEffects._changed_points differs from a full scan")
	points.append([points[5][0], 0.0, 0.0])  # liste modifiée : index refait
	expected.append(points.size() - 1)
	check(fx._changed_points("test", points, changed) == expected, "LifeEffects._changed_points not refreshed when the list grows")
	fx.free()
