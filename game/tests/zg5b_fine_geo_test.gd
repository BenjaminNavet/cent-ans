extends TestCase

## Test headless du lot ZG5b (rendu de l'hydrographie fine, ADR 0036), sans dépendre du cache :
##  1. `CafvTile` : lecture d'une tuile CAFV écrite ici (en-tête 28 o, lignes, drapeaux, x/y/z/w) ;
##  2. `FineGeoStore` : index, chargement dans un fil (`request` / `poll`), `load_sync`, LRU borné,
##     ancrages ;
##  3. `FineBedCarver` : page plate creusée sous un fleuve (fond sous le niveau d'eau, berge
##     fondue, rien au loin, rien sous l'emprise d'une colonie, rien sous E2 pour un ordre < 5) ;
##  4. `FineRibbonJob` : rubans des fleuves et des routes (hauteurs en mètres, relevées au-dessus de
##     la surface), coupe sous une colonie avec pont-porte au bord de l'emprise ;
##  5. `RiverCrossings` : un pont passe sur son ancrage fin (position, niveau d'eau × échelle
##     verticale, axe en travers du courant, portée réelle) puis revient au tracé V4 ;
##  6. données réelles si le cache ZG5a est là : tuile de Rouen, lit creusé d'une page E4 réelle.
## Usage : godot --headless --path game --script res://tests/zg5b_fine_geo_test.gd

const TEST_DIR := "user://zg5b_test"


func _init() -> void:
	await process_frame
	_run()
	finish()


## Tuile CAFV : lignes (entité, flags, points [x, y, z, w]).
static func _cafv_bytes(layer: int, col: int, row: int, lines: Array) -> PackedByteArray:
	var xs := PackedFloat32Array()
	var ys := PackedFloat32Array()
	var zs := PackedFloat32Array()
	var ws := PackedFloat32Array()
	var table := PackedInt32Array()
	for line: Dictionary in lines:
		var pts: Array = line["points"]
		table.append_array([int(line["feature"]), xs.size(), pts.size(), int(line["flags"])])
		for p: Array in pts:
			xs.append(p[0])
			ys.append(p[1])
			zs.append(p[2])
			ws.append(p[3])
	var out := PackedByteArray()
	out.resize(28)
	out.encode_u8(0, 67)  # C
	out.encode_u8(1, 65)  # A
	out.encode_u8(2, 70)  # F
	out.encode_u8(3, 86)  # V
	out.encode_u16(4, 1)
	out.encode_u16(6, layer)
	out.encode_u16(8, 2)
	out.encode_u16(10, 0)
	out.encode_u32(12, col)
	out.encode_u32(16, row)
	out.encode_u32(20, lines.size())
	out.encode_u32(24, xs.size())
	out.append_array(table.to_byte_array())
	for arr in [xs, ys, zs, ws]:
		out.append_array((arr as PackedFloat32Array).to_byte_array())
	return out


func _run() -> void:
	var order5 := 5 << CafvTile.ORDER_SHIFT
	var order3 := 3 << CafvTile.ORDER_SHIFT
	# Tuile E2 (32, 29) : x ∈ [2048, 2112), y ∈ [1856, 1920). Fleuve d'ordre 5 d'ouest en est à
	# y = 1890 (niveau 48 m, 100 m de large, marée), ruisseau d'ordre 3 au nord, route principale.
	var river_lines := [
		{"feature": 7, "flags": order5 | CafvTile.FLAG_TIDAL | (1 << CafvTile.SOURCE_SHIFT), "points": [[2050.0, 1890.0, 48.0, 100.0], [2080.0, 1890.0, 47.0, 100.0], [2110.0, 1890.0, 46.0, 100.0]]},
		{"feature": 8, "flags": order3, "points": [[2090.0, 1860.0, 60.0, 5.0], [2090.0, 1880.0, 55.0, 5.0]]},
	]
	var road_lines := [
		{"feature": 3, "flags": CafvTile.FLAG_MAIN_ROAD, "points": [[2060.0, 1870.0, 52.0, 6.0], [2060.0, 1900.0, 53.0, 6.0]]},
	]
	var river_bytes := _cafv_bytes(1, 32, 29, river_lines)

	# 1. Lecture CAFV.
	var tile := CafvTile.parse(river_bytes)
	if check(tile != null, "CAFV tile not parsed"):
		check(tile.layer == 1 and tile.col == 32 and tile.row == 29, "CAFV header")
		check(tile.lines() == 2 and tile.points() == 5, "CAFV counts %d/%d" % [tile.lines(), tile.points()])
		check(CafvTile.order_of(tile.line_flags[0]) == 5 and CafvTile.source_of(tile.line_flags[0]) == 1, "CAFV flags")
		check(tile.line_flags[0] & CafvTile.FLAG_TIDAL != 0, "CAFV tidal flag")
		check(is_equal_approx(tile.z[1], 47.0) and is_equal_approx(tile.w[4], 5.0), "CAFV arrays")
		check(tile.line_bounds[0].is_equal_approx(Vector4(2050, 1890, 2110, 1890)), "CAFV bounds")
		check(tile.line_rank[0] == 7 and tile.line_rank[1] == 3, "CAFV rank (order vs width)")
	check(CafvTile.parse(PackedByteArray([1, 2, 3])) == null, "garbage accepted")

	# 2. Store : fixture dans user://.
	var dir := ProjectSettings.globalize_path(TEST_DIR)
	DirAccess.make_dir_recursive_absolute(dir.path_join("pyramid/hydro_fine/E2"))
	DirAccess.make_dir_recursive_absolute(dir.path_join("pyramid/roads_fine/E2"))
	_write(dir.path_join("pyramid/hydro_fine/E2/32_29.bin"), river_bytes)
	_write(dir.path_join("pyramid/hydro_fine/E2/33_29.bin"), _cafv_bytes(1, 33, 29, []))
	_write(dir.path_join("pyramid/hydro_fine/E2/34_29.bin"), _cafv_bytes(1, 34, 29, []))
	_write(dir.path_join("pyramid/roads_fine/E2/32_29.bin"), _cafv_bytes(2, 32, 29, road_lines))
	var tiles := []
	for c in [32, 33, 34]:
		tiles.append({"col": c, "row": 29})
	_write_json(dir.path_join("rivers_fine.json"), {"format": "CAFV", "dir": "pyramid/hydro_fine", "pattern": "E2/{col}_{row}.bin", "tiles": tiles})
	_write_json(dir.path_join("fine_anchors.json"), {
		"settlements": {"set_test": {"px": [2070.5, 1885.25], "z": 51.0, "moved_m": 120.0}},
		"hamlets": {"items": [[2071.0, 1880.0, 50.0, 0.0]]},
		"crossings": [{"id": "bridge_test", "px": [2080.0, 1890.0], "snapped": true, "dir": [1.0, 0.0], "width_m": 100.0, "z_water": 47.0, "z_deck": 58.0}],
		"roads": {"dir": "pyramid/roads_fine", "pattern": "E2/{col}_{row}.bin", "tiles": [{"col": 32, "row": 29}]},
	})
	var store := FineGeoStore.new()
	check(store.load_from(dir), "store not available")
	check(store.has_tile(1, 32, 29) and store.has_tile(2, 32, 29) and not store.has_tile(2, 33, 29), "store index")
	check(store.settlements.has("set_test") and (store.settlements["set_test"]["px"] as Vector2).is_equal_approx(Vector2(2070.5, 1885.25)), "settlement anchor")
	check(store.hamlets.size() == 1 and store.crossings.size() == 1, "hamlet / crossing anchors")
	store.request(1, 32, 29)
	check(store.pending() == 1, "request queued")
	store.poll(true)
	var loaded := store.get_tile(1, 32, 29)
	check(loaded != null and loaded.lines() == 2, "threaded load")
	store.max_cached_tiles = 2
	store.load_sync(1, 33, 29)
	store.poll()  # image suivante : les tuiles précédentes deviennent évinçables
	store.load_sync(1, 34, 29)
	check((store._tiles[1] as Dictionary).size() <= 2, "LRU bounded: %d" % (store._tiles[1] as Dictionary).size())
	check(store.load_sync(1, 40, 40) == null, "missing tile")

	# 2b. Cache dans le cadre de la pyramide (ADR 0115, `root_origin_tiles` [0, 5]) : index et points
	# en coordonnées monde (+5 × 256 = +1280 en y, +20 tuiles E2), fichiers en coordonnées de cache.
	var shifted_dir := ProjectSettings.globalize_path(TEST_DIR + "_origin")
	DirAccess.make_dir_recursive_absolute(shifted_dir.path_join("pyramid/hydro_fine/E2"))
	_write(shifted_dir.path_join("pyramid/hydro_fine/E2/32_29.bin"), river_bytes)
	_write_json(shifted_dir.path_join("rivers_fine.json"), {"format": "CAFV", "dir": "pyramid/hydro_fine", "pattern": "E2/{col}_{row}.bin", "tiles": [{"col": 32, "row": 29}]})
	_write_json(shifted_dir.path_join("relief_pyramid.json"), {"root_origin_tiles": [0, 5]})
	var shifted := FineGeoStore.new()
	check(shifted.load_from(shifted_dir), "shifted store not available")
	check(shifted.origin_tiles == Vector2i(0, 20), "E2 origin %s" % shifted.origin_tiles)
	check(shifted.has_tile(1, 32, 49) and not shifted.has_tile(1, 32, 29), "shifted index in world tiles")
	check(shifted.tile_path(1, 32, 49).ends_with("E2/32_29.bin"), "shifted path in cache tiles: %s" % shifted.tile_path(1, 32, 49))
	var moved := shifted.load_sync(1, 32, 49)
	if check(moved != null, "shifted tile not loaded"):
		check(moved.row == 49 and moved.col == 32, "shifted header %d, %d" % [moved.col, moved.row])
		check(is_equal_approx(moved.x[0], 2050.0) and is_equal_approx(moved.y[0], 1890.0 + 1280.0), "shifted points %s, %s" % [moved.x[0], moved.y[0]])
		check(moved.line_bounds[0].is_equal_approx(Vector4(2050, 3170, 2110, 3170)), "shifted bounds %s" % moved.line_bounds[0])
		check(FineGeoStore.tile_rect(32, 49).has_point(Vector2(moved.x[1], moved.y[1])), "shifted points inside their world tile")
	(shifted._tiles[1] as Dictionary).clear()
	shifted.request(1, 32, 49)
	shifted.poll(true)
	var threaded := shifted.get_tile(1, 32, 49)
	check(threaded != null and is_equal_approx(threaded.y[0], 3170.0), "threaded shifted load")

	# 3. Lit creusé : page E4 plate à 50 m contenant le fleuve (tuile E4 (131, 118) :
	# x ∈ [2096, 2112), y ∈ [1888, 1904) depuis SZ2b, ADR 0086).
	store.max_cached_tiles = 64
	var carver := FineBedCarver.new(store, 719.0, -200.0, 5000.0)
	var level := 4
	var key := ReliefPyramid.key_of(level, 131, 118)
	var origin := ReliefPyramid.tile_origin(level, 131, 118)
	check(origin.is_equal_approx(Vector2(2096.0, 1888.0)), "E4 origin %s" % origin)
	var page := _flat_page(50.0)
	var task: Object = carver.carve_job(key)
	if check(task != null, "no carve task for a page crossed by a river"):
		var carved: PackedByteArray = task.call("apply", page.duplicate())
		var px_units := ReliefPyramid.pixel_units(level)
		var h_center := _page_h(carved, origin, px_units, Vector2(2100.0, 1890.0))
		var h_bank := _page_h(carved, origin, px_units, Vector2(2100.0, 1890.0 + 60.0 / 719.0))
		var h_far := _page_h(carved, origin, px_units, Vector2(2100.0, 1900.0))
		check(h_center < 46.6 - FineBedCarver.EDGE_DEPTH, "bed not below water: %.2f" % h_center)
		check(h_bank > h_center and h_bank < 50.0, "bank not blended: %.2f" % h_bank)
		check(absf(h_far - 50.0) < 0.1, "far pixel changed: %.2f" % h_far)
		# Emprise de colonie : rien de creusé dedans.
		carver.covers = PackedVector4Array([Vector4(2100.0, 1890.0, 1.0, 0)])
		var covered: PackedByteArray = carver.carve_job(key).call("apply", page.duplicate())
		check(absf(_page_h(covered, origin, px_units, Vector2(2100.0, 1890.0)) - 50.0) < 0.1, "carved under a settlement cover")
		carver.covers = PackedVector4Array()
	check(carver.carve_job(ReliefPyramid.key_of(1, 16, 14)) == null, "E1 page carved")
	# Pages E1-E2 jamais creusées ; E3 : seul le rang ≥ 5 (le ruisseau de 5 m, rang 3, ne l'est pas).
	check(carver.carve_job(ReliefPyramid.key_of(2, 32, 29)) == null, "E2 page carved")
	var e3_key := ReliefPyramid.key_of(3, 65, 59)
	var e3_task: Object = carver.carve_job(e3_key)
	if check(e3_task != null, "no carve task for E3"):
		var e3 := (e3_task.call("apply", _flat_page(80.0)) as PackedByteArray)
		var o3 := ReliefPyramid.tile_origin(3, 65, 59)
		var u3 := ReliefPyramid.pixel_units(3)
		check(_page_h(e3, o3, u3, Vector2(2090.0, 1890.0)) < 47.0, "E3 rank-7 river not carved")
	var brook_key := ReliefPyramid.key_of(3, 65, 58)
	var brook_task: Object = carver.carve_job(brook_key)
	var brook := _flat_page(80.0)
	if brook_task != null:
		brook = brook_task.call("apply", brook)
	check(absf(_page_h(brook, ReliefPyramid.tile_origin(3, 65, 58), ReliefPyramid.pixel_units(3), Vector2(2090.0, 1870.0)) - 80.0) < 0.1, "E3 rank-3 brook carved")

	# 4. Rubans.
	var job := FineRibbonJob.new()
	job.river_tile = store.load_sync(1, 32, 29)
	job.road_tile = store.load_sync(2, 32, 29)
	job.meters_per_unit = 719.0
	job.covers = PackedVector4Array([Vector4(2080.0, 1890.0, 3.0, 5)])
	job.run()
	check(not job.river_arrays.is_empty() and not job.road_arrays.is_empty(), "ribbons empty")
	if not job.river_arrays.is_empty():
		var v: PackedVector3Array = job.river_arrays[Mesh.ARRAY_VERTEX]
		# SZ2b : l'eau traverse l'emprise, ses sommets y sont marqués (effacés par le shader tant
		# que les maquettes sont affichées).
		var uv2: PackedVector2Array = job.river_arrays[Mesh.ARRAY_TEX_UV2]
		var inside := 0
		var wrong := 0
		for k in v.size():
			var d := Vector2(v[k].x, v[k].z).distance_to(Vector2(2080.0, 1890.0))
			var mark := int(floor(uv2[k].y / FineRibbonJob.MARK_STEP))
			if d < 2.99:
				inside += 1
				wrong += 0 if mark == FineRibbonJob.INSIDE_COVER else 1
			elif d > 3.01:
				wrong += 0 if mark == 0 else 1
		check(inside > 0, "river cut inside a settlement cover")
		check(wrong == 0, "cover marks wrong on %d vertices" % wrong)
		# Zone personnalisée d'une maquette sans ville 1:1 : coupée ; d'une ville 1:1 : marquée.
		var closed := FineRibbonJob.new()
		closed.river_tile = job.river_tile
		closed.meters_per_unit = 719.0
		closed.zones = PackedVector3Array([Vector3(2080.0, 1890.0, 3.0)])
		closed.run()
		var cv: PackedVector3Array = closed.river_arrays[Mesh.ARRAY_VERTEX]
		var in_closed := 0
		for p in cv:
			if Vector2(p.x, p.z).distance_to(Vector2(2080.0, 1890.0)) < 2.9:
				in_closed += 1
		check(in_closed == 0, "river drawn inside a closed custom zone")
		var opened := FineRibbonJob.new()
		opened.river_tile = job.river_tile
		opened.meters_per_unit = 719.0
		opened.open_zones = PackedVector3Array([Vector3(2080.0, 1890.0, 3.0)])
		opened.run()
		var ov: PackedVector3Array = opened.river_arrays[Mesh.ARRAY_VERTEX]
		var ou: PackedVector2Array = opened.river_arrays[Mesh.ARRAY_TEX_UV2]
		var zone_marks := 0
		for k in ov.size():
			if Vector2(ov[k].x, ov[k].z).distance_to(Vector2(2080.0, 1890.0)) < 2.9 and int(floor(ou[k].y / FineRibbonJob.MARK_STEP)) == FineRibbonJob.INSIDE_ZONE:
				zone_marks += 1
		check(zone_marks > 0, "open zone (1:1 city) not marked")
		check(opened.gates.is_empty(), "gate bridges on an open zone")
		check(is_equal_approx(v[0].y, 48.0), "river height is the water level in metres: %.2f" % v[0].y)
		var colors: PackedColorArray = job.river_arrays[Mesh.ARRAY_COLOR]
		check(colors[0].g > 0.5, "tidal flag lost")
	check(job.gates.size() == 2, "gate bridges: %d" % job.gates.size())
	for gate in job.gates:
		check(absf((gate["px"] as Vector2).distance_to(Vector2(2080.0, 1890.0)) - 3.0) < 0.01, "gate not on the cover edge")
		check(absf((gate["dir"] as Vector2).x - 1.0) < 1e-3, "gate flow direction")
	if not job.road_arrays.is_empty():
		var rv: PackedVector3Array = job.road_arrays[Mesh.ARRAY_VERTEX]
		check(rv.size() > 4 * 2, "road not densified: %d" % rv.size())
		check(rv[0].y > 52.0 and rv[0].y < 52.5, "road height: %.2f" % rv[0].y)
		var rc: PackedColorArray = job.road_arrays[Mesh.ARRAY_COLOR]
		check(rc[0].r > 0.5, "main road flag lost")
	# Surface plus haute que l'eau (lit pas encore creusé) : l'eau passe au-dessus.
	var lifted := FineRibbonJob.new()
	lifted.river_tile = job.river_tile
	lifted.meters_per_unit = 719.0
	lifted.snapshot_scale = 0.006
	var pages := {}
	var e2_origin := ReliefPyramid.tile_origin(2, 32, 29)
	pages[ReliefPyramid.key_of(2, 32, 29)] = _flat_page(60.0)
	lifted.snapshot = {"qt_pages": pages, "max_level": 4, "h_min": -200.0, "h_range": 5000.0, "origin": e2_origin, "map": null}
	lifted.run()
	var lv: PackedVector3Array = lifted.river_arrays[Mesh.ARRAY_VERTEX]
	check(absf(lv[0].y - (60.0 + FineRibbonJob.RIVER_LIFT_M)) < 0.1, "uncarved river not lifted: %.2f" % lv[0].y)

	# 5. Pont sur son ancrage fin.
	_test_bridge(store)

	# 6. Données réelles (cache ZG5a) : Rouen.
	_test_real_cache()


func _test_bridge(store: FineGeoStore) -> void:
	var map := MapData.load_from_dir(MAP_PATHS.default_data_dir().path_join("map"))
	var renderer := RiversRenderer.new()
	renderer.map_data = map
	var crossings := RiverCrossings.new()
	crossings.renderer = renderer
	root.add_child(crossings)
	crossings._add({"id": "bridge_test", "name": "Pont", "structure": "stone", "px": Vector2(2080.3, 1890.4), "dir": Vector2(0.0, 1.0), "width": 0.4, "type": "bridge", "index": 0})
	var item: Dictionary = crossings.items[0]
	var node: MeshInstance3D = item["node"]
	check(node != null, "bridge not instantiated")
	crossings.set_fine_anchors(store.crossings)
	crossings.set_fine_mode(true)
	check(Vector2(node.position.x, node.position.z).is_equal_approx(Vector2(2080.0, 1890.0)), "bridge not on its fine anchor: %s" % node.position)
	check(absf(node.position.y - 47.0 * MapData.vertical_scale()) < 1e-4, "bridge not at water level")
	var across := node.transform.basis.x.normalized()
	check(absf(across.dot(Vector3(1.0, 0.0, 0.0))) < 1e-3, "bridge not across the current")
	var span := node.mesh.get_aabb().size.x * node.transform.basis.x.length()
	var river := 100.0 / 719.0
	check(span > river and span < river * 1.3 + 0.1, "bridge span %.3f for a river of %.3f" % [span, river])
	var deck := node.transform.basis.y.length() * (0.05 + 0.02 * river / RiverCrossings.FINE_SCALE)
	check(absf(deck - 11.0 * MapData.vertical_scale()) < 1e-4, "deck not at z_deck: %.4f" % deck)
	crossings.set_fine_mode(false)
	check(Vector2(node.position.x, node.position.z).is_equal_approx(Vector2(2080.3, 1890.4)), "bridge did not return to the V4 crossing")
	# ZG4b : bascule étalée — budget de l'image épuisé, un seul ouvrage remis en forme par image.
	for k in 3:
		crossings._add({"id": "bridge_test", "name": "Pont", "structure": "stone", "px": Vector2(2080.3 + k, 1890.4), "dir": Vector2(0.0, 1.0), "width": 0.4, "type": "bridge", "index": 0})
	FrameBudget.begin_frame()
	FrameBudget._frame_start_usec -= 1000000
	crossings.set_fine_mode(true)
	check(crossings.pending_reshapes() == 3, "spread bridge switch: %d pending" % crossings.pending_reshapes())
	crossings.pump_reshape(true)
	check(crossings.pending_reshapes() == 0 and Vector2(node.position.x, node.position.z).is_equal_approx(Vector2(2080.0, 1890.0)), "bridge switch completed")
	FrameBudget._frame = -1
	crossings.queue_free()
	renderer.free()


func _test_real_cache() -> void:
	var map_dir := MAP_PATHS.default_data_dir().path_join("map")
	var store := FineGeoStore.new()
	if not store.load_from(map_dir) or not store.available(1):
		print("zg5b_fine_geo_test: no ZG5a cache, real-data checks skipped")
		return
	# Rouen ≈ (2097, 3099) en unités monde : tuile E2 (32, 48) (cache dans le cadre monde, ADR 0121).
	var tile := store.load_sync(1, 32, 48)
	if not check(tile != null and tile.lines() > 0, "Rouen tile missing"):
		return
	var seine := 0
	for i in tile.lines():
		if tile.line_rank[i] >= 7:
			seine += 1
	check(seine > 0, "no large river near Rouen")
	check(store.crossings.size() > 800 and store.hamlets.size() > 2000, "anchors: %d crossings, %d hamlets" % [store.crossings.size(), store.hamlets.size()])
	# Page E4 réelle sous la Seine, creusée (temps mesuré).
	var pyramid := ReliefPyramid.new()
	if not pyramid.load_manifest(map_dir):
		return
	var carver := FineBedCarver.new(store, 719.0, pyramid.height_min_m, pyramid.height_range_m)
	var best_key := -1
	for i in tile.lines():
		if tile.line_rank[i] >= 7:
			var k := tile.line_start[i] + tile.line_count[i] / 2
			var t := ReliefPyramid.tile_at(4, tile.x[k], tile.y[k])
			if pyramid.has_tile(4, t.x, t.y):
				best_key = ReliefPyramid.key_of(4, t.x, t.y)
				break
	if best_key < 0 or not ClassDB.class_exists("GameDataStore"):
		return
	var decoder: Object = ClassDB.instantiate("GameDataStore")
	var bytes: PackedByteArray = decoder.call("load_heightmap_u16", pyramid.tile_path(4, ReliefPyramid.col_of_key(best_key), ReliefPyramid.row_of_key(best_key)))
	var task: Object = carver.carve_job(best_key)
	if not check(task != null and bytes.size() == 512 * 512 * 2, "real E4 page under the Seine not carved"):
		return
	var t0 := Time.get_ticks_usec()
	task.call("apply", bytes.duplicate())
	var ms := (Time.get_ticks_usec() - t0) / 1000.0
	print("zg5b_fine_geo_test: real E4 page carved in %.1f ms (%d px)" % [ms, int(task.get("carved_px"))])
	check(int(task.get("carved_px")) > 100, "too few carved pixels")


static func _flat_page(h_m: float) -> PackedByteArray:
	var code := int(round((h_m + 200.0) / 5000.0 * 65535.0))
	var page := PackedByteArray()
	page.resize(512 * 512 * 2)
	for o in range(0, page.size(), 2):
		page.encode_u16(o, code)
	return page


static func _page_h(bytes: PackedByteArray, origin: Vector2, px_units: float, p: Vector2) -> float:
	var i := int((p.x - origin.x) / px_units)
	var j := int((p.y - origin.y) / px_units)
	return -200.0 + bytes.decode_u16((j * 512 + i) * 2) / 65535.0 * 5000.0


static func _write(path: String, bytes: PackedByteArray) -> void:
	var f := FileAccess.open(path, FileAccess.WRITE)
	f.store_buffer(bytes)
	f.close()


static func _write_json(path: String, value: Variant) -> void:
	var f := FileAccess.open(path, FileAccess.WRITE)
	f.store_string(JSON.stringify(value))
	f.close()
