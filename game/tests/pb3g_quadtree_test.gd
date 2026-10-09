extends TestCase

## Test headless du lot PB3g (sélection native du quadtree de relief, ADR 0092) sur les vraies
## données et la pyramide en cache (sauté, avec un message, sans quadtree ou sans extension) :
##  1. pour plusieurs caméras (France entière, Paris de haut, Paris proche, au ras du sol vers
##     l'horizon, Loire en oblique) et plusieurs images, la sélection native (`ReliefLod`) et la
##     sélection GDScript sur le même état des pages donnent les mêmes nœuds (clés), les mêmes
##     pages fines et grossières, les mêmes pages voulues ;
##  2. les nœuds affichés correspondent à la sélection (un `MeshInstance3D` visible par nœud) ;
##  3. `VegetationScatter.request_reground` : même résultat avec les pages partagées du magasin
##     natif (`qt_store`) qu'avec la copie des octets ;
##  4. hauteurs groupées `TerrainBuilder.surface_heights_at` (bilinéaire natif) = point par point ;
##  5. tampon des hameaux écrit en une fois (`SettlementLayer.hamlet_buffer`).
## Usage : godot --headless --path game --script res://tests/pb3g_quadtree_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	if not ClassDB.class_exists("ReliefLod"):
		print("pb3g_quadtree_test: extension without ReliefLod, skipped")
		return
	var map_dir := MAP_PATHS.default_data_dir().path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var world := Node3D.new()
	root.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	if terrain.quadtree == null:
		print("pb3g_quadtree_test: no relief pyramid in cache, skipped")
		return
	var qt := terrain.quadtree
	check(qt.is_native(), "quadtree should use the native selection")
	var camera := Camera3D.new()
	world.add_child(camera)
	camera.current = true
	var paris := Vector2(2213.2, 3203.9)
	var loire := Vector2(1850.0, 3610.0)
	var views := [
		["france", Vector2(2048.0, 3328.0), Vector3(0.0, 2600.0, 1400.0)],
		["paris_150", paris, Vector3(0.0, 110.0, 100.0)],
		["paris_40", paris, Vector3(0.0, 25.0, 30.0)],
		["paris_ground", paris, Vector3(0.0, 1.5, 6.0)],
		["loire_oblique", loire, Vector3(60.0, 18.0, 70.0)],
	]
	var compared := 0
	for view: Array in views:
		var at: Vector2 = view[1]
		var focus := Vector3(at.x, map_data.surface_world_at(at.x, at.y), at.y)
		camera.look_at_from_position(focus + (view[2] as Vector3), focus, Vector3.UP)
		var d := (view[2] as Vector3).length()
		for frame in 6:
			terrain.update_lod(camera.global_position, d, focus, 170.0)
			compared += _compare(qt, camera, "%s/%d" % [view[0], frame])
			if frame % 2 == 1:
				qt.wait_jobs()
			await process_frame
		_check_slots(qt, str(view[0]))
	check(compared >= 25, "too few comparisons: %d" % compared)
	print("pb3g_quadtree_test: %d selections compared, %d pages resident" % [compared, qt.page_count()])
	_check_reground(qt, paris)
	_check_heights(terrain, paris)
	_check_hamlet_buffer()


## Hauteurs groupées (bilinéaire natif) = `surface_height_at` point par point.
func _check_heights(terrain: TerrainBuilder, at: Vector2) -> void:
	var points := PackedVector2Array()
	for k in 500:
		points.append(at + Vector2(fposmod(k * 7.31, 600.0) - 300.0, fposmod(k * 3.17, 600.0) - 300.0))
	points.append(Vector2(-5.0, 10.0))
	var batch := terrain.surface_heights_at(points)
	var worst := 0.0
	for n in points.size():
		worst = maxf(worst, absf(batch[n] - float(terrain.surface_height_at(points[n].x, points[n].y))))
	check(worst < 1e-4, "surface_heights_at differs from surface_height_at by %f" % worst)


## Tampon des hameaux écrit en une fois = transformations attendues, dans la disposition de
## `MultiMesh.buffer` (lignes de la base, puis origine ; le serveur factice headless ne relit pas
## les instances, d'où la comparaison directe du tampon).
func _check_hamlet_buffer() -> void:
	var entries: Array = []
	for i in 5:
		entries.append(PackedFloat32Array([100.0 + i, 200.0 - i, i * 1.3, 0.4 + i * 0.05, 1.0 + i * 0.1, 0.9 + i * 0.1]))
	var s := 0.7
	var buffer := SettlementLayer.hamlet_buffer(entries, s)
	check(buffer.size() == entries.size() * 12, "hamlet buffer size")
	for t in entries.size():
		var e: PackedFloat32Array = entries[t]
		var x := Transform3D(Basis(Vector3.UP, e[2]).scaled(Vector3.ONE * (e[3] * s)), Vector3(e[0], lerpf(e[4], e[5], s) - 0.03 * s, e[1]))
		var expected := PackedFloat32Array([
			x.basis.x.x, x.basis.y.x, x.basis.z.x, x.origin.x,
			x.basis.x.y, x.basis.y.y, x.basis.z.y, x.origin.y,
			x.basis.x.z, x.basis.y.z, x.basis.z.z, x.origin.z,
		])
		check(buffer.slice(t * 12, t * 12 + 12) == expected, "hamlet buffer transform %d differs" % t)


## Compare la sélection native de la dernière image à la sélection GDScript ; 1 si comparée.
func _compare(qt: ReliefQuadtree, camera: Camera3D, label: String) -> int:
	var cmp := qt.compare_selection(camera)
	if not check(not cmp.is_empty(), "%s: no comparison" % label):
		return 0
	var native: Dictionary = cmp["native"]
	var gd: Dictionary = cmp["gdscript"]
	var keys_n: PackedInt64Array = native["keys"]
	var keys_g: PackedInt64Array = gd["keys"]
	check(keys_n.size() > 0, "%s: empty selection" % label)
	if not check(keys_n == keys_g, "%s: node keys differ (%d native / %d GDScript)" % [label, keys_n.size(), keys_g.size()]):
		return 1
	check(native["fine"] == gd["fine"], "%s: fine pages differ" % label)
	check(native["coarse"] == gd["coarse"], "%s: coarse pages differ" % label)
	var dist_n: PackedFloat64Array = native["dist"]
	var dist_g: PackedFloat64Array = gd["dist"]
	var worst := 0.0
	for i in dist_n.size():
		worst = maxf(worst, absf(dist_n[i] - dist_g[i]))
	check(worst == 0.0, "%s: distances differ by %f" % [label, worst])
	check(int(cmp["native_missing"]) == int(cmp["gd_missing"]), "%s: missing pages %d / %d" % [label, cmp["native_missing"], cmp["gd_missing"]])
	check(cmp["native_wanted"] == cmp["gd_wanted"], "%s: wanted pages differ" % label)
	return 1


## Un nœud visible par élément sélectionné, aux clés de la sélection.
func _check_slots(qt: ReliefQuadtree, label: String) -> void:
	var visible := 0
	for child in qt.get_children():
		if child is MeshInstance3D and child.visible:
			visible += 1
	check(visible == qt.item_count(), "%s: %d visible nodes for %d items" % [label, visible, qt.item_count()])


## Recalage natif : pages partagées (`qt_store`) = pages copiées.
func _check_reground(qt: ReliefQuadtree, at: Vector2) -> void:
	if not ClassDB.class_exists("VegetationScatter"):
		return
	var origin := Vector2(floorf(at.x / 256.0) * 256.0, floorf(at.y / 256.0) * 256.0)
	var shared := qt.surface_snapshot(Rect2(origin, Vector2(256.0, 256.0)), origin)
	check(shared["qt_store"] != null and not (shared["qt_pages"] as Dictionary).is_empty(), "snapshot should carry the native store and pages")
	var copied := shared.duplicate()
	copied.erase("qt_store")
	var results := []
	for grid: Dictionary in [shared, copied]:
		var scatter: Object = ClassDB.instantiate("VegetationScatter")
		scatter.call("start", 1)
		var buffer := PackedFloat32Array()
		for i in 64:
			var x := fposmod(i * 37.3, 256.0)
			var z := fposmod(i * 91.7, 256.0)
			buffer.append_array([1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, z, 0.0, 0.0, 0.0, 0.0])
		var buffers: Array = [buffer]
		var t0 := Time.get_ticks_usec()
		scatter.call("request_reground", 1, buffers, grid, origin, MapData.vertical_scale(), MapData.relief_gain(), MapData.relief_squash())
		print("pb3g_quadtree_test: request_reground %s, %d pages: %.2f ms" % ["shared" if grid.has("qt_store") else "copied", (grid["qt_pages"] as Dictionary).size(), (Time.get_ticks_usec() - t0) / 1000.0])
		var polled: Array = []
		for _i in 2000:
			polled = scatter.call("poll", 1)
			if not polled.is_empty():
				break
			OS.delay_msec(1)
		if not check(not polled.is_empty(), "reground did not answer"):
			return
		results.append((polled[0]["buffers"] as Array)[0])
	check(results[0] == results[1], "reground with shared pages differs from copied pages")
	var moved := false
	for i in range(7, (results[0] as PackedFloat32Array).size(), 16):
		moved = moved or absf((results[0] as PackedFloat32Array)[i]) > 1e-6
	check(moved, "reground should seat the instances on the relief")
