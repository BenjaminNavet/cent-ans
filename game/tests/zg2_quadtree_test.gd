extends TestCase

## Test headless du lot ZG2 (relief streamé en quadtree, ADR 0036), sans dépendre du cache réel :
## une pyramide factice est écrite dans `user://zg2_test/` (tuiles E0 versionnées recopiées comme
## tuiles E1/E2 : format réel, contenu décalé) avec son manifeste.
##  1. `ReliefPyramid` : RLE, `has_tile`, `finest_level_at`, `max_level_under`, `finest_ancestor`,
##     indisponible sans cache ;
##  2. `TerrainBuilder` + quadtree au-dessus de Paris : nœuds sélectionnés, pages décodées et
##     téléversées, morceaux E0 masqués, `fine_ready`, signal `chunk_surface_changed` à l'arrivée
##     des pages ;
##  3. `surface_height_at` = bilinéaire de la page la plus fine (comparée à un décodage direct),
##     instantané `surface_grid` cohérent, repli heightmap hors pyramide ;
##  4. LRU : avec 12 couches seulement, le nombre de pages reste borné ;
##  5. quadtree absent quand le manifeste ne liste aucune tuile.
## Usage : godot --headless --path game --script res://tests/zg2_quadtree_test.gd

const TEST_DIR := "user://zg2_test"

var _changed := 0


func _init() -> void:
	await process_frame
	await _run()
	finish()


## Tuiles E1 de la tuile E0 (8, 7) (Paris) et E2 de son quart nord-ouest ; contenu = tuile E0.
func _write_fixture(map_dir: String, list_tiles: bool) -> String:
	var dir := ProjectSettings.globalize_path(TEST_DIR)
	DirAccess.make_dir_recursive_absolute(dir.path_join("pyramid/E1"))
	DirAccess.make_dir_recursive_absolute(dir.path_join("pyramid/E2"))
	var source := map_dir.path_join("height/h_8_7.png")
	for row in range(14, 16):
		for col in range(16, 18):
			DirAccess.copy_absolute(source, dir.path_join("pyramid/E1/%d_%d.png" % [col, row]))
	for row in range(28, 30):
		for col in range(32, 34):
			DirAccess.copy_absolute(source, dir.path_join("pyramid/E2/%d_%d.png" % [col, row]))
	var levels := [
		{"level": 1, "meters_per_px": 179.744, "tier": 1, "source": "test", "tiles_rle": [
			{"row": 14, "runs": [[16, 2]]}, {"row": 15, "runs": [[16, 2]]}]},
		{"level": 2, "meters_per_px": 89.872, "tier": 1, "source": "test", "tiles_rle": [
			{"row": 28, "runs": [[32, 2]]}, {"row": 29, "runs": [[32, 2]]}]},
	]
	if not list_tiles:
		for entry in levels:
			entry["tiles_rle"] = []
	var manifest := {
		"version": 1, "tile_px": 512, "root_tile_units": 256, "height_min_m": -200.0,
		"height_range_m": 5000.0, "dir": "pyramid", "pattern": "E{level}/{col}_{row}.png", "levels": levels,
	}
	var path := dir.path_join("relief_pyramid.json" if list_tiles else "empty.json")
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_string(JSON.stringify(manifest))
	file.close()
	return path


func _run() -> void:
	var map_dir := MAP_PATHS.default_data_dir().path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var manifest := _write_fixture(map_dir, true)
	var empty_manifest := _write_fixture(map_dir, false)

	# 1. Pyramide.
	var pyramid := ReliefPyramid.new()
	check(pyramid.load_manifest(map_dir, manifest), "fixture pyramid should load: %s" % pyramid.load_error)
	check(pyramid.max_level == 2 and pyramid.tile_count() == 8, "expected E2 max and 8 tiles, got E%d / %d" % [pyramid.max_level, pyramid.tile_count()])
	check(pyramid.has_tile(1, 17, 15) and not pyramid.has_tile(1, 18, 15), "has_tile E1")
	check(pyramid.has_tile(0, 8, 7), "E0 tiles from data/map/height")
	# Tuile E2 (32, 28) : [2047,5 ; 2111,5] × [1791,5 ; 1855,5].
	check(pyramid.finest_level_at(2060.0, 1800.0) == 2, "finest level in the E2 block")
	check(pyramid.finest_level_at(2200.0, 1900.0) == 1, "finest level in the E1 block")
	check(pyramid.finest_level_at(100.0, 100.0) == 0, "finest level elsewhere = E0")
	check(pyramid.max_level_under(0, 8, 7) == 2 and pyramid.max_level_under(1, 17, 15) == 1, "max_level_under")
	check(pyramid.finest_ancestor(5, 32 * 8, 28 * 8) == 2 and pyramid.finest_ancestor(3, 70, 60) == 1, "finest_ancestor")
	var none := ReliefPyramid.new()
	check(not none.load_manifest(map_dir, empty_manifest) and not none.is_available(), "empty manifest must not be available")

	# 2. Terrain + quadtree au-dessus de Paris.
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	terrain.pyramid_manifest_path = manifest
	world.add_child(terrain)
	terrain.build(map_data)
	if not check(terrain.quadtree != null, "quadtree should be active with the fixture pyramid"):
		return
	terrain.chunk_surface_changed.connect(func(_i: int) -> void: _changed += 1)
	var hidden := true
	for child in terrain.get_children():
		if child is MeshInstance3D and child.visible:
			hidden = false
	check(hidden, "E0 chunks should be hidden under the quadtree")
	var camera := Camera3D.new()
	world.add_child(camera)
	var paris := Vector2(2213.2, 1923.9)
	var focus := Vector3(paris.x, map_data.surface_world_at(paris.x, paris.y), paris.y)
	camera.look_at_from_position(focus + Vector3(0.0, 20.0, 18.0), focus, Vector3.UP)
	camera.current = true
	for _i in 200:
		terrain.update_lod(camera.global_position, 27.0, focus, 170.0)
		await process_frame
		if terrain.fine_ready() and terrain.quadtree.page_count() >= 4:
			break
		terrain.wait_fine_jobs()
	var qt := terrain.quadtree
	print("zg2_quadtree_test: %s" % JSON.stringify(qt.perf_stats()))
	check(qt.item_count() > 8, "expected several quadtree nodes, got %d" % qt.item_count())
	check(qt.page_count() >= 4, "expected E1/E2 pages resident, got %d" % qt.page_count())
	check(terrain.fine_ready(), "quadtree should settle")
	check(terrain.chunk_level(terrain.chunk_index_at(paris.x, paris.y)) == 2, "Paris chunk should be at level 2")
	check(_changed > 0, "chunk_surface_changed should fire when pages arrive")
	# Vue parchemin (caméra nulle) : plus rien de voulu, le quadtree doit se déclarer stable
	# (régression PB1 : `is_settled()` restait faux à d = 1500).
	qt.update_view(camera)
	for _i in 60:
		qt.update_view(null)
		await process_frame
		if qt.is_settled():
			break
	check(qt.is_settled(), "quadtree should settle when the camera is gone (parchment view)")

	# 3. Surface : E1 (17, 15) = contenu de h_8_7 ; son pixel (100, 200) est centré en
	#    origine + (100,5 ; 200,5) × 0,25.
	var decoded: PackedByteArray = ClassDB.instantiate("GameDataStore").call("load_heightmap_u16", map_dir.path_join("height/h_8_7.png")) \
		if ClassDB.class_exists("GameDataStore") else PackedByteArray()
	if decoded.is_empty():
		var big := Png16.load_gray16(map_dir.path_join("height/h_8_7.png"))
		var data: PackedByteArray = big["data"]
		decoded.resize(data.size())
		for o in range(0, data.size(), 2):
			decoded[o] = data[o + 1]
			decoded[o + 1] = data[o]
	var origin := ReliefPyramid.tile_origin(1, 17, 15)
	var x := origin.x + 100.5 * 0.25
	var y := origin.y + 200.5 * 0.25
	# ZG8 : hauteur affichée (relief local exagéré) de l'altitude de la page.
	var expected := MapData.display_height(-200.0 + decoded.decode_u16((200 * 512 + 100) * 2) / 65535.0 * 5000.0, x, y)
	if qt.surface_height_at(x, y) == qt.surface_height_at(x, y):  # page E1 chargée
		check(absf(qt.surface_height_at(x, y) - expected) < 1e-4, "E1 surface %f vs %f" % [qt.surface_height_at(x, y), expected])
		var index := terrain.chunk_index_at(x, y)
		var grid := terrain.surface_grid(index)
		var local := Vector2(x, y) - Vector2((index % terrain.chunks_x) * terrain.chunk_px, (index / terrain.chunks_x) * terrain.chunk_px)
		check(absf(TerrainBuilder.grid_height(grid, local.x, local.y) - qt.surface_height_at(x, y)) < 1e-4, "snapshot grid matches surface")
	else:
		check(false, "E1 page (17, 15) should be resident")
	var t_probe := Time.get_ticks_usec()
	for k in 20000:
		terrain.surface_height_at(paris.x + (k % 141) * 0.07, paris.y + (k / 141) * 0.07)
	print("zg2_quadtree_test: surface_height_at %.2f us/call" % ((Time.get_ticks_usec() - t_probe) / 20000.0))
	check(is_nan(qt.surface_height_at(100.0, 100.0)), "no page far from Paris")
	check(absf(terrain.surface_height_at(100.0, 100.0) - map_data.surface_world_at(100.0, 100.0)) < 1e-4, "heightmap fallback outside pages")

	# 4. LRU borné.
	terrain.queue_free()
	await process_frame
	var small := TerrainBuilder.new()
	small.pyramid_manifest_path = manifest
	world.add_child(small)
	small.build(map_data)
	small.quadtree.max_pages = 12
	small.quadtree.setup(small.pyramid, small.material, map_data, small._chunk_bounds_m())
	for i in 60:
		var p := paris + Vector2(-150.0 + i * 5.0, -60.0 + i * 2.0)
		var f := Vector3(p.x, map_data.surface_world_at(p.x, p.y), p.y)
		camera.look_at_from_position(f + Vector3(0.0, 12.0, 10.0), f, Vector3.UP)
		small.update_lod(camera.global_position, 16.0, f, 170.0)
		small.wait_fine_jobs()
		await process_frame
	check(small.quadtree.page_count() <= 12, "LRU should bound resident pages, got %d" % small.quadtree.page_count())

	# 5. Sans tuile listée : repli sans quadtree.
	var fallback := TerrainBuilder.new()
	fallback.pyramid_manifest_path = empty_manifest
	world.add_child(fallback)
	fallback.build(map_data)
	check(fallback.quadtree == null, "no quadtree without pyramid tiles")
	world.queue_free()
	await process_frame
