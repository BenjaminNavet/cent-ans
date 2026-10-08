extends SceneTree

## FC3 : touffes d'herbe et broussailles proches (`GroundClutter`, 1:1 depuis VT3) : préréglages, portée,
## disque autour du point visé, budget d'instances, déterminisme.
## Usage : godot --headless --path game --script res://tests/fc3_clutter_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

var _failures := 0


func _init() -> void:
	await process_frame
	_test_presets()
	_test_low_preset()
	_test_in_range()
	_test_far_camera()
	_test_deterministic()
	_test_mesh()
	_test_threaded()
	RenderQuality.override_level = ""
	print("fc3_clutter_test: %s" % ("OK" if _failures == 0 else "%d échec(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _make(level: String) -> GroundClutter:
	RenderQuality.override_level = level
	var clutter := GroundClutter.new()
	root.add_child(clutter)
	clutter.max_cells_per_frame = 1000
	clutter.threaded = false
	clutter.weight_sampler = func(_x: float, _y: float) -> Vector2: return Vector2(1.0, 0.2)
	clutter.height_sampler = func(points: PackedVector2Array) -> PackedFloat32Array:
		var h := PackedFloat32Array()
		h.resize(points.size())
		return h
	return clutter


func _test_presets() -> void:
	var expected := {"low": 0.0, "medium": 0.5, "high": 1.0, "ultra": 1.5, "legacy": 0.0}
	for level: String in expected:
		_check(is_equal_approx(float(RenderQuality.presets()[level]["clutter_density"]), expected[level]), "%s clutter_density" % level)


func _test_low_preset() -> void:
	var clutter := _make("low")
	_check(is_zero_approx(clutter.quality_density), "low preset reaches the clutter node")
	clutter.update_view(Vector2(500, 500), 1.5)
	_check(clutter.visible_instances() == 0 and clutter.cell_count() == 0, "low: no tufts, nothing built")
	clutter.free()


func _test_in_range() -> void:
	var clutter := _make("high")
	var at := Vector2(500, 500)
	clutter.update_view(at, 1.5)
	var shown := clutter.visible_instances()
	_check(shown > 0, "high: tufts near the camera (%d)" % shown)
	_check(shown <= clutter.max_visible_instances, "high: within the instance budget (%d)" % shown)
	var r := clutter.radius()
	print("fc3: high d=30 full cover: %d tufts in %d cells, build max %.1f ms" % [shown, clutter.visible_cells().size(), float(clutter.stats["build_ms_max"])])
	var cells := clutter.visible_cells()
	var outside := 0
	var far := 0
	for cell: Dictionary in cells:
		var rect: Rect2 = cell["rect"]
		for p: Vector2 in cell["points"]:
			if not rect.grow(1e-3).has_point(p):
				outside += 1
			elif p.distance_to(at) > r + clutter.cell_size * 1.5:
				far += 1
	_check(outside == 0 and far == 0, "high: instances only within range (%d cells, r = %.0f, %d outside their cell, %d too far)" % [cells.size(), r, outside, far])
	# Ultra : plus de touffes, toujours sous le plafond.
	RenderQuality.override_level = "ultra"
	RenderQuality.apply_clients(self)
	clutter.update_view(at, 1.5)
	_check(clutter.visible_instances() > shown and clutter.visible_instances() <= clutter.max_visible_instances, "ultra: denser, still within budget (%d)" % clutter.visible_instances())
	RenderQuality.override_level = "low"
	RenderQuality.apply_clients(self)
	clutter.update_view(at, 1.5)
	_check(clutter.visible_instances() == 0, "switch to low hides every tuft")
	# Point visé déplacé loin : les anciennes cellules sont masquées.
	RenderQuality.override_level = "high"
	RenderQuality.apply_clients(self)
	clutter.update_view(Vector2(1500, 1500), 1.5)
	for cell: Dictionary in clutter.visible_cells():
		_check((cell["rect"] as Rect2).get_center().distance_to(Vector2(1500, 1500)) < clutter.radius() + clutter.cell_size, "moved: old cells hidden")
	clutter.free()


func _test_far_camera() -> void:
	var clutter := _make("high")
	clutter.update_view(Vector2(500, 500), 1.5)
	clutter.update_view(Vector2(500, 500), 4.0)
	_check(not clutter.visible and clutter.visible_instances() == 0, "beyond max_camera_distance: nothing shown")
	# VT3 : touffes 1:1, portée où une broussaille fait ≈ 1 px (1080p, fov 55°).
	var bush_px := MapPropScale.pixels_for(clutter.bush_height_m, clutter.max_camera_distance)
	_check(bush_px > 0.7 and bush_px < 1.4, "real-size bush at the range: %.2f px" % bush_px)
	_check(clutter.grass_height_m >= 0.3 and clutter.grass_height_m <= 1.0 and clutter.bush_height_m >= 1.0 and clutter.bush_height_m <= 3.0, "real clutter heights")
	clutter.free()


func _test_deterministic() -> void:
	var a := _make("high")
	a.update_view(Vector2(300, 300), 1.2)
	var b := _make("high")
	b.update_view(Vector2(300, 300), 1.2)
	var ca := a.visible_cells()
	var cb := b.visible_cells()
	var same := ca.size() == cb.size() and ca.size() > 0
	if same:
		var pa: Dictionary = {}
		for cell: Dictionary in ca:
			pa[cell["rect"]] = cell["points"]
		for cell: Dictionary in cb:
			same = same and pa.get(cell["rect"], PackedVector2Array()) == cell["points"]
	_check(same, "same cells → same tufts")
	a.free()
	b.free()


## Semis dans le `WorkerThreadPool`, pose sur le fil principal : même résultat qu'en direct.
func _test_threaded() -> void:
	var direct := _make("high")
	direct.update_view(Vector2(700, 700), 1.5)
	var pooled := _make("high")
	pooled.threaded = true
	pooled.max_cells_per_frame = 4
	var guard := 0
	pooled.update_view(Vector2(700, 700), 1.5)
	while (pooled.pending_jobs() > 0 or pooled.visible_cells().size() < direct.visible_cells().size()) and guard < 2000:
		guard += 1
		OS.delay_msec(1)
		pooled.update_view(Vector2(700, 700), 1.5)
	_check(pooled.visible_instances() == direct.visible_instances() and pooled.visible_instances() > 0, "threaded seeding matches direct seeding (%d / %d)" % [pooled.visible_instances(), direct.visible_instances()])
	print("fc3: threaded install max %.2f ms per cell (main thread)" % float(pooled.stats["build_ms_max"]))
	direct.free()
	pooled.free()


func _test_mesh() -> void:
	var mesh := GroundClutter.crossed_cards_mesh()
	var arrays := mesh.surface_get_arrays(0)
	_check((arrays[Mesh.ARRAY_INDEX] as PackedInt32Array).size() == 18, "3 crossed quads (6 triangles)")


func _check(condition: bool, label: String) -> void:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
