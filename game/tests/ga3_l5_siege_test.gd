extends TestCase

## GA3-L5 : trébuchet et bélier générés branchés sur les engins animés (`SiegeEnginesFx`,
## ADR 0140 § L5). Vérifie : la variante GA3 (`ga3` de `data/fx/siege_engines.json`) est posée
## pour le trébuchet et le bélier, proche et lointaine, avec la hiérarchie du modèle procédural
## (nœuds animés présents, pièces GA3 texturées sur `Frame`, `ArmBeam`, `CounterweightBox`,
## `Shed`, `Wheel_i`, `Beam`) ; pivot de la verge sur l'axe (6,4 m), verge de 8,5 m côté fronde
## et 2,2 m côté contrepoids ; la verge bascule et la pierre part au bout de la fronde ; roues sur
## leur centre ; réglages du bélier remplacés (roues, cordes) ; engins sans variante (mangonneau,
## bombarde, beffroi) inchangés.
## Usage : godot --headless --path game --script res://tests/ga3_l5_siege_test.gd


## Boîte englobante (repère de `ref`) des maillages sous `node`.
func _aabb(node: Node3D, ref: Node3D) -> AABB:
	var box := AABB()
	var first := true
	var meshes: Array = [node] if node is MeshInstance3D else []
	meshes.append_array(node.find_children("*", "MeshInstance3D", true, false))
	for child in meshes:
		var mi := child as MeshInstance3D
		var local := ref.global_transform.affine_inverse() * mi.global_transform * mi.get_aabb()
		box = local if first else box.merge(local)
		first = false
	return box


## Vrai quand le maillage porte une matière GA3 texturée (albédo cuit).
func _textured(mi: MeshInstance3D) -> bool:
	if mi == null or mi.mesh == null:
		return false
	for s in mi.mesh.get_surface_count():
		var mat := mi.get_active_material(s) as BaseMaterial3D
		if mat != null and mat.albedo_texture != null and mat.resource_name.begins_with("Ga3"):
			return true
	return false


func _node(root: Node, node_name: String) -> Node3D:
	return root.find_child(node_name, true, false) as Node3D


func _trebuchet() -> void:
	for model in ["trebuchet", "trebuchet_lod"]:
		var node := SiegeEnginesFx.instantiate(model)
		check(node != null, "%s instantiates" % model)
		if node == null:
			continue
		root.add_child(node)
		var variant := str(node.get_meta("ga3", ""))
		check(variant == "ga3_" + model, "%s: GA3 variant (%s)" % [model, variant])
		for part in ["Frame", "Arm", "ArmBeam", "Counterweight", "CounterweightBox", "Sling", "SlingRope", "Stone", "Winch", "Axle", "WinchFrame"]:
			check(_node(node, part) != null, "%s: node %s" % [model, part])
		for part in ["Frame", "ArmBeam", "CounterweightBox"]:
			check(_textured(_node(node, part) as MeshInstance3D), "%s: %s dressed with the GA3 texture" % [model, part])
		var arm := _node(node, "Arm")
		var counter := _node(node, "Counterweight")
		var sling := _node(node, "Sling")
		if arm == null or counter == null or sling == null:
			node.free()
			continue
		check(arm.position.distance_to(Vector3(0, 6.4, 0)) < 0.01, "%s: arm pivot on the axle (%s)" % [model, arm.position])
		check(counter.position.distance_to(Vector3(0, 0, 2.2)) < 0.01, "%s: counterweight hinge" % model)
		check(sling.position.distance_to(Vector3(0, 0, -8.5)) < 0.01, "%s: sling at the long end" % model)
		# Verge GA3 : du bout de la fronde (-8,5 m) au crochet (+2,4 m), autour du pivot.
		var beam := _aabb(_node(node, "ArmBeam"), arm)
		check(absf(beam.position.z + 8.5) < 0.35, "%s: arm long end at -8.5 m (%.2f)" % [model, beam.position.z])
		check(absf(beam.end.z - 2.4) < 0.35, "%s: arm short end at +2.4 m (%.2f)" % [model, beam.end.z])
		check(absf(beam.get_center().y) < 0.4 and beam.size.y < 1.8, "%s: arm straight through the pivot (%s)" % [model, beam])
		# Bâti : paliers à hauteur d'axe, posé au sol ; contrepoids entre les montants.
		var frame := _aabb(_node(node, "Frame"), node)
		check(frame.position.y > -0.05 and frame.position.y < 0.3, "%s: frame on the ground (%.2f)" % [model, frame.position.y])
		check(absf(frame.end.y - 6.4) < 0.8, "%s: frame top at the axle (%.2f)" % [model, frame.end.y])
		var box := _aabb(_node(node, "CounterweightBox"), counter)
		check(box.end.y <= 0.05 and box.position.y > -3.2 and box.size.x < 2.0, "%s: box hangs from the hinge (%s)" % [model, box])
		# Pose : la verge bascule (armée -> au repos) et la boîte reste pendue sous l'axe.
		var fx := SiegeEnginesFx.new()
		root.add_child(fx)
		var c: Dictionary = SiegeEnginesFx.settings()["trebuchet"]
		var unit := {"reload": 0.0, "reload_period": 12.0}
		fx._pose_trebuchet(node, c, unit, -1.0, false)
		check(absf(arm.rotation.x - deg_to_rad(float(c["cocked_deg"]))) < 0.01, "%s: cocked pose" % model)
		var cocked := _aabb(_node(node, "ArmBeam"), node)
		fx._pose_trebuchet(node, c, unit, float(c["swing_s"]) + float(c["settle_s"]) + 5.0, true)
		unit["reload"] = 11.0
		fx._pose_trebuchet(node, c, unit, float(c["swing_s"]) + float(c["settle_s"]) + 5.0, true)
		var rest := _aabb(_node(node, "ArmBeam"), node)
		check(rest.end.y > cocked.end.y + 3.0, "%s: arm swings up (%.1f -> %.1f)" % [model, cocked.end.y, rest.end.y])
		var hanging := _aabb(_node(node, "CounterweightBox"), node)
		check(hanging.position.y > 0.9, "%s: box clears the base at rest (%.2f)" % [model, hanging.position.y])
		check(absf(hanging.get_center().x) < 0.3, "%s: box between the posts" % model)
		var stone := fx._stone_at_release(node, c)
		check(stone.y > 6.0 and stone.distance_to(Vector3(0, 6.4, 0)) > 8.0, "%s: stone released at the sling end (%s)" % [model, stone])
		fx.free()
		node.free()


func _ram() -> void:
	for model in ["ram", "ram_lod"]:
		var node := SiegeEnginesFx.instantiate(model)
		check(node != null, "%s instantiates" % model)
		if node == null:
			continue
		root.add_child(node)
		var c := SiegeEnginesFx.kind_settings("ram", node)
		var base: Dictionary = SiegeEnginesFx.settings()["ram"]
		check(str(node.get_meta("ga3", "")) == "ga3_" + model, "%s: GA3 variant" % model)
		var over: Dictionary = SiegeEnginesFx.settings()["ga3"]["ram"]
		check(is_equal_approx(float(c["wheel_radius"]), float(over["wheel_radius"])), "%s: GA3 wheel radius" % model)
		check(is_equal_approx(float(c["beam_drop"]), float(over["beam_drop"])), "%s: GA3 beam drop" % model)
		check(is_equal_approx(float(c["sway_deg"]), float(base["sway_deg"])), "%s: other settings kept" % model)
		for part in ["Shed", "BeamPivot", "Beam", "Wheel_0", "Wheel_1", "Wheel_2", "Wheel_3"]:
			check(_node(node, part) != null, "%s: node %s" % [model, part])
		for part in ["Shed", "Beam", "Wheel_0", "Wheel_3"]:
			check(_textured(_node(node, part) as MeshInstance3D), "%s: %s dressed with the GA3 texture" % [model, part])
		var sides := {}
		for i in 4:
			var wheel := _node(node, "Wheel_%d" % i)
			if wheel == null:
				continue
			var box := _aabb(wheel, wheel)
			check(box.get_center().distance_to(Vector3(box.get_center().x, 0, 0)) < 0.25, "Wheel_%d centred on its axle (%s)" % [i, box])
			check(absf(wheel.position.y - float(c["wheel_radius"])) < 0.2, "Wheel_%d radius (%.2f)" % [i, wheel.position.y])
			sides[signf(wheel.position.x) * 10.0 + signf(wheel.position.z)] = true
		check(sides.size() == 4, "%s: one wheel per corner" % model)
		var pivot := _node(node, "BeamPivot")
		var beam := _aabb(_node(node, "Beam"), pivot)
		check(absf(-beam.position.y - float(c["beam_drop"])) < 0.6, "%s: beam hangs %.2f m under its pivot" % [model, -beam.position.y])
		check(beam.end.z > 4.8 and beam.position.z < -2.0, "%s: beam along the ram, head forward (%s)" % [model, beam])
		node.free()


func _others() -> void:
	for model in ["mangonel", "bombard", "siege_tower", "siege_tower_lod"]:
		check(SiegeEnginesFx.ga3_variant(model) == "", "%s: no GA3 variant" % model)
		var node := SiegeEnginesFx.instantiate(model)
		check(node != null and not node.has_meta("ga3"), "%s: procedural model" % model)
		if node != null:
			node.free()


func _init() -> void:
	await process_frame  # nœuds dans l'arbre : transformations globales valides
	_trebuchet()
	_ram()
	_others()
	finish()
