extends TestCase

## Lot AS1 : bêtes animées. Vérifie par valeurs (le shader de sommets ne tourne pas sans GPU) :
## les données (`data/fx/animal_motion.json`) et leur fusion `defaults` < `base` < modèle, les
## uniformes posés sur les surfaces des accessoires de la carte, et que les masques du shader
## (pattes, tête, queue, roues) touchent bien de la géométrie des modèles FK2 et des chevaux du
## kit ; chevaux de camp : une surface animée par matière d'origine.
## Usage : godot --headless --path game --script res://tests/as1_test.gd

const BEASTS := ["ox", "cow", "horse", "sheep"]
const CARTS := ["stone_cart", "merchant_cart"]


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
	check(AnimalMotion.enabled(), "enabled() devrait suivre les données")
	var settings := AnimalMotion.settings()
	check(settings.has("campaign") and settings.has("camp_horse"), "données absentes")
	# Fusion des réglages.
	var ox := AnimalMotion.model_params("ox")
	var cart := AnimalMotion.model_params("stone_cart")
	check(is_equal_approx(float(cart["anchor_m"]), 3.0), "stone_cart : ancre")
	check(is_equal_approx(float(cart["half_m"]), float(ox["half_m"])), "stone_cart : hérite du bœuf")
	check(float(AnimalMotion.model_params("dead_cart")["half_m"]) == 0.0, "dead_cart sans bête")
	check(AnimalMotion.model_params("plough").is_empty(), "plough non décrit")
	# Uniformes et masques sur les accessoires.
	for role in BEASTS + CARTS + ["dead_cart", "plough"]:
		var mesh := FolkModels.prop_mesh(role)
		check(mesh != null and FolkModels.prop_source(role).begins_with("res://"), "%s : glb absent" % role)
		if mesh == null:
			continue
		var mat := mesh.surface_get_material(0) as ShaderMaterial
		var on := float(mat.get_shader_parameter("am_on"))
		var described: bool = not AnimalMotion.model_params(str(FolkModels.PROPS[role]["model"])).is_empty()
		var beast: bool = role in BEASTS or role in CARTS
		if not beast:
			check(on == 0.0, "%s : am_on devrait être 0" % role)
		else:
			check(on == 1.0, "%s : am_on devrait être 1" % role)
		if not described:
			continue
		var p := AnimalMotion.model_params(str(FolkModels.PROPS[role]["model"]))
		var counts := _mask_counts(_vertices(mesh), p)
		if beast:
			for k in ["legs", "head", "tail"]:
				check(int(counts[k]) >= 6, "%s : masque %s vide (%d)" % [role, k, counts[k]])
		if role in CARTS or role == "dead_cart":
			check(int(counts["wheel"]) >= 20, "%s : masque des roues vide (%d)" % [role, counts["wheel"]])
			var am_cart: Vector4 = mat.get_shader_parameter("am_cart")
			check(is_equal_approx(am_cart.x, float(p["wheel_radius_m"])), "%s : rayon de roue" % role)
		if role in CARTS:
			var am_cart2: Vector4 = mat.get_shader_parameter("am_cart")
			check(is_equal_approx(am_cart2.w, float(p["stride_m"])), "%s : cahot calé sur la foulée" % role)
		elif role in BEASTS:
			check(int(counts["wheel"]) == 0, "%s : roues inattendues" % role)
	# Chevaux de camp.
	for model_name in BuildingKit.models_of("horse"):
		var mesh := AnimalMotion.camp_horse_mesh(model_name)
		check(mesh != null, "%s : maillage" % model_name)
		if mesh == null:
			continue
		var source := BuildingKit.source_mesh(model_name)
		check(mesh.get_surface_count() == source.get_surface_count(), "%s : surfaces" % model_name)
		var hair := 0
		for s in mesh.get_surface_count():
			var mat := mesh.surface_get_material(s) as ShaderMaterial
			check(mat != null and mat.shader == AnimalMotion.CAMP_SHADER, "%s : surface %d sans shader" % [model_name, s])
			if mat != null and float(mat.get_shader_parameter("ch_hair")) > 0.5:
				hair += 1
				# La queue pend vers +X : des crins sont dans la zone du masque.
				var tail: Vector4 = mat.get_shader_parameter("ch_tail")
				var n := 0
				for v in mesh.surface_get_arrays(s)[Mesh.ARRAY_VERTEX]:
					if v.x > tail.y and v.y < tail.z - 0.2:
						n += 1
				check(n >= 10, "%s : queue absente du masque (%d)" % [model_name, n])
		check(hair == 1, "%s : une surface de crins attendue (%d)" % [model_name, hair])
	# Pose en MultiMesh.
	var parent := Node3D.new()
	root.add_child(parent)
	var made := AnimalMotion.build_camp_horses(parent, {"horse_0": [Transform3D.IDENTITY, Transform3D(Basis(), Vector3(5, 0, 0))]}, 100.0)
	check(made.size() == 1 and made[0].multimesh.instance_count == 2, "build_camp_horses")
	check(is_equal_approx(made[0].visibility_range_end, 100.0), "portée des chevaux")
	# Décor de bataille : les chevaux d'une ligne sont des MultiMesh animés.
	var terrain := BattleTerrain.new()
	root.add_child(terrain)
	var decor := BattleDecor.new()
	root.add_child(decor)
	var camp := {"side": "a", "area": {"x": 0.0, "z": 0.0}, "items": [{"kind": "horse_line", "x": 0.0, "z": 0.0, "yaw": 0.0, "length": 12.0, "count": 6}]}
	decor.build(terrain, {"profile": "as1", "camps": [camp]}, "clear")
	check(decor.horse_count == 6, "décor : %d chevaux posés au lieu de 6" % decor.horse_count)
	var animated := 0
	var plain := 0
	for node in decor.find_children("Kit_horse_*", "MultiMeshInstance3D", true, false):
		var mmi := node as MultiMeshInstance3D
		var shaded := mmi.multimesh.mesh.surface_get_material(0) is ShaderMaterial
		if shaded:
			animated += mmi.multimesh.instance_count
		else:
			plain += mmi.multimesh.instance_count
	check(animated == 6 and plain == 0, "décor : %d animés, %d fixes" % [animated, plain])
	finish()
