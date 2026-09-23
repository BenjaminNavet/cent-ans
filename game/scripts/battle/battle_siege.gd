class_name BattleSiege
extends Node3D

## Ville assiégée d'une bataille de siège (spec M8 § 2), maillée depuis `BattleSim.get_siege()` :
## courtines crénelées, tours rondes à toit conique, porte (vantaux et linteau), place centrale
## pavée, maisons, et les machines des assiégeants (tours de siège, bélier, échelles des
## régiments qui escaladent). `update()` montre les dégâts : pans assombris puis effondrés
## (éboulis), porte enfoncée, tour de siège accostée. Rendu seulement : tout vient de la
## simulation. Maillages procéduraux low-poly (pas encore de modèles Blender).

const STONE := Color(0.62, 0.58, 0.50)
const STONE_DARK := Color(0.45, 0.42, 0.37)
const ROOF := Color(0.52, 0.22, 0.16)
const SLATE := Color(0.28, 0.30, 0.36)
const WOOD := Color(0.42, 0.29, 0.17)
const WOOD_DARK := Color(0.30, 0.20, 0.12)
const PAVING := Color(0.55, 0.52, 0.46)
const PLASTER := Color(0.82, 0.77, 0.66)
const LADDER := Color(0.72, 0.55, 0.30)

var siege: Dictionary = {}
var height_at: Callable
var _pieces: Array = []  # [{node, wall, rubble, material, hp_ratio}]
var _machines: Dictionary = {}  # unit id -> Node3D (tower / ram)
var _ladders: Dictionary = {}  # unit id -> Node3D (group of ladders)
var _ladder_mesh: ArrayMesh
var wall_height: float = 8.0
var thickness: float = 3.0


func build(p_siege: Dictionary, p_height_at: Callable) -> void:
	siege = p_siege
	height_at = p_height_at
	for child in get_children():
		child.queue_free()
	_pieces.clear()
	_machines.clear()
	_ladders.clear()
	wall_height = float(siege.get("wall_height", 8.0))
	thickness = float(siege.get("thickness", 3.0))
	_ladder_mesh = _make_ladder(wall_height + 1.5)
	for piece in siege.get("pieces", []):
		_build_piece(piece)
	for tower in siege.get("towers", []):
		_build_tower(tower)
	_build_square()
	_build_houses()


static func _material(color: Color, roughness: float = 0.95) -> StandardMaterial3D:
	var mat := StandardMaterial3D.new()
	mat.albedo_color = color
	mat.roughness = roughness
	return mat


func _ground(x: float, z: float) -> float:
	return height_at.call(x, z) if height_at.is_valid() else 0.0


## Un pan de courtine (ou la porte) : maçonnerie, merlons côté extérieur, éboulis cachés.
func _build_piece(piece: Dictionary) -> void:
	var a: Vector2 = piece["a"]
	var b: Vector2 = piece["b"]
	var mid := (a + b) * 0.5
	var length := a.distance_to(b)
	var dir := (b - a).normalized()
	var outward := Vector2(dir.y, -dir.x)
	var center: Vector2 = siege.get("center", Vector2(600, 560))
	if outward.dot(mid - center) < 0.0:
		outward = -outward
	var node := Node3D.new()
	node.name = "Piece%d" % int(piece["index"])
	var base_y := minf(_ground(a.x, a.y), _ground(b.x, b.y)) - 0.5
	node.position = Vector3(mid.x, base_y, mid.y)
	# Axe local X le long du pan, Z vers l'extérieur.
	node.basis = Basis(Vector3(dir.x, 0, dir.y), Vector3.UP, Vector3(outward.x, 0, outward.y))
	add_child(node)
	var mat := _material(STONE)
	var wall := Node3D.new()
	node.add_child(wall)
	var gate := str(piece["kind"]) == "gate"
	var height := wall_height + 0.5
	if gate:
		# Linteau au-dessus du passage, vantaux de bois dessous.
		var lintel := MeshInstance3D.new()
		var lintel_box := BoxMesh.new()
		lintel_box.size = Vector3(length + 0.2, height - 5.0, thickness + 0.6)
		lintel.mesh = lintel_box
		lintel.material_override = mat
		lintel.position = Vector3(0, 5.0 + (height - 5.0) * 0.5, 0)
		wall.add_child(lintel)
		for side in [-1.0, 1.0]:
			var door := MeshInstance3D.new()
			var door_box := BoxMesh.new()
			door_box.size = Vector3(length * 0.5 - 0.1, 5.0, 0.5)
			door.mesh = door_box
			door.material_override = _material(WOOD_DARK)
			door.position = Vector3(side * length * 0.25, 2.5, thickness * 0.5 - 0.2)
			door.name = "Door"
			wall.add_child(door)
	else:
		var body := MeshInstance3D.new()
		var box := BoxMesh.new()
		box.size = Vector3(length + 0.4, height, thickness)
		body.mesh = box
		body.material_override = mat
		body.position = Vector3(0, height * 0.5, 0)
		wall.add_child(body)
	# Merlons sur le bord extérieur.
	var merlon := BoxMesh.new()
	merlon.size = Vector3(1.0, 1.2, 0.6)
	var count := int(length / 2.2)
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = merlon
	mm.instance_count = count
	for i in count:
		var x := -length * 0.5 + (float(i) + 0.5) * length / float(count)
		mm.set_instance_transform(i, Transform3D(Basis(), Vector3(x, height + 0.6, thickness * 0.5 - 0.3)))
	var merlons := MultiMeshInstance3D.new()
	merlons.multimesh = mm
	merlons.material_override = mat
	wall.add_child(merlons)
	# Éboulis (pan effondré).
	var rubble := Node3D.new()
	rubble.visible = false
	node.add_child(rubble)
	var rng := RandomNumberGenerator.new()
	rng.seed = int(piece["index"]) * 7919 + 17
	var rubble_mat := _material(STONE_DARK)
	for i in int(length / 3.0) + 3:
		var block := MeshInstance3D.new()
		var block_box := BoxMesh.new()
		var s := rng.randf_range(1.2, 3.2)
		block_box.size = Vector3(s, s * 0.6, s * 0.8)
		block.mesh = block_box
		block.material_override = rubble_mat
		block.position = Vector3(rng.randf_range(-length * 0.5, length * 0.5), s * 0.2, rng.randf_range(-thickness, thickness * 1.5))
		block.rotation = Vector3(rng.randf() * 0.6, rng.randf() * TAU, rng.randf() * 0.6)
		rubble.add_child(block)
	# Un moignon de mur aux deux extrémités.
	for side in [-1.0, 1.0]:
		var stump := MeshInstance3D.new()
		var stump_box := BoxMesh.new()
		stump_box.size = Vector3(2.5, height * 0.45, thickness)
		stump.mesh = stump_box
		stump.material_override = rubble_mat
		stump.position = Vector3(side * (length * 0.5 - 1.25), height * 0.22, 0)
		rubble.add_child(stump)
	_pieces.append({"node": node, "wall": wall, "rubble": rubble, "material": mat, "gate": gate, "ratio": 1.0})


func _build_tower(tower: Dictionary) -> void:
	var x := float(tower["x"])
	var z := float(tower["z"])
	var r := float(tower["radius"])
	var h := float(tower["height"])
	var node := Node3D.new()
	node.position = Vector3(x, _ground(x, z) - 0.5, z)
	add_child(node)
	var body := MeshInstance3D.new()
	var cylinder := CylinderMesh.new()
	cylinder.top_radius = r
	cylinder.bottom_radius = r * 1.08
	cylinder.height = h
	cylinder.radial_segments = 12
	body.mesh = cylinder
	body.material_override = _material(STONE)
	body.position = Vector3(0, h * 0.5, 0)
	node.add_child(body)
	var roof := MeshInstance3D.new()
	var cone := CylinderMesh.new()
	cone.top_radius = 0.0
	cone.bottom_radius = r * 1.25
	cone.height = r * 1.6
	cone.radial_segments = 12
	roof.mesh = cone
	roof.material_override = _material(SLATE, 0.8)
	roof.position = Vector3(0, h + r * 0.8, 0)
	node.add_child(roof)


func _build_square() -> void:
	var center: Vector2 = siege.get("center", Vector2(600, 560))
	var radius := float(siege.get("square_radius", 35.0))
	var disc := MeshInstance3D.new()
	var cylinder := CylinderMesh.new()
	cylinder.top_radius = radius
	cylinder.bottom_radius = radius
	cylinder.height = 0.3
	cylinder.radial_segments = 32
	disc.mesh = cylinder
	disc.material_override = _material(PAVING)
	disc.position = Vector3(center.x, _ground(center.x, center.y) + 0.05, center.y)
	disc.name = "Square"
	add_child(disc)
	# Puits au centre.
	var well := MeshInstance3D.new()
	var ring := CylinderMesh.new()
	ring.top_radius = 1.4
	ring.bottom_radius = 1.4
	ring.height = 1.0
	well.mesh = ring
	well.material_override = _material(STONE_DARK)
	well.position = disc.position + Vector3(0, 0.5, 0)
	add_child(well)


## Maisons et église dans la moitié arrière de la ville (loin du front et de la place).
func _build_houses() -> void:
	var center: Vector2 = siege.get("center", Vector2(600, 560))
	var radius := float(siege.get("square_radius", 35.0))
	var rng := RandomNumberGenerator.new()
	rng.seed = 1340
	var walls: Array = siege.get("pieces", [])
	var placed := 0
	var tries := 0
	while placed < 28 and tries < 400:
		tries += 1
		var p := center + Vector2(rng.randf_range(-115, 115), rng.randf_range(-40, 120))
		if p.distance_to(center) < radius + 12.0:
			continue
		var near_wall := false
		for piece in walls:
			var q := Geometry2D.get_closest_point_to_segment(p, piece["a"], piece["b"])
			if q.distance_to(p) < 22.0:
				near_wall = true
				break
		if near_wall or not Geometry2D.is_point_in_polygon(p, _ring()):
			continue
		_house(p, rng.randf_range(6.0, 11.0), rng.randf_range(5.0, 8.0), rng.randf_range(4.0, 7.0), rng.randf() * TAU)
		placed += 1
	# L'église, au fond de la ville.
	var church := center + Vector2(0, 75)
	if Geometry2D.is_point_in_polygon(church, _ring()):
		_house(church, 22.0, 10.0, 11.0, 0.0)
		var spire := MeshInstance3D.new()
		var cone := CylinderMesh.new()
		cone.top_radius = 0.0
		cone.bottom_radius = 3.2
		cone.height = 14.0
		cone.radial_segments = 8
		spire.mesh = cone
		spire.material_override = _material(SLATE, 0.8)
		spire.position = Vector3(church.x - 8.0, _ground(church.x, church.y) + 11.0 + 7.0 + 4.0, church.y)
		add_child(spire)


func _ring() -> PackedVector2Array:
	var ring := PackedVector2Array()
	for piece in siege.get("pieces", []):
		ring.append(piece["a"])
	return ring


func _house(p: Vector2, w: float, d: float, h: float, angle: float) -> void:
	var node := Node3D.new()
	node.position = Vector3(p.x, _ground(p.x, p.y) - 0.2, p.y)
	node.rotation.y = angle
	add_child(node)
	var body := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = Vector3(w, h, d)
	body.mesh = box
	body.material_override = _material(PLASTER)
	body.position = Vector3(0, h * 0.5, 0)
	node.add_child(body)
	var roof := MeshInstance3D.new()
	var prism := PrismMesh.new()
	prism.size = Vector3(d + 0.8, h * 0.6, w + 0.6)
	roof.mesh = prism
	roof.material_override = _material(ROOF, 0.85)
	roof.position = Vector3(0, h + h * 0.3, 0)
	roof.rotation.y = PI * 0.5
	node.add_child(roof)


## Dégâts des pans et machines/échelles des assiégeants (chaque image).
func update(p_siege: Dictionary, units: Array) -> void:
	if p_siege.is_empty():
		return
	var pieces: Array = p_siege.get("pieces", [])
	for i in mini(pieces.size(), _pieces.size()):
		var piece: Dictionary = pieces[i]
		var view: Dictionary = _pieces[i]
		var ratio := clampf(float(piece["hp"]) / maxf(float(piece["max_hp"]), 1.0), 0.0, 1.0)
		if absf(ratio - float(view["ratio"])) < 0.01:
			continue
		view["ratio"] = ratio
		var intact := bool(piece["intact"])
		var wall: Node3D = view["wall"]
		var rubble: Node3D = view["rubble"]
		if view["gate"]:
			for door in wall.get_children():
				if door.name.begins_with("Door"):
					door.visible = intact
			rubble.visible = false
		else:
			wall.visible = intact
			rubble.visible = not intact
			# Les pans battus s'abaissent un peu et noircissent.
			wall.scale = Vector3(1, 0.8 + 0.2 * ratio, 1)
		var mat: StandardMaterial3D = view["material"]
		mat.albedo_color = STONE_DARK.lerp(STONE, ratio)
	_update_machines(units)


func _update_machines(units: Array) -> void:
	var seen := {}
	for unit in units:
		var id := int(unit["id"])
		var render := str(unit.get("render", ""))
		var present: bool = unit["present"]
		if render == "tower" or render == "ram":
			if not _machines.has(id):
				_machines[id] = _make_tower() if render == "tower" else _make_ram()
				add_child(_machines[id])
			var machine: Node3D = _machines[id]
			machine.visible = present
			if present:
				var x := float(unit["x"])
				var z := float(unit["z"])
				machine.position = Vector3(x, _ground(x, z), z)
				machine.rotation.y = float(unit["facing"])
		if bool(unit.get("ladders", false)) and present:
			seen[id] = true
			if not _ladders.has(id):
				_ladders[id] = _make_ladder_group(float(unit["width"]))
				add_child(_ladders[id])
			var group: Node3D = _ladders[id]
			group.visible = true
			var x := float(unit["x"])
			var z := float(unit["z"])
			group.position = Vector3(x, _ground(x, z), z)
			group.rotation.y = float(unit["facing"])
	for id in _ladders:
		if not seen.has(id):
			(_ladders[id] as Node3D).visible = false


## Tour de siège (beffroi) : caisse de bois sur roues, peaux, pont-levis ; face avant vers +Z local.
func _make_tower() -> Node3D:
	var node := Node3D.new()
	var h := wall_height + 4.0
	var body := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = Vector3(5.0, h, 5.0)
	body.mesh = box
	body.material_override = _material(WOOD)
	body.position = Vector3(0, h * 0.5 + 0.8, 0)
	node.add_child(body)
	var hides := MeshInstance3D.new()
	var hide_box := BoxMesh.new()
	hide_box.size = Vector3(5.3, h * 0.55, 5.3)
	hides.mesh = hide_box
	hides.material_override = _material(Color(0.55, 0.45, 0.32))
	hides.position = Vector3(0, h * 0.72 + 0.8, 0)
	node.add_child(hides)
	var bridge := MeshInstance3D.new()
	var bridge_box := BoxMesh.new()
	bridge_box.size = Vector3(3.2, 0.3, 4.0)
	bridge.mesh = bridge_box
	bridge.material_override = _material(WOOD_DARK)
	bridge.position = Vector3(0, wall_height + 0.8, 4.2)
	node.add_child(bridge)
	for x in [-2.2, 2.2]:
		for z in [-1.8, 1.8]:
			var wheel := MeshInstance3D.new()
			var cylinder := CylinderMesh.new()
			cylinder.top_radius = 0.8
			cylinder.bottom_radius = 0.8
			cylinder.height = 0.4
			wheel.mesh = cylinder
			wheel.material_override = _material(WOOD_DARK)
			wheel.position = Vector3(x, 0.8, z)
			wheel.rotation.z = PI * 0.5
			node.add_child(wheel)
	return node


## Bélier : appentis couvert (toit en bâtière) et poutre à tête de fer.
func _make_ram() -> Node3D:
	var node := Node3D.new()
	var roof := MeshInstance3D.new()
	var prism := PrismMesh.new()
	prism.size = Vector3(3.6, 2.2, 8.0)
	roof.mesh = prism
	roof.material_override = _material(Color(0.5, 0.4, 0.28))
	roof.position = Vector3(0, 2.3, 0)
	node.add_child(roof)
	var beam := MeshInstance3D.new()
	var cylinder := CylinderMesh.new()
	cylinder.top_radius = 0.35
	cylinder.bottom_radius = 0.35
	cylinder.height = 9.0
	beam.mesh = cylinder
	beam.material_override = _material(WOOD_DARK)
	beam.position = Vector3(0, 1.2, 0.8)
	beam.rotation.x = PI * 0.5
	node.add_child(beam)
	var head := MeshInstance3D.new()
	var head_box := BoxMesh.new()
	head_box.size = Vector3(0.9, 0.9, 0.9)
	head.mesh = head_box
	head.material_override = _material(Color(0.35, 0.36, 0.38), 0.4)
	head.position = Vector3(0, 1.2, 5.4)
	node.add_child(head)
	return node


## Échelle de `length` m : deux montants et des barreaux (couleurs de sommets).
static func _make_ladder(length: float) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for x in [-0.45, 0.45]:
		BattleMeshes.add_box(st, Vector3(x, length * 0.5, 0), Vector3(0.18, length, 0.18), LADDER)
	var rungs := int(length / 0.5)
	for i in rungs:
		BattleMeshes.add_box(st, Vector3(0, 0.3 + i * 0.5, 0), Vector3(0.9, 0.1, 0.1), LADDER)
	var mesh := st.commit()
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.vertex_color_is_srgb = true
	mesh.surface_set_material(0, mat)
	return mesh


## Quelques échelles réparties sur le front du régiment, appuyées vers l'avant (+Z local).
func _make_ladder_group(width: float) -> Node3D:
	var node := Node3D.new()
	var count := clampi(int(width / 6.0), 2, 7)
	for i in count:
		var ladder := MeshInstance3D.new()
		ladder.mesh = _ladder_mesh
		var x := -width * 0.5 + (float(i) + 0.5) * width / float(count)
		ladder.position = Vector3(x, 0, 1.0)
		ladder.rotation.x = 0.28
		node.add_child(ladder)
	return node
