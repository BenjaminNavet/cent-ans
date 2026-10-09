extends TestCase

## Test headless du lot OM1 (ADR 0115) : monde rectangulaire synthétique 7168 × 6144.
## - `ReliefPyramid` : grille E0 28 × 24 lue dans map.json, décalage `root_origin_tiles` [0, 5]
##   (tuile du cache (k, c, r) = tuile monde (c, r + 5·2^k)), chemins de fichiers en coordonnées
##   de cache, présence et ancêtres en coordonnées monde.
## - `ReliefQuadtree` : racines à la profondeur 2 (7 × 6 nœuds de 1024), vue lointaine = 42 nœuds,
##   sélection GDScript identique à `ReliefLod` (natif) sur plusieurs vues, y compris à l'est de
##   x = 4096 et au sud de y = 4096.
## - `CampaignCamera` : bornes 7168 × 6144 (le point visé reste dans le rectangle).
## - `TerrainBuilder.chunk_grid_for` : 28 × 24 tuiles de 256 (16 × 16 pour 4096², inchangé).
## Usage : godot --headless --path game --script res://tests/om1_wide_world_test.gd

const WORLD := Vector2i(7168, 6144)
const ORIGIN := Vector2i(0, 5)

var _dir := ""


func _init() -> void:
	await process_frame
	_dir = OS.get_user_data_dir().path_join("om1_wide_world")
	await _run()
	finish()


func _run() -> void:
	_test_chunk_grid()
	var pyramid := _test_pyramid()
	if pyramid != null:
		await _test_quadtree(pyramid)
	_test_camera()


func _test_chunk_grid() -> void:
	check(TerrainBuilder.chunk_grid_for(Vector2i(4096, 4096)) == Vector3i(256, 16, 16), "4096² keeps 16 × 16 chunks of 256")
	check(TerrainBuilder.chunk_grid_for(WORLD) == Vector3i(256, 28, 24), "7168 × 6144 has 28 × 24 chunks of 256")
	check(TerrainBuilder.chunk_grid_for(Vector2i(512, 512)) == Vector3i(32, 16, 16), "512² test map keeps 16 × 16 chunks")
	check(ReliefQuadtree.root_depth(16, 16) == 0 and ReliefQuadtree.root_depth(28, 24) == 2, "root depth mirrors relief_lod")


## Jeu de données synthétique : map.json 7168 × 6144 (sans tuiles E0), manifeste E1 avec décalage
## [0, 5] et fichiers vides nommés comme le cache.
func _test_pyramid() -> ReliefPyramid:
	DirAccess.make_dir_recursive_absolute(_dir.path_join("pyramid/E1"))
	var meta := {"crs": "EPSG:3035", "size_px": [WORLD.x, WORLD.y], "meters_per_px": 718.9765625}
	_write(_dir.path_join("map.json"), JSON.stringify(meta))
	# Cache E1 : lignes 6-7, colonnes 40-41 (au sud-est de l'ancienne carte une fois décalées de 10 lignes).
	var manifest := {
		"version": 1, "tile_px": 512, "root_tile_units": 256, "height_min_m": -200.0, "height_range_m": 5000.0,
		"dir": "pyramid", "pattern": "E{level}/{col}_{row}.png", "root_origin_tiles": [ORIGIN.x, ORIGIN.y],
		"levels": [{"level": 1, "tiles_rle": [{"row": 6, "runs": [[40, 2]]}, {"row": 7, "runs": [[40, 2]]}]}],
	}
	_write(_dir.path_join("relief_pyramid.json"), JSON.stringify(manifest))
	for row in [6, 7]:
		for col in [40, 41]:
			_write(_dir.path_join("pyramid/E1/%d_%d.png" % [col, row]), "")
	var pyramid := ReliefPyramid.new()
	var ok := pyramid.load_manifest(_dir)
	if not check(ok, "synthetic pyramid should load: %s" % pyramid.load_error):
		return null
	check(pyramid.root_cols == 28 and pyramid.root_rows == 24, "root grid %d × %d" % [pyramid.root_cols, pyramid.root_rows])
	check(pyramid.origin_tiles == ORIGIN, "origin offset read from the manifest")
	check(pyramid.cols(1) == 56 and pyramid.rows(1) == 48, "E1 grid 56 × 48")
	# Cache (40, 6) → monde (40, 16).
	check(pyramid.has_tile(1, 40, 16) and pyramid.has_tile(1, 41, 17), "offset tiles present in world coordinates")
	check(not pyramid.has_tile(1, 40, 6), "cache coordinates are not world coordinates")
	check(pyramid.tile_path(1, 41, 17).ends_with("E1/41_7.png"), "file path back in cache coordinates: %s" % pyramid.tile_path(1, 41, 17))
	check(pyramid.max_level_under(0, 20, 8) == 1, "ancestor index in world coordinates")
	check(pyramid.finest_level_at(40 * 128.0 + 10.0, 16 * 128.0 + 10.0) == 1, "finest level at a world point")
	check(ReliefPyramid.cache_to_world(3, 5, 7, ORIGIN) == Vector2i(5, 47), "cache_to_world at E3")
	check(ReliefPyramid.world_to_cache(3, 5, 47, ORIGIN) == Vector2i(5, 7), "world_to_cache at E3")
	return pyramid


func _test_quadtree(pyramid: ReliefPyramid) -> void:
	var bounds := PackedVector2Array()
	bounds.resize(pyramid.root_cols * pyramid.root_rows)
	bounds.fill(Vector2(0.0, 100.0))
	var qt := ReliefQuadtree.new()
	root.add_child(qt)
	qt.setup_selection(pyramid, bounds)
	check(qt.root_depth_used() == 2, "root depth 2 for 28 × 24")
	var camera := Camera3D.new()
	camera.fov = 60.0
	camera.far = 200000.0
	root.add_child(camera)
	await process_frame
	# Vue lointaine au zénith : toutes les racines, aucune subdivision.
	camera.look_at_from_position(Vector3(3584.0, 60000.0, 3072.0), Vector3(3584.0, 0.0, 3072.1))
	qt.max_vertex_px = 1000.0
	if not qt.is_native():
		check(false, "ReliefLod (native) unavailable: build core/build.sh")
		return
	qt._prepare_camera(camera)
	qt._native_update()
	var far_keys: PackedInt64Array = (qt._native.call("items") as Dictionary)["keys"]
	check(far_keys.size() == 42, "far view: %d nodes, 42 roots expected" % far_keys.size())
	var area := 0.0
	for key: int in far_keys:
		var size := qt.node_size(key >> 40)
		area += size * size
	check(is_equal_approx(area, float(WORLD.x * WORLD.y)), "roots cover the world exactly")
	qt.max_vertex_px = 4.0
	var views := [
		[Vector3(3584.0, 60000.0, 3072.0), Vector3(3584.0, 0.0, 3072.1)],
		[Vector3(5200.0, 40.0, 2300.0), Vector3(5250.0, 0.0, 2150.0)],  # sur les tuiles E1 décalées
		[Vector3(6900.0, 300.0, 5900.0), Vector3(6600.0, 0.0, 5600.0)],  # coin sud-est
		[Vector3(1000.0, 800.0, 5000.0), Vector3(1500.0, 0.0, 4600.0)],
	]
	for view: Array in views:
		camera.look_at_from_position(view[0], view[1])
		qt._prepare_camera(camera)
		qt._native_update()
		var keys: PackedInt64Array = (qt._native.call("items") as Dictionary)["keys"]
		check(keys.size() > 0, "%s: empty selection" % str(view[0]))
	# Vue sur les tuiles décalées : la page E1 monde (40 + i, 16 + j) est voulue.
	camera.look_at_from_position(Vector3(5200.0, 40.0, 2150.0), Vector3(5250.0, 0.0, 2100.0))
	qt._prepare_camera(camera)
	qt._native_update()
	var wanted_levels := {}
	for key: int in qt._wanted_order:
		wanted_levels[ReliefPyramid.level_of_key(key)] = true
		if ReliefPyramid.level_of_key(key) == 1:
			check(pyramid.has_tile(1, ReliefPyramid.col_of_key(key), ReliefPyramid.row_of_key(key)), "wanted E1 page listed by the pyramid")
	check(wanted_levels.has(1), "E1 pages of the offset cache wanted near them")
	qt.queue_free()
	camera.queue_free()


func _test_camera() -> void:
	var rig := CampaignCamera.new()
	var camera := Camera3D.new()
	camera.name = "Camera3D"
	rig.add_child(camera)
	root.add_child(rig)
	rig.edge_pan_enabled = false
	rig.setup(Rect2(Vector2.ZERO, Vector2(WORLD)), 500.0)
	check(is_equal_approx(rig.target_focus.x, 3584.0) and is_equal_approx(rig.target_focus.z, 3072.0), "camera starts at the world centre")
	rig._move_target(Vector3(10000.0, 0.0, 10000.0))
	check(is_equal_approx(rig.target_focus.x, 7168.0) and is_equal_approx(rig.target_focus.z, 6144.0), "camera clamped to 7168 × 6144: %s" % rig.target_focus)
	rig._move_target(Vector3(-20000.0, 0.0, -20000.0))
	check(rig.target_focus.x == 0.0 and rig.target_focus.z == 0.0, "camera clamped to the origin")
	rig.queue_free()


func _write(path: String, text: String) -> void:
	var f := FileAccess.open(path, FileAccess.WRITE)
	if f != null:
		f.store_string(text)
		f.close()
