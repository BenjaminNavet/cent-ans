extends TestCase

## Lot SC PF-08/BT7/GB6 : le terrain de bataille calculé en Rust (`BattleTerrainKernel`) donne les
## mêmes hauteurs, splatmaps, carte de relief et maillages que l'ancien GDScript. Les valeurs
## attendues (`GOLDEN`) ont été relevées sur le GDScript avant le portage ; une valeur absente
## (`--dump`) imprime les résumés au lieu de les comparer.
## Usage : godot --headless --path game --script res://tests/sc_terrain_golden_test.gd [-- --dump]

func _scene(kind: String) -> Dictionary:
	var nx := 121
	var nz := 81
	var heights := PackedFloat32Array()
	heights.resize(nx * nz)
	for iz in nz:
		for ix in nx:
			heights[iz * nx + ix] = 6.0 * sin(ix * 0.11) * cos(iz * 0.17) + 0.02 * ix
	var pts := PackedVector2Array()
	var widths := PackedFloat32Array()
	for i in nx:
		pts.append(Vector2(i * 10.0, 380.0 + 40.0 * sin(i * 0.09)))
		widths.append(14.0 + 4.0 * sin(i * 0.2))
	var t := {
		"nx": nx, "nz": nz, "resolution": 10.0, "heights": heights, "width": 1200.0, "depth": 800.0,
		"terrain": "hills", "season": "summer", "ground": "dry", "woodland": 0.6,
		"forests": [{"x": 300.0, "z": 200.0, "radius": 60.0}, {"x": 900.0, "z": 600.0, "radius": 45.0}],
		"mud": [{"x": 500.0, "z": 150.0, "radius": 30.0}],
		"pools": [{"x": 700.0, "z": 120.0, "radius": 14.0}],
		"river": {"points": pts, "widths": widths, "width": 16.0, "flow": 1,
			"fords": [{"x": 600.0, "half_width": 25.0}],
			"banks": [{"x0": 100.0, "x1": 400.0, "north": true, "kind": "marsh"}, {"x0": 700.0, "x1": 900.0, "north": false, "kind": "steep"}]},
		"streams": [{"points": PackedVector2Array([Vector2(100, 100), Vector2(160, 140), Vector2(230, 130)]), "width": 4.0, "kind": "tributary"}],
		"roads": [{"points": PackedVector2Array([Vector2(0, 100), Vector2(300, 150), Vector2(600, 130)]), "width": 4.0}],
		"obstacles": [{"kind": "ditch", "a": Vector2(200, 600), "b": Vector2(320, 640)}],
	}
	if kind in ["coast", "horizon_coast"]:
		t["coast"] = {"flank": "west", "shore_x": -40.0, "beach": 30.0}
		t["terrain"] = "mountains"
	return t


func _digest(terrain: BattleTerrain) -> Dictionary:
	var d := {}
	var vals := PackedFloat64Array()
	for p in [Vector2(5, 5), Vector2(600, 400), Vector2(1190, 790), Vector2(-300, 200), Vector2(1700, 900), Vector2(300, -1200), Vector2(-3000, 4000), Vector2(6000, -5000), Vector2(612.5, 391.3), Vector2(-20, 400)]:
		vals.append(terrain.height_at(clampf(p.x, 0, 1200), clampf(p.y, 0, 800)))
		vals.append(terrain.world_height(p.x, p.y))
		vals.append(terrain.river_distance(p.x, p.y) if terrain.river_distance(p.x, p.y) < INF else -1.0)
		vals.append(terrain.river_span_at(p.x))
		vals.append(terrain.river_width_at(p.x))
		vals.append(1.0 if terrain.in_water(p.x, p.y) else 0.0)
	d["values"] = vals
	d["splat_a"] = hash(terrain.splat_a.get_image().get_data())
	d["splat_b"] = hash(terrain.splat_b.get_image().get_data())
	d["height_tex"] = hash(terrain.height_texture.get_image().get_data())
	d["relief_tex"] = hash(terrain.relief_texture.get_image().get_data())
	for mesh_name in ["Ground", "NearRing", "FarRing"]:
		var inst := terrain.get_node_or_null(mesh_name) as MeshInstance3D
		if inst == null:
			continue
		var arrays := inst.mesh.surface_get_arrays(0)
		d[mesh_name + "_v"] = hash((arrays[Mesh.ARRAY_VERTEX] as PackedVector3Array).to_byte_array())
		d[mesh_name + "_n"] = hash((arrays[Mesh.ARRAY_NORMAL] as PackedVector3Array).to_byte_array())
		d[mesh_name + "_i"] = hash((arrays[Mesh.ARRAY_INDEX] as PackedInt32Array).to_byte_array())
	return d


func _init() -> void:
	var dump := OS.get_cmdline_user_args().has("--dump")
	var started := Time.get_ticks_msec()
	for kind in ["inland", "coast", "horizon", "horizon_coast"]:
		var world := Node3D.new()
		root.add_child(world)
		var terrain := BattleTerrain.new()
		world.add_child(terrain)
		if kind.begins_with("horizon"):
			terrain.province_id = "prov_agenais"
		var t0 := Time.get_ticks_msec()
		terrain.build(_scene(kind), "clear")
		print("sc_terrain_golden %s: build %d ms" % [kind, Time.get_ticks_msec() - t0])
		var digest := _digest(terrain)
		if dump:
			print("GOLDEN_%s = %s" % [kind, var_to_str(digest)])
		else:
			var expected: Dictionary = str_to_var(GOLDEN_TEXT[kind])
			for key in expected:
				if key == "values":
					var a: PackedFloat64Array = expected[key]
					var b: PackedFloat64Array = digest[key]
					for i in a.size():
						check(absf(a[i] - b[i]) < 1e-3 or (is_inf(a[i]) and is_inf(b[i])), "%s values[%d]: %f vs %f" % [kind, i, a[i], b[i]])
				else:
					check(expected[key] == digest[key], "%s %s" % [kind, key])
		world.queue_free()
		await process_frame
	print("sc_terrain_golden: total %d ms" % (Time.get_ticks_msec() - started))
	finish()


const GOLDEN_TEXT := {
	"inland": """{
"FarRing_i": 807888147,
"FarRing_n": 4081291284,
"FarRing_v": 2321437119,
"Ground_i": 3726180062,
"Ground_n": 1012320593,
"Ground_v": 3234688819,
"NearRing_i": 3078129345,
"NearRing_n": 1277175276,
"NearRing_v": 3876858190,
"height_tex": 2032257030,
"relief_tex": 2821093605,
"splat_a": 1551478310,
"splat_b": 2856821121,
"values": PackedFloat64Array(0.33696118, 0.33696118, 375.03333, 21.596008, 14.397339, 0, 2.8251197, 2.8251197, 49.450066, 17.78056240081787, 11.853708, 0, 4.329151, 4.329151, 443.75662, 15.1692095, 10.112806, 0, 0, 4.42256344941125, 187.57469, 19.323506355285645, 12.882338, 0, 4.217798, 7.775449681946189, 506.8624, 26.943644, 17.96243, 0, -0.34647417, 29.03516976764562, 1583.7654, 19.323506355285645, 12.882338, 0, 0, 83.90142207588238, -1, 16.459185, 10.97279, 0, 5.952441, 72.83910865117952, -1, 26.47767162322998, 17.651781, 0, 3.6740974669206476, 3.6740974669206476, 37.799213, 19.139561533927917, 12.759707689285278, 0, 0, 0.002518279921537331, 12.114975, 23.336510181427002, 15.557673, 0)
}""",
	"coast": """{
"FarRing_i": 807888147,
"FarRing_n": 644713562,
"FarRing_v": 981978977,
"Ground_i": 3726180062,
"Ground_n": 1012320593,
"Ground_v": 3234688819,
"NearRing_i": 3078129345,
"NearRing_n": 3242702313,
"NearRing_v": 767552055,
"height_tex": 3117878218,
"relief_tex": 1625844275,
"splat_a": 2921775868,
"splat_b": 2961074298,
"values": PackedFloat64Array(0.33696118, 0.33696118, 375.03333, 21.596008, 14.397339, 0, 2.8251197, 2.8251197, 49.450066, 17.78056240081787, 11.853708, 0, 4.329151, 4.329151, 443.75662, 15.1692095, 10.112806, 0, 0, -4, 187.57469, 19.323506355285645, 12.882338, 0, 4.217798, 17.969188619145132, 506.8624, 26.943644, 17.96243, 0, -0.34647417, 84.64296574549809, 1583.7654, 19.323506355285645, 12.882338, 0, 0, -4, -1, 16.459185, 10.97279, 0, 5.952441, 195.80227587318603, -1, 26.47767162322998, 17.651781, 0, 3.6740974669206476, 3.6740974669206476, 37.799213, 19.139561533927917, 12.759707689285278, 0, 0, -0.2215743440233236, 12.114975, 23.336510181427002, 15.557673, 0)
}""",
	"horizon": """{
"FarRing_i": 807888147,
"FarRing_n": 3813276901,
"FarRing_v": 1295227695,
"Ground_i": 3726180062,
"Ground_n": 1012320593,
"Ground_v": 3234688819,
"NearRing_i": 3078129345,
"NearRing_n": 49916584,
"NearRing_v": 3316694961,
"height_tex": 2032257030,
"relief_tex": 2821093605,
"splat_a": 1551478310,
"splat_b": 2856821121,
"values": PackedFloat64Array(0.33696118, 0.33696118, 375.03333, 21.596008, 14.397339, 0, 2.8251197, 2.8251197, 49.450066, 17.78056240081787, 11.853708, 0, 4.329151, 4.329151, 443.75662, 15.1692095, 10.112806, 0, 0, 4.42256344941125, 187.57469, 19.323506355285645, 12.882338, 0, 4.217798, 7.775449681946189, 506.8624, 26.943644, 17.96243, 0, -0.34647417, 22.938822450216396, 1583.7654, 19.323506355285645, 12.882338, 0, 0, 69.74753735329892, -1, 16.459185, 10.97279, 0, 5.952441, -22.502462646701076, -1, 26.47767162322998, 17.651781, 0, 3.6740974669206476, 3.6740974669206476, 37.799213, 19.139561533927917, 12.759707689285278, 0, 0, 0.002518279921537331, 12.114975, 23.336510181427002, 15.557673, 0)
}""",
	"horizon_coast": """{
"FarRing_i": 807888147,
"FarRing_n": 2086503080,
"FarRing_v": 2298680585,
"Ground_i": 3726180062,
"Ground_n": 1012320593,
"Ground_v": 3234688819,
"NearRing_i": 3078129345,
"NearRing_n": 2310576327,
"NearRing_v": 3461445764,
"height_tex": 3117878218,
"relief_tex": 1625844275,
"splat_a": 2921775868,
"splat_b": 2961074298,
"values": PackedFloat64Array(0.33696118, 0.33696118, 375.03333, 21.596008, 14.397339, 0, 2.8251197, 2.8251197, 49.450066, 17.78056240081787, 11.853708, 0, 4.329151, 4.329151, 443.75662, 15.1692095, 10.112806, 0, 0, -4, 187.57469, 19.323506355285645, 12.882338, 0, 4.217798, 17.969188619145132, 506.8624, 26.943644, 17.96243, 0, -0.34647417, 79.80675924639316, 1583.7654, 19.323506355285645, 12.882338, 0, 0, -4, -1, 16.459185, 10.97279, 0, 5.952441, 43.5, -1, 26.47767162322998, 17.651781, 0, 3.6740974669206476, 3.6740974669206476, 37.799213, 19.139561533927917, 12.759707689285278, 0, 0, -0.2215743440233236, 12.114975, 23.336510181427002, 15.557673, 0)
}""",
}
