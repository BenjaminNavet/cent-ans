class_name BattleBridges
extends Node3D

## Lot EP3 : ponts du champ de bataille, d'après `BattleSim.get_terrain()["bridges"]` (position,
## lacet, longueur, largeur du tablier, largeur de l'eau franchie, hauteur du tablier, pierre ou
## bois). Assemblés avec les pièces du kit Blender (`building_kit.py`, même atlas que les maisons) :
## travées répétées au-dessus de l'eau (arches de pierre ou palées de pieux), culées en rampe sur
## chaque berge. Écume au pied des piles, entraînée par le courant (`battle_foam.gdshader`).
## Rendu seulement : l'emprise et la hauteur du tablier sont celles de la simulation.

const FOAM_SHADER := preload("res://shaders/battle_foam.gdshader")
## Pièces du kit (dimensions de `building_kit.py`).
const STONE_BAY := 9.0
const STONE_WIDTH := 6.0
const WOOD_BAY := 4.0
const WOOD_WIDTH := 5.0
const END_LENGTH := 5.0

var bridge_count: int = 0
var pier_count: int = 0


func build(terrain: BattleTerrain, bridges: Array, flow: float, snowy: bool) -> void:
	if bridges.is_empty():
		return
	var kit := BuildingKit.available() and not BuildingKit.models_of("bridge_stone_bay").is_empty()
	var batch := BuildingKit.Batch.new("snow" if snowy else "", 2200.0) if kit else null
	var foam: Array[Transform3D] = []
	for bridge in bridges:
		var stone := bool(bridge["stone"])
		var yaw := float(bridge["yaw"])
		var dir := Vector2(cos(yaw), sin(yaw))
		var centre := Vector2(float(bridge["x"]), float(bridge["z"]))
		var deck := float(bridge["deck"])
		var span := maxf(float(bridge["span"]), 2.0)
		var width := float(bridge["width"])
		var bay := STONE_BAY if stone else WOOD_BAY
		var count := maxi(int(round(span / bay)), 1)
		var stretch := span / (count * bay)
		var squeeze := width / (STONE_WIDTH if stone else WOOD_WIDTH)
		var kind := "stone" if stone else "wood"
		var basis := Basis(Vector3.UP, -yaw)
		if kit:
			var bay_model: String = BuildingKit.models_of("bridge_%s_bay" % kind)[0]
			var end_model: String = BuildingKit.models_of("bridge_%s_end" % kind)[0]
			for k in count:
				var along := -span * 0.5 + (k + 0.5) * span / count
				var p := centre + dir * along
				batch.add(bay_model, Transform3D(basis * Basis.from_scale(Vector3(stretch, 1.0, squeeze)), Vector3(p.x, deck, p.y)))
			# Culées : du bord de l'eau vers la berge, de chaque côté.
			for s in [-1.0, 1.0]:
				var p: Vector2 = centre + dir * (s * span * 0.5)
				var end_basis := Basis(Vector3.UP, -yaw + (PI if s < 0.0 else 0.0))
				batch.add(end_model, Transform3D(end_basis * Basis.from_scale(Vector3(1.0, 1.0, squeeze)), Vector3(p.x, deck, p.y)))
		else:
			_fallback(centre, yaw, float(bridge["length"]), width, deck, stone)
		bridge_count += 1
		# Écume au pied des piles (dans l'eau : les palées intermédiaires, les deux culées).
		if int(bridge.get("stream", -1)) >= 0:
			continue
		var level := terrain.water_level_at(centre.x)
		if level == -INF:
			continue
		var stream_dir := Vector2(dir.y, -dir.x) * flow
		for k in range(0, count + 1):
			var along := -span * 0.5 + k * span / count
			var p := centre + dir * along
			var t := Transform3D(Basis(Vector3.UP, -stream_dir.angle() + PI * 0.5), Vector3(p.x, level + 0.04, p.y))
			foam.append(t.translated_local(Vector3(0.0, 0.0, width * 0.5 + 3.5)))
			pier_count += 1
	if kit:
		batch.build(self)
	if not foam.is_empty():
		_build_foam(foam)


## Écume : un quad posé sur l'eau par pile, en aval (MultiMesh, un appel de dessin).
func _build_foam(transforms: Array[Transform3D]) -> void:
	var quad := QuadMesh.new()
	quad.size = Vector2(3.2, 9.0)
	quad.orientation = PlaneMesh.FACE_Y
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = quad
	mm.instance_count = transforms.size()
	for i in transforms.size():
		mm.set_instance_transform(i, transforms[i])
	var mat := ShaderMaterial.new()
	mat.shader = FOAM_SHADER
	var mi := MultiMeshInstance3D.new()
	mi.name = "PierFoam"
	mi.multimesh = mm
	mi.material_override = mat
	mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	mi.visibility_range_end = 900.0
	add_child(mi)


## Sans le kit (modèles absents) : tablier et parapets en boîtes.
func _fallback(centre: Vector2, yaw: float, length: float, width: float, deck: float, stone: bool) -> void:
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.62, 0.58, 0.52) if stone else Color(0.42, 0.32, 0.22)
	var box := BoxMesh.new()
	box.size = Vector3(length, 0.6, width)
	var mi := MeshInstance3D.new()
	mi.mesh = box
	mi.material_override = mat
	mi.transform = Transform3D(Basis(Vector3.UP, -yaw), Vector3(centre.x, deck - 0.3, centre.y))
	add_child(mi)
