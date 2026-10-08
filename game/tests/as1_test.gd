extends SceneTree

## Lot AS1 : bêtes animées. Vérifie par valeurs (le shader de sommets ne tourne pas sans GPU) :
## les données (`data/fx/animal_motion.json`) et leur fusion `defaults` < `base` < modèle, les
## uniformes posés sur les surfaces des accessoires de la carte, et que les masques du shader
## (pattes, tête, queue, roues) touchent bien de la géométrie des modèles FK2 et des chevaux du
## kit ; chevaux de camp : une surface animée par matière d'origine.
## Usage : godot --headless --path game --script res://tests/as1_test.gd [-- --no-as1]

const BEASTS := ["ox", "cow", "horse", "sheep"]
const CARTS := ["stone_cart", "merchant_cart"]

var ok := true


func _check(cond: bool, what: String) -> void:
	if not cond:
		ok = false
		print("AS1 FAIL: ", what)


func _vertices(mesh: Mesh) -> PackedVector3Array:
	var out := PackedVector3Array()
	for s in mesh.get_surface_count():
		out.append_array(mesh.surface_get_arrays(s)[Mesh.ARRAY_VERTEX])
	return out


## Nombre de sommets de `verts` satisfaisant chaque masque de `animal_motion.gdshaderinc`.
func _mask_counts(verts: PackedVector3Array, p: Dictionary) -> Dictionary:
	var anchor := float(p["anchor_m"])
	var half := float(p["half_m"])
	var belly := float(p.get("belly_m", 0.0))
	var counts := {"legs": 0, "head": 0, "tail": 0, "wheel": 0}
	for v in verts:
		var zr := v.z - anchor
		if half > 0.0:
			if absf(zr) < half * 0.95 and v.y < belly * 0.5:
				counts["legs"] += 1
			if zr > float(p.get("neck_start_m", 0.0)) + 0.3 and v.y > belly * 0.75:
				counts["head"] += 1
			if -zr - half > 0.02 and absf(v.x) <= 0.1 and v.y >= belly * 0.6 and -zr - half < 0.3:
				counts["tail"] += 1
		var radius := float(p.get("wheel_radius_m", 0.0))
		if radius > 0.0 and absf(v.x) > float(p["wheel_x_min_m"]) and Vector2(v.y - radius, v.z).length() < radius + 0.03:
			counts["wheel"] += 1
	return counts


func _init() -> void:
	var off := OS.get_cmdline_user_args().has("--no-as1")
	_check(AnimalMotion.enabled() == not off, "enabled() ne suit pas --no-as1")
	var settings := AnimalMotion.settings()
	_check(settings.has("campaign") and settings.has("camp_horse"), "données absentes")
	# Fusion des réglages.
	var ox := AnimalMotion.model_params("ox")
	var cart := AnimalMotion.model_params("stone_cart")
	_check(is_equal_approx(float(cart["anchor_m"]), 3.0), "stone_cart : ancre")
	_check(is_equal_approx(float(cart["half_m"]), float(ox["half_m"])), "stone_cart : hérite du bœuf")
	_check(float(AnimalMotion.model_params("dead_cart")["half_m"]) == 0.0, "dead_cart sans bête")
	_check(AnimalMotion.model_params("plough").is_empty(), "plough non décrit")
	# Uniformes et masques sur les accessoires.
	for role in BEASTS + CARTS + ["dead_cart", "plough"]:
		var mesh := FolkModels.prop_mesh(role)
		_check(mesh != null and FolkModels.prop_source(role).begins_with("res://"), "%s : glb absent" % role)
		if mesh == null:
			continue
		var mat := mesh.surface_get_material(0) as ShaderMaterial
		var on := float(mat.get_shader_parameter("am_on"))
		var described: bool = not AnimalMotion.model_params(str(FolkModels.PROPS[role]["model"])).is_empty()
		var beast: bool = role in BEASTS or role in CARTS
		if off or not beast:
			_check(on == 0.0, "%s : am_on devrait être 0" % role)
		else:
			_check(on == 1.0, "%s : am_on devrait être 1" % role)
		if off or not described:
			continue
		var p := AnimalMotion.model_params(str(FolkModels.PROPS[role]["model"]))
		var counts := _mask_counts(_vertices(mesh), p)
		if beast:
			for k in ["legs", "head", "tail"]:
				_check(int(counts[k]) >= 6, "%s : masque %s vide (%d)" % [role, k, counts[k]])
		if role in CARTS or role == "dead_cart":
			_check(int(counts["wheel"]) >= 20, "%s : masque des roues vide (%d)" % [role, counts["wheel"]])
			var am_cart: Vector4 = mat.get_shader_parameter("am_cart")
			_check(is_equal_approx(am_cart.x, float(p["wheel_radius_m"])), "%s : rayon de roue" % role)
		if role in CARTS:
			var am_cart2: Vector4 = mat.get_shader_parameter("am_cart")
			_check(is_equal_approx(am_cart2.w, float(p["stride_m"])), "%s : cahot calé sur la foulée" % role)
		elif role in BEASTS:
			_check(int(counts["wheel"]) == 0, "%s : roues inattendues" % role)
	# Chevaux de camp.
	for model_name in BuildingKit.models_of("horse"):
		var mesh := AnimalMotion.camp_horse_mesh(model_name)
		_check(mesh != null, "%s : maillage" % model_name)
		if mesh == null:
			continue
		var source := BuildingKit.source_mesh(model_name)
		_check(mesh.get_surface_count() == source.get_surface_count(), "%s : surfaces" % model_name)
		var hair := 0
		for s in mesh.get_surface_count():
			var mat := mesh.surface_get_material(s) as ShaderMaterial
			_check(mat != null and mat.shader == AnimalMotion.CAMP_SHADER, "%s : surface %d sans shader" % [model_name, s])
			if mat != null and float(mat.get_shader_parameter("ch_hair")) > 0.5:
				hair += 1
				# La queue pend vers +X : des crins sont dans la zone du masque.
				var tail: Vector4 = mat.get_shader_parameter("ch_tail")
				var n := 0
				for v in mesh.surface_get_arrays(s)[Mesh.ARRAY_VERTEX]:
					if v.x > tail.y and v.y < tail.z - 0.2:
						n += 1
				_check(n >= 10, "%s : queue absente du masque (%d)" % [model_name, n])
		_check(hair == 1, "%s : une surface de crins attendue (%d)" % [model_name, hair])
	# Pose en MultiMesh.
	var parent := Node3D.new()
	root.add_child(parent)
	var made := AnimalMotion.build_camp_horses(parent, {"horse_0": [Transform3D.IDENTITY, Transform3D(Basis(), Vector3(5, 0, 0))]}, 100.0)
	_check(made.size() == 1 and made[0].multimesh.instance_count == 2, "build_camp_horses")
	_check(is_equal_approx(made[0].visibility_range_end, 100.0), "portée des chevaux")
	print("AS1 ", "OK" if ok else "FAILED")
	quit(0 if ok else 1)
