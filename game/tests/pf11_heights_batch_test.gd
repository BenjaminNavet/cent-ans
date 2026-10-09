extends TestCase

## Lot SC PF-11 : `MapData.heights_m_at` (lot) rend les mêmes altitudes que `height_m_at` (point
## par point), bords compris ; temps des deux chemins. `TileJobPool` (PF-12) : soumission, reprise.
## Usage : godot --headless --path game --script res://tests/pf11_heights_batch_test.gd


func _init() -> void:
	var map_data := MapData.load_from_dir(MAP_PATHS.default_data_dir().path_join("map"))
	if map_data.load_error != "":
		print("PF11: carte illisible (%s)" % map_data.load_error)
		failures += 1
		finish()
		return
	var rng := RandomNumberGenerator.new()
	rng.seed = 11
	var points := PackedVector2Array([Vector2(-5, -5), Vector2(0, 0), Vector2(map_data.size.x + 3.0, map_data.size.y + 3.0), Vector2(map_data.size)])
	for _n in 20000:
		points.append(Vector2(rng.randf() * map_data.size.x, rng.randf() * map_data.size.y))
	var t0 := Time.get_ticks_usec()
	var single := PackedFloat32Array()
	for p in points:
		single.append(map_data.height_m_at(p.x, p.y))
	var t1 := Time.get_ticks_usec()
	var batch := map_data.heights_m_at(points)
	var t2 := Time.get_ticks_usec()
	var worst := 0.0
	for n in points.size():
		worst = maxf(worst, absf(single[n] - batch[n]))
	check(batch.size() == points.size() and worst < 1e-3, "PF11: écart max %f m" % worst)
	print("PF11: point par point %d us, lot %d us (%d points)" % [t1 - t0, t2 - t1, points.size()])

	var pool := TileJobPool.new()
	var holder := {"result": 0}
	pool.submit(Vector2i(1, 2), holder, func() -> void: holder["result"] = 42, "test")
	check(pool.has(Vector2i(1, 2)) and pool.size() == 1, "PF12: soumission")
	check(pool.submit(Vector2i(1, 2), {}, func() -> void: pass) == null, "PF12: clé déjà soumise refusée")
	pool.wait_done()
	check(pool.is_done(Vector2i(1, 2)) and pool.ready_keys() == [Vector2i(1, 2)], "PF12: tâche attendue = prête")
	var taken: Dictionary = pool.take(Vector2i(1, 2))
	check(taken["result"] == 42 and pool.is_empty(), "PF12: reprise du résultat")
	pool.submit(0, null, func() -> void: pass)
	pool.wait_all()
	check(pool.is_empty(), "PF12: wait_all vide la file")
	finish()
