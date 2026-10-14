extends TestCase

## Lot CV-A2 : mouvement des bêtes et des roues sur les glb DN Meshy. Le shader de sommets ne tourne pas
## sans GPU : on en rejoue ici, sur les sommets du maillage tel que FolkModels le construit (orienté vers
## +Z), les masques des pattes, du dos et des roues avec les réglages posés dans les uniformes.
## Usage : godot --headless --path game --script res://tests/cv_a2_test.gd

const BEASTS := {"cow": "animal_cattle_red", "ox": "animal_ox_draught", "sheep": "animal_sheep_wool", "horse": "animal_horse_draught"}
const CARTS := {"merchant_cart": "cart_merchant", "peasant_cart": "env_cart_ox_peasant", "dead_cart": "folk_dead_cart", "plough": "folk_plough"}


func _vertices(mesh: Mesh) -> PackedVector3Array:
	var out := PackedVector3Array()
	for s in mesh.get_surface_count():
		out.append_array(mesh.surface_get_arrays(s)[Mesh.ARRAY_VERTEX])
	return out


## Pas des pattes de `animal_pose` (déplacement avant et levée), pour la phase de pas `turn_cycles`.
func _leg_offset(v: Vector3, p: Dictionary, turn_cycles: float) -> Vector2:
	var zr := v.z - float(p["anchor_m"])
	var half := float(p["half_m"])
	var belly := float(p["belly_m"])
	if absf(zr) >= half * 0.95:
		return Vector2.ZERO
	var leg_w := clampf((belly - v.y) / belly, 0.0, 1.0)
	var front := 0.25 if zr >= 0.0 else 0.0
	var leg_u: float = turn_cycles + (0.0 if v.x >= 0.0 else 0.5) + front
	return Vector2(AnimalMotion.lut_at(p["swing_lut"], leg_u) * float(p["swing_m"]) * leg_w, AnimalMotion.lut_at(p["lift_lut"], leg_u) * float(p["lift_m"]) * leg_w * leg_w)


## Roue de `animal_pose` : sommet tourné autour de l'essieu s'il est dans le masque, sinon inchangé.
func _wheel(v: Vector3, p: Dictionary, angle: float) -> Vector3:
	var r := float(p["wheel_radius_m"])
	if absf(v.x) <= float(p["wheel_x_min_m"]):
		return v
	for axle_z in p["axle_z_m"]:
		if Vector2(v.y - r, v.z - float(axle_z)).length() < r + 0.03:
			var d := Vector2(v.y - r, v.z - float(axle_z))
			var c := cos(angle)
			var s := sin(angle)
			return Vector3(v.x, r + d.x * c - d.y * s, float(axle_z) + d.y * c + d.x * s)
	return v


func _init() -> void:
	for role in BEASTS:
		var dn: String = BEASTS[role]
		var p := AnimalMotion.model_params(dn)
		check(not p.is_empty() and float(p["half_m"]) > 0.0, "%s : entrée DN absente" % dn)
		var mesh := FolkModels.prop_mesh(role)
		check(FolkModels.prop_source(role).contains("/dn/"), "%s : glb DN attendu" % role)
		if p.is_empty() or mesh == null:
			continue
		var mat := mesh.surface_get_material(0) as ShaderMaterial
		check(float(mat.get_shader_parameter("am_on")) == 1.0, "%s : am_on" % role)
		var body: Vector4 = mat.get_shader_parameter("am_body")
		check(is_equal_approx(body.x, float(p["anchor_m"])) and is_equal_approx(body.z, float(p["belly_m"])), "%s : am_body" % role)
		var verts := _vertices(mesh)
		var box := mesh.get_aabb()
		var belly := float(p["belly_m"])
		# Les mesures tiennent dans la boîte du maillage (garrot sous le sommet, ventre sous le garrot).
		check(float(p["withers_m"]) <= box.end.y + 0.01 and belly < float(p["withers_m"]), "%s : hauteurs incohérentes" % role)
		check(float(p["anchor_m"]) - float(p["half_m"]) >= box.position.z - 0.12, "%s : croupe hors du maillage" % role)
		var legs_moved := 0
		var legs_total := 0
		var back_moved := 0
		var back_total := 0
		var feet_lift := 0.0
		var zone_end := float(p["anchor_m"]) + float(p["half_m"]) * 0.95
		for v in verts:
			var zr := v.z - float(p["anchor_m"])
			var in_body := absf(zr) < float(p["half_m"]) * 0.95
			var worst := 0.0
			for i in 16:
				var o := _leg_offset(v, p, float(i) / 16.0)
				worst = maxf(worst, o.length())
				if v.y < belly * 0.3:
					feet_lift = maxf(feet_lift, o.y)
			if v.y < belly * 0.5 and in_body:
				legs_total += 1
				legs_moved += 1 if worst > 0.01 else 0
			elif v.y > float(p["withers_m"]) * 0.8 and in_body and v.z < zone_end - 0.1:
				back_total += 1
				back_moved += 1 if worst > 0.001 else 0
		check(legs_total >= 30, "%s : peu de sommets de pattes (%d)" % [role, legs_total])
		check(legs_moved >= legs_total * 0.6, "%s : pattes immobiles (%d / %d)" % [role, legs_moved, legs_total])
		check(back_total >= 10, "%s : peu de sommets de dos (%d)" % [role, back_total])
		check(back_moved == 0, "%s : le dos suit les pattes (%d / %d)" % [role, back_moved, back_total])
		check(feet_lift > 0.005, "%s : sabots non levés (%.3f)" % [role, feet_lift])
		# Tête : des sommets dans la zone du cou, au-dessus du ventre.
		var head := 0
		for v in verts:
			if v.z - float(p["anchor_m"]) > float(p["neck_start_m"]) + 0.1 and v.y > belly * 0.75:
				head += 1
		check(head >= 30, "%s : masque de tête vide (%d)" % [role, head])
	for role in CARTS:
		var dn: String = CARTS[role]
		var p := AnimalMotion.model_params(dn)
		check(not p.is_empty() and float(p["wheel_radius_m"]) > 0.0, "%s : entrée DN absente" % dn)
		var mesh := FolkModels.prop_mesh(role)
		check(FolkModels.prop_source(role).contains("/dn/"), "%s : glb DN attendu" % role)
		if p.is_empty() or mesh == null:
			continue
		var mat := mesh.surface_get_material(0) as ShaderMaterial
		var am_cart: Vector4 = mat.get_shader_parameter("am_cart")
		var am_axle: Vector4 = mat.get_shader_parameter("am_axle")
		var axles: Array = p["axle_z_m"]
		check(is_equal_approx(am_cart.x, float(p["wheel_radius_m"])) and is_equal_approx(am_axle.x, float(axles[0])), "%s : uniformes de roue" % role)
		check((am_axle.z > 0.5) == (axles.size() > 1), "%s : 2e essieu" % role)
		check(float(mat.get_shader_parameter("am_on")) == 0.0, "%s : pas de bête sur la charrette seule" % role)
		var box := mesh.get_aabb()
		var r := float(p["wheel_radius_m"])
		check(2.0 * r <= box.size.y + 0.05, "%s : roue plus haute que le modèle" % role)
		# Par essieu : assez de sommets de roue ; ils tournent en gardant leur distance à l'essieu ; les autres restent.
		var verts := _vertices(mesh)
		for axle_z in axles:
			var count := 0
			var moved := 0
			for v in verts:
				if absf(v.x) > float(p["wheel_x_min_m"]) and Vector2(v.y - r, v.z - float(axle_z)).length() < r + 0.03:
					count += 1
					var w := _wheel(v, p, 1.0)
					var before := Vector2(v.y - r, v.z - float(axle_z)).length()
					var after := Vector2(w.y - r, w.z - float(axle_z)).length()
					check(absf(before - after) < 1e-4 and is_equal_approx(v.x, w.x), "%s : la roue ne tourne pas autour de son axe" % role)
					moved += 1 if Vector2(w.y - v.y, w.z - v.z).length() > 0.02 else 0
			check(count >= 60, "%s essieu %.2f : masque de roue maigre (%d)" % [role, axle_z, count])
			check(moved >= count * 0.7, "%s essieu %.2f : %d / %d sommets tournent" % [role, axle_z, moved, count])
		var still := 0
		for v in verts:
			if absf(v.x) <= float(p["wheel_x_min_m"]):
				still += 1 if _wheel(v, p, 1.0) == v else 0
		check(still > 0, "%s : la caisse tourne avec les roues" % role)
		# Essieux dans la boîte du modèle.
		for axle_z in axles:
			check(float(axle_z) > box.position.z + 0.3 and float(axle_z) < box.end.z - 0.3, "%s : essieu %.2f hors du modèle" % [role, axle_z])
	finish()
