extends SceneTree

## Test headless du lot ZG7c : pyramide de relief trouée (cache partiel, cuisson interrompue).
## Une pyramide factice est écrite dans `user://zg7c_test/` (tuiles E0 versionnées recopiées comme
## tuiles E1-E3) ; le manifeste liste plus de tuiles que le disque n'en a :
##  - E1 : 4 tuiles listées, la première (16, 14) absente ;
##  - E2 : 5 tuiles listées, la première (32, 28), (33, 29) et (34, 28) absentes, (33, 28) corrompue ;
##  - E3 : 4 tuiles sous la tuile E2 absente (33, 29), toutes présentes (trou au milieu de la chaîne).
## Attendu : l'étage n'est pas ignoré en bloc, chaque trou retombe sur l'ancêtre le plus fin présent,
## la tuile corrompue est écartée à l'exécution, et le quadtree finit par se stabiliser.
## Usage : godot --headless --path game --script res://tests/zg7c_partial_cache_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const TEST_DIR := "user://zg7c_test"

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("zg7c_partial_cache_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("zg7c_partial_cache_test: " + message)
	return condition


func _write_fixture(map_dir: String) -> String:
	var dir := ProjectSettings.globalize_path(TEST_DIR)
	# Repart d'un dossier propre (tuiles d'un essai précédent).
	for level in [1, 2, 3]:
		var level_dir := dir.path_join("pyramid/E%d" % level)
		DirAccess.make_dir_recursive_absolute(level_dir)
		for name in DirAccess.get_files_at(level_dir):
			DirAccess.remove_absolute(level_dir.path_join(name))
	var source := map_dir.path_join("height/h_8_7.png")
	for row in range(14, 16):
		for col in range(16, 18):
			if not (col == 16 and row == 14):
				DirAccess.copy_absolute(source, dir.path_join("pyramid/E1/%d_%d.png" % [col, row]))
	DirAccess.copy_absolute(source, dir.path_join("pyramid/E2/32_29.png"))
	var broken := FileAccess.open(dir.path_join("pyramid/E2/33_28.png"), FileAccess.WRITE)
	broken.store_string("not a png")
	broken.close()
	for row in range(58, 60):
		for col in range(66, 68):
			DirAccess.copy_absolute(source, dir.path_join("pyramid/E3/%d_%d.png" % [col, row]))
	# Reliquat d'une écriture interrompue (écriture puis renommage) : jamais pris pour une tuile.
	DirAccess.copy_absolute(source, dir.path_join("pyramid/E3/66_60.part.png"))
	var levels := [
		{"level": 1, "meters_per_px": 179.744, "tier": 1, "source": "test", "tiles_rle": [
			{"row": 14, "runs": [[16, 2]]}, {"row": 15, "runs": [[16, 2]]}]},
		{"level": 2, "meters_per_px": 89.872, "tier": 1, "source": "test", "tiles_rle": [
			{"row": 28, "runs": [[32, 3]]}, {"row": 29, "runs": [[32, 2]]}]},
		{"level": 3, "meters_per_px": 44.936, "tier": 2, "source": "test", "tiles_rle": [
			{"row": 58, "runs": [[66, 2]]}, {"row": 59, "runs": [[66, 2]]}, {"row": 60, "runs": [[66, 1]]}]},
	]
	var manifest := {
		"version": 1, "tile_px": 512, "root_tile_units": 256, "height_min_m": -200.0,
		"height_range_m": 5000.0, "dir": "pyramid", "pattern": "E{level}/{col}_{row}.png", "levels": levels,
	}
	var path := dir.path_join("relief_pyramid.json")
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_string(JSON.stringify(manifest))
	file.close()
	return path


func _run() -> void:
	var map_dir := MAP_PATHS.default_data_dir().path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not _check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var manifest := _write_fixture(map_dir)

	# 1. Pyramide : les étages ne sont pas ignorés en bloc, seuls les trous le sont.
	var pyramid := ReliefPyramid.new()
	_check(pyramid.load_manifest(map_dir, manifest), "holed pyramid should load: %s" % pyramid.load_error)
	_check(pyramid.max_level == 3, "E3 should stay available, got E%d" % pyramid.max_level)
	# Listées : 4 + 5 + 5 ; absentes : E1 (16, 14), E2 (32, 28), (33, 29), (34, 28), E3 (66, 60).
	_check(pyramid.missing_tiles == 5, "expected 5 missing tiles, got %d" % pyramid.missing_tiles)
	_check(pyramid.tile_count() == 9, "expected 9 tiles on disk, got %d" % pyramid.tile_count())
	_check(not pyramid.has_tile(1, 16, 14) and pyramid.has_tile(1, 17, 14), "E1 hole")
	_check(not pyramid.has_tile(2, 32, 28) and pyramid.has_tile(2, 32, 29), "E2 hole")
	_check(not pyramid.has_tile(3, 66, 60), "a .part.png leftover is not a tile")
	# Trou E1 (16, 14) : retombe sur E0 ; trou E2 (32, 28) : sur E1 (16, 14) absent → E0.
	_check(pyramid.finest_ancestor(1, 16, 14) == 0, "E1 hole falls back to E0")
	_check(pyramid.finest_ancestor(2, 32, 28) == 0, "E2 hole under an E1 hole falls back to E0")
	_check(pyramid.finest_ancestor(2, 34, 28) == 1, "E2 hole falls back to E1")
	_check(pyramid.finest_ancestor(3, 66, 58) == 3, "E3 tile under a missing E2 tile stays usable")
	_check(pyramid.max_level_under(2, 33, 29) == 3, "E3 below the E2 hole is indexed")
	_check(pyramid.finest_ancestor(5, 66 * 4, 58 * 4) == 3, "finest ancestor of an E5 key")

	# 2. Quadtree au-dessus des trous : doit se stabiliser malgré la tuile corrompue.
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	terrain.pyramid_manifest_path = manifest
	world.add_child(terrain)
	terrain.build(map_data)
	if not _check(terrain.quadtree != null, "quadtree should be active with a holed pyramid"):
		world.queue_free()
		return
	var camera := Camera3D.new()
	world.add_child(camera)
	# Centre de la tuile E2 (33, 28) corrompue, voisine du trou (33, 29) et des E3.
	var spot := ReliefPyramid.tile_origin(2, 33, 28) + Vector2(64.0, 64.0)
	var settled := false
	for d in [40.0, 10.0, 4.0]:
		var focus := Vector3(spot.x, map_data.surface_world_at(spot.x, spot.y), spot.y)
		camera.look_at_from_position(focus + Vector3(0.0, d, d * 0.9), focus, Vector3.UP)
		camera.current = true
		settled = false
		for _i in 300:
			terrain.update_lod(camera.global_position, d, focus, 170.0)
			await process_frame
			terrain.wait_fine_jobs()
			if terrain.quadtree.is_settled():
				settled = true
				break
		_check(settled, "quadtree should settle over the holes at d = %.0f" % d)
	_check(not terrain.pyramid.has_tile(2, 33, 28), "corrupt tile should be marked broken")
	_check(terrain.quadtree.page_count() > 0, "some pages should be resident")
	_check(terrain.fine_ready(), "fine_ready after settling")
	var h := terrain.surface_height_at(spot.x, spot.y)
	_check(is_finite(h), "surface height over the corrupt tile should be finite")
	world.queue_free()
	await process_frame
