extends TestCase

## VT-B (ADR 0138) : maillage lointain des villes (`TownFarBuilder`) : déterminisme, budget de
## triangles F1/F2 sur tout `towns_1340.json`, sommets dans l'emprise, contrat de sommets (UV2,
## jupe), repli sans `ground_m`, villes v2, fusion par tuile, temps de génération.
## Usage : godot --headless --path game --script res://tests/tf_far_mesh_test.gd
## Code de sortie 0 si tout passe, 1 sinon.

const F1_MEAN_MAX := 400.0
const F1_MAX := 1500
const F2_MAX := 60
const V2_MAX := 6000

var _mpu := 719.0


func _init() -> void:
	await process_frame
	var towns_file: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(MAP_PATHS.default_data_dir().path_join("map/towns_1340.json")))
	var towns: Dictionary = towns_file["towns"]
	var mpu := float(towns_file.get("meters_per_unit", 719.0))
	_mpu = mpu
	var wall_params: Dictionary = towns_file.get("walls", {})
	_test_all(towns, mpu, wall_params)
	_test_deterministic(towns, mpu, wall_params)
	_test_no_ground(towns, mpu, wall_params)
	_test_append(towns, mpu, wall_params)
	_test_v2(mpu)
	finish()


## Budget, emprise et contrat sur toutes les villes ; temps F1 + F2 mesuré.
func _test_all(towns: Dictionary, mpu: float, wall_params: Dictionary) -> void:
	var ids := towns.keys()
	ids.sort()
	var f1_total := 0
	var f1_max := 0
	var f1_max_id := ""
	var f2_total := 0
	var f2_max := 0
	var usec_f1 := 0
	var usec_f2 := 0
	var bad_extent := 0
	var bad_contract := 0
	for i in ids.size():
		var town: Dictionary = towns[ids[i]]
		var t0 := Time.get_ticks_usec()
		var f1 := TownFarBuilder.build_f1(town, i, mpu, wall_params)
		var t1 := Time.get_ticks_usec()
		var f2 := TownFarBuilder.build_f2(town, i, mpu)
		usec_f2 += Time.get_ticks_usec() - t1
		usec_f1 += t1 - t0
		var n1 := TownFarBuilder.triangle_count(f1)
		var n2 := TownFarBuilder.triangle_count(f2)
		f1_total += n1
		f2_total += n2
		if n1 > f1_max:
			f1_max = n1
			f1_max_id = str(ids[i])
		f2_max = maxi(f2_max, n2)
		if not _in_extent(town, f1, mpu) or not _in_extent(town, f2, mpu):
			bad_extent += 1
		if not _contract_ok(town, f1, i) or not _contract_ok(town, f2, i):
			bad_contract += 1
	var mean1 := float(f1_total) / ids.size()
	var mean2 := float(f2_total) / ids.size()
	print("tf_far_mesh_test: %d villes ; F1 %.0f tri/ville (max %d, %s) ; F2 %.1f tri/ville (max %d)" % [ids.size(), mean1, f1_max, f1_max_id, mean2, f2_max])
	print("tf_far_mesh_test: génération F1 %.0f ms, F2 %.0f ms, total %.0f ms (un fil)" % [usec_f1 / 1000.0, usec_f2 / 1000.0, (usec_f1 + usec_f2) / 1000.0])
	check(mean1 <= F1_MEAN_MAX, "F1 moyenne %.0f > %.0f" % [mean1, F1_MEAN_MAX])
	check(f1_max <= F1_MAX, "F1 max %d > %d (%s)" % [f1_max, F1_MAX, f1_max_id])
	check(f2_max <= F2_MAX, "F2 max %d > %d" % [f2_max, F2_MAX])
	check(bad_extent == 0, "%d villes débordent de l'emprise (rayon max × 1,2)" % bad_extent)
	check(bad_contract == 0, "%d villes hors contrat de sommets" % bad_contract)


## Rayon d'emprise (m) : rayon max, faubourgs compris, × 1,2.
func _extent_m(town: Dictionary) -> float:
	var r := 60.0
	for v in town.get("radii", []):
		r = maxf(r, float(v))
	for f: Dictionary in town.get("faubourgs", []):
		r = maxf(r, float(f["start_m"]) + float(f["length_m"]))
	for m: Dictionary in town.get("monuments", []):
		var at: Array = m["at"]
		r = maxf(r, Vector2(float(at[0]), float(at[1])).length() + float(m.get("size_m", 20.0)))
	return r * 1.2


func _in_extent(town: Dictionary, part: Dictionary, mpu: float) -> bool:
	var px: Array = town["px"]
	var anchor := Vector2(float(px[0]), float(px[1]))
	var limit := _extent_m(town) / mpu
	for v: Vector3 in part["vertices"]:
		if Vector2(v.x, v.z).distance_to(anchor) > limit:
			return false
	return true


## UV2.x = index, UV2.y ≥ −30 (jupe) et ≥ −30 au plus bas ; y absolu = sol + UV2.y dans une
## plage plausible ; normales unitaires ; indices valides ; faces avant vers l'extérieur.
func _contract_ok(town: Dictionary, part: Dictionary, index: int) -> bool:
	var verts: PackedVector3Array = part["vertices"]
	var uv2: PackedVector2Array = part["uv2"]
	var normals: PackedVector3Array = part["normals"]
	if verts.size() != uv2.size() or verts.size() != normals.size() or verts.size() != (part["colors"] as PackedColorArray).size():
		return false
	var lo := INF
	var hi := -INF
	var ground: Array = town.get("ground_m", [town["z_m"]])
	for g in ground:
		lo = minf(lo, float(g))
		hi = maxf(hi, float(g))
	var min_rel := INF
	for k in verts.size():
		if not is_equal_approx(uv2[k].x, float(index)):
			return false
		min_rel = minf(min_rel, uv2[k].y)
		if uv2[k].y < -TownFarBuilder.SKIRT_M - 0.01 or uv2[k].y > 200.0:
			return false
		var ground_y := verts[k].y - uv2[k].y
		if ground_y < lo - 1.0 or ground_y > hi + 1.0:
			return false
		if absf(normals[k].length() - 1.0) > 0.01:
			return false
	if not is_equal_approx(min_rel, -TownFarBuilder.SKIRT_M):
		return false
	var idx: PackedInt32Array = part["indices"]
	for i in idx:
		if i < 0 or i >= verts.size():
			return false
	# Faces avant (sens horaire vu de l'extérieur, convention Godot) : la normale géométrique
	# (c − a) × (b − a), en mètres, suit la normale des sommets.
	var mpu := _mpu
	for t in range(0, idx.size(), 3):
		var a := _meters(verts[idx[t]], mpu)
		var b := _meters(verts[idx[t + 1]], mpu)
		var c := _meters(verts[idx[t + 2]], mpu)
		var n := (c - a).cross(b - a)
		if n.dot(normals[idx[t]] + normals[idx[t + 1]] + normals[idx[t + 2]]) < 0.0:
			return false
	return true


func _meters(v: Vector3, mpu: float) -> Vector3:
	return Vector3(v.x * mpu, v.y, v.z * mpu)


func _test_deterministic(towns: Dictionary, mpu: float, wall_params: Dictionary) -> void:
	for id in ["set_paris", "set_rouen", "set_a_coruna", "set_aalborg"]:
		if not towns.has(id):
			continue
		var a := TownFarBuilder.build_f1(towns[id], 3, mpu, wall_params)
		var b := TownFarBuilder.build_f1(towns[id], 3, mpu, wall_params)
		check(a["vertices"] == b["vertices"] and a["indices"] == b["indices"] and a["colors"] == b["colors"], "F1 non déterministe (%s)" % id)
		var c := TownFarBuilder.build_f2(towns[id], 3, mpu)
		var d := TownFarBuilder.build_f2(towns[id], 3, mpu)
		check(c["vertices"] == d["vertices"] and c["indices"] == d["indices"], "F2 non déterministe (%s)" % id)


## Sans `ground_m` : sol constant `z_m` (pied de jupe = z_m − 30 partout).
func _test_no_ground(towns: Dictionary, mpu: float, wall_params: Dictionary) -> void:
	var with_ground := 0
	for id in towns:
		if (towns[id] as Dictionary).has("ground_m"):
			with_ground += 1
	print("tf_far_mesh_test: %d villes avec ground_m" % with_ground)
	var town: Dictionary = (towns[towns.keys()[0]] as Dictionary).duplicate(true)
	town.erase("ground_m")
	var z := float(town["z_m"])
	for part in [TownFarBuilder.build_f1(town, 0, mpu, wall_params), TownFarBuilder.build_f2(town, 0, mpu)]:
		var verts: PackedVector3Array = part["vertices"]
		var uv2: PackedVector2Array = part["uv2"]
		check(verts.size() > 0, "sans ground_m : maillage vide")
		var ok := true
		for k in verts.size():
			ok = ok and absf(verts[k].y - uv2[k].y - z) < 0.01
		check(ok, "sans ground_m : sol différent de z_m")
		check(is_equal_approx(float(part["y_min"]), z - TownFarBuilder.SKIRT_M), "sans ground_m : pied de jupe %.2f ≠ z_m − 30" % float(part["y_min"]))


func _test_append(towns: Dictionary, mpu: float, wall_params: Dictionary) -> void:
	var ids := towns.keys()
	ids.sort()
	var tile := {}
	var tris := 0
	var verts := 0
	for i in 5:
		var part := TownFarBuilder.build_f1(towns[ids[i]], i, mpu, wall_params)
		tris += TownFarBuilder.triangle_count(part)
		verts += (part["vertices"] as PackedVector3Array).size()
		TownFarBuilder.append(tile, part)
	TownFarBuilder.append(tile, {})
	check(TownFarBuilder.triangle_count(tile) == tris, "append : triangles %d ≠ %d" % [TownFarBuilder.triangle_count(tile), tris])
	check((tile["vertices"] as PackedVector3Array).size() == verts, "append : sommets")
	var max_index := 0
	for i in (tile["indices"] as PackedInt32Array):
		max_index = maxi(max_index, i)
	check(max_index == verts - 1, "append : indices non décalés")
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, TownFarBuilder.mesh_arrays(tile))
	check(mesh.get_surface_count() == 1 and mesh.surface_get_array_index_len(0) == tris * 3, "append : ArrayMesh")
	check(TownFarBuilder.mesh_arrays({}).is_empty(), "mesh_arrays({}) non vide")


func _test_v2(mpu: float) -> void:
	var cities := LandmarkV2Library.all()
	check(cities.size() >= 8, "villes v2 : %d < 8" % cities.size())
	var t0 := Time.get_ticks_usec()
	var total := 0
	var worst := 0
	for i in cities.size():
		var city: Dictionary = cities[i]
		var part := TownFarBuilder.build_v2_far(city, 5000 + i, mpu)
		var n := TownFarBuilder.triangle_count(part)
		total += n
		worst = maxi(worst, n)
		check(n > 100, "v2 %s : %d triangles" % [city["id"], n])
		var anchor := LandmarkV2Library.anchor_units(city)
		var limit := float(city.get("extent_m", 1500.0)) * 1.5 / mpu
		var inside := true
		for v: Vector3 in part["vertices"]:
			inside = inside and Vector2(v.x, v.z).distance_to(anchor) <= limit
		check(inside, "v2 %s : sommets hors emprise" % city["id"])
		if str(city["id"]) == "london":
			check(float(part["y_max"]) >= 140.0, "Londres : flèche de Saint-Paul %.0f m < 140" % float(part["y_max"]))
	print("tf_far_mesh_test: v2 %d villes, %.0f tri/ville (max %d), %.0f ms" % [cities.size(), float(total) / maxi(cities.size(), 1), worst, (Time.get_ticks_usec() - t0) / 1000.0])
	check(worst <= V2_MAX, "v2 max %d > %d" % [worst, V2_MAX])
	# Relief sous la ville (Paris, 30/09) : chaque sommet des quartiers (nappe et jupe) posé sur le
	# sol échantillonné à sa position (monuments et murs gardent une base plane, hors test).
	var paris: Dictionary = {}
	for city: Dictionary in cities:
		if str(city["id"]) == "paris":
			paris = city
	if not paris.is_empty():
		var h := TownPlan.Heights.new()
		h.func_m = func(lx: float, ly: float) -> float: return 30.0 + 0.05 * lx + 40.0 * sin(ly / 300.0)
		var pa := LandmarkV2Library.anchor_units(paris)
		var districts_only := paris.duplicate()
		districts_only["walls"] = []
		districts_only["monuments"] = []
		var part := TownFarBuilder.build_v2_far(districts_only, 5000, mpu, pa, NAN, TownFarBuilder.V2_YEAR, h)
		var verts: PackedVector3Array = part["vertices"]
		var uv2: PackedVector2Array = part["uv2"]
		var worst_err := 0.0
		for k in verts.size():
			var local := (Vector2(verts[k].x, verts[k].z) - pa) * mpu
			var err := absf(verts[k].y - uv2[k].y - h.height_m(local.x, local.y))
			worst_err = maxf(worst_err, err)
		check(worst_err < 0.5, "v2 Paris : sommet hors du relief de %.1f m" % worst_err)
		print("tf_far_mesh_test: v2 Paris sur relief, %d triangles, écart max %.2f m" % [TownFarBuilder.triangle_count(part), worst_err])
