extends SceneTree

## Test headless du lot RS-K : `ReliefQuadtree.finest_levels` (étages parcourus du plus fin au
## plus grossier, tuiles candidates ou pages de l'étage) rend exactement le même résultat que le
## parcours complet des pages (`finest_levels_scan`), sur des pages synthétiques réparties comme
## en jeu (anneaux de plus en plus fins autour d'un point) et des rectangles de toutes tailles
## (morceaux E0, tuiles du réseau fin, pages, bords exacts de tuiles) ; affiche le gain.
## Usage : godot --headless --path game --script res://tests/rs_k_finest_levels_test.gd

var _failures := 0


func _init() -> void:
	_run()
	print("rs_k_finest_levels_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("rs_k_finest_levels_test: " + message)
	return condition


func _run() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 1337
	for trial in 4:
		var qt := ReliefQuadtree.new()
		var focus := Vector2(rng.randf_range(300.0, 3800.0), rng.randf_range(300.0, 3800.0))
		_fill_pages(qt, focus, rng)
		var rects: Array[Rect2] = []
		for i in 200:
			var size: float = [256.0, 64.0, 32.0, 2.0, 0.5, 700.0][i % 6]
			var p := focus + Vector2(rng.randf_range(-600.0, 600.0), rng.randf_range(-600.0, 600.0))
			if i % 7 == 0:  # bords exacts de tuiles (test strict de `Rect2.intersects`)
				p = (p / 64.0).floor() * 64.0
			rects.append(Rect2(p, Vector2(size, size)))
		rects.append(Rect2(-100.0, -100.0, 50.0, 50.0))  # hors carte
		rects.append(Rect2(0.0, 0.0, 4096.0, 4096.0))  # carte entière
		var t0 := Time.get_ticks_usec()
		var fast := qt.finest_levels(rects)
		var t1 := Time.get_ticks_usec()
		var slow := qt.finest_levels_scan(rects)
		var t2 := Time.get_ticks_usec()
		var mismatches := 0
		for i in rects.size():
			if fast[i] != slow[i]:
				mismatches += 1
				if mismatches <= 3:
					push_error("rect %s: %d != %d" % [rects[i], fast[i], slow[i]])
		_check(mismatches == 0, "trial %d: %d mismatches over %d rects" % [trial, mismatches, rects.size()])
		print("rs_k_finest_levels_test: trial %d, %d pages, %d rects: levels %.2f ms, scan %.2f ms" % [trial, qt._page_bytes.size(), rects.size(), (t1 - t0) / 1000.0, (t2 - t1) / 1000.0])
		# Éviction : la page retirée ne compte plus.
		var some_key: int = qt._page_bytes.keys()[0]
		var some_rect := ReliefQuadtree._tile_rect(some_key)
		qt._page_bytes.erase(some_key)
		qt._note_level_page(some_key, false)
		var probe: Array[Rect2] = [some_rect.grow(-0.01)]
		_check(qt.finest_levels(probe)[0] == qt.finest_levels_scan(probe)[0], "trial %d: same result after eviction" % trial)
		qt.free()


## ~256 pages : à chaque étage, un carré de tuiles autour de `focus` de plus en plus petit, plus
## quelques pages isolées ailleurs (LRU d'un panoramique).
func _fill_pages(qt: ReliefQuadtree, focus: Vector2, rng: RandomNumberGenerator) -> void:
	for level in 8:
		var t := ReliefPyramid.tile_units(level)
		var half := 2 if level < 6 else 3
		var c := int(focus.x / t)
		var r := int(focus.y / t)
		for row in range(r - half, r + half + 1):
			for col in range(c - half, c + half + 1):
				if col < 0 or row < 0 or col >= ReliefPyramid.tiles_per_side(level) or row >= ReliefPyramid.tiles_per_side(level):
					continue
				if rng.randf() < 0.8:
					_add(qt, ReliefPyramid.key_of(level, col, row))
		for extra in 6:
			var n := ReliefPyramid.tiles_per_side(level)
			_add(qt, ReliefPyramid.key_of(level, rng.randi_range(0, n - 1), rng.randi_range(0, n - 1)))


func _add(qt: ReliefQuadtree, key: int) -> void:
	if qt._page_bytes.has(key):
		return
	qt._page_bytes[key] = PackedByteArray()
	qt._note_level_page(key, true)
