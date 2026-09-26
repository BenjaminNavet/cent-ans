class_name BattleSiege
extends Node3D

## Ville assiégée d'une bataille de siège (spec M8 § 2), maillée depuis `BattleSim.get_siege()` :
## courtines crénelées, tours rondes à toit conique, porte (vantaux et linteau), place centrale
## pavée, maisons, et les machines des assiégeants (tours de siège, bélier, échelles des
## régiments qui escaladent). `update()` montre les dégâts : pans assombris puis effondrés
## (éboulis), porte enfoncée, tour de siège accostée. Rendu seulement : tout vient de la
## simulation. Maillages procéduraux ; matières PBR Poly Haven (CC0) projetées en triplanaire
## monde (lot V4) : pierre des murailles, ardoise, tuiles, chaume, enduit, pavés, bois.

const STONE := Color(0.95, 0.9, 0.8)
const STONE_DARK := Color(0.55, 0.52, 0.47)
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
var _slit_mat: StandardMaterial3D  # matière des archères, partagée entre toutes les tours
var house_sites: Array = []  # F5c/BR3 : [{p, radius, length, depth, yaw, rows, church}] = obstacles de la simulation
var _fx: WallCollapseFx  # S1 : effondrement physique (rendu seulement)
var fire_fx: Node3D = null  # S2 : incendies des maisons (`siege_fire_fx.gd`)
var _kit_batch: BuildingKit.Batch = null  # BR1 : bâtiments du kit (MultiMesh par modèle)
var _kit_sites: Dictionary = {}  # BR1 : indice de maison → [[poignée, ruine, Transform3D], …]
var _kit_ruins: Dictionary = {}  # indice de maison → true (ruine déjà posée)
var external_ladders := false  # SG1 : échelles posées contre le mur par `SiegeAssaultFx`


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
	_fx = WallCollapseFx.new()
	_fx.name = "CollapseFx"
	add_child(_fx)
	_fx.setup(height_at)
	# Tours (statiques, aucun dégât suivi contrairement aux pans de courtine) : les archères,
	# de taille fixe, sont regroupées après coup par `BattleSiegeBatcher` (le fût conique et son
	# couronnement restent des nœuds, rayon/hauteur variables par tour).
	var towers_root := Node3D.new()
	towers_root.name = "Towers"
	add_child(towers_root)
	for tower in siege.get("towers", []):
		_build_tower(towers_root, SiegeAssaultFx.gatehouse_tower(siege, tower))
	BattleSiegeBatcher.batch_and_replace(towers_root)
	_build_square()
	_build_houses()
	fire_fx = preload("res://scripts/battle/siege_fire_fx.gd").new()
	add_child(fire_fx)
	fire_fx.setup(self, height_at)


static func _material(color: Color, roughness: float = 0.95) -> StandardMaterial3D:
	var mat := StandardMaterial3D.new()
	mat.albedo_color = color
	mat.roughness = roughness
	return mat


const TEXTURES := {
	"stone": ["castle_wall_varriation", 0.22],
	"slate": ["roof_slates_02", 0.45],
	"tiles": ["clay_roof_tiles_02", 0.4],
	"thatch": ["thatch_roof_angled", 0.3],
	"plaster": ["plastered_wall_02", 0.25],
	"paving": ["cobblestone_floor_01", 0.3],
	"wood": ["wood_planks", 0.35],
}


## Matière texturée (albédo + normale Poly Haven) en projection triplanaire monde : les
## BoxMesh et CylinderMesh n'ont pas à porter d'UV cohérentes. `scale` = répétitions par mètre.
static func _textured(kind: String, tint: Color = Color.WHITE, roughness: float = 0.92) -> StandardMaterial3D:
	var entry: Array = TEXTURES[kind]
	var base := "res://assets/textures/battle/%s" % entry[0]
	var mat := StandardMaterial3D.new()
	mat.albedo_texture = load(base + "_diff.jpg")
	mat.albedo_color = tint
	mat.normal_enabled = true
	mat.normal_texture = load(base + "_nor.jpg")
	mat.normal_scale = 0.8
	mat.roughness = roughness
	mat.uv1_triplanar = true
	mat.uv1_world_triplanar = true
	mat.uv1_triplanar_sharpness = 4.0
	var scale: float = entry[1]
	mat.uv1_scale = Vector3(scale, scale, scale)
	mat.texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS_ANISOTROPIC
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
	var mat := _textured("stone", STONE)
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
			door.material_override = _textured("wood", Color(0.55, 0.45, 0.38))
			door.position = Vector3(side * length * 0.25, 2.5, thickness * 0.5 - 0.2)
			door.name = "Door"
			wall.add_child(door, true)  # nom lisible (« Door2 ») : les deux vantaux se cachent
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
	var rubble_mat := _textured("stone", STONE_DARK)
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
	_pieces.append({"node": node, "wall": wall, "rubble": rubble, "material": mat, "gate": gate, "ratio": 1.0, "intact": true})


func _build_tower(parent: Node3D, tower: Dictionary) -> void:
	var x := float(tower["x"])
	var z := float(tower["z"])
	var r := float(tower["radius"])
	var h := float(tower["height"])
	var node := Node3D.new()
	node.position = Vector3(x, _ground(x, z) - 0.5, z)
	parent.add_child(node)
	var body := MeshInstance3D.new()
	var cylinder := CylinderMesh.new()
	cylinder.top_radius = r
	cylinder.bottom_radius = r * 1.08
	cylinder.height = h
	cylinder.radial_segments = 12
	body.mesh = cylinder
	body.material_override = _textured("stone", STONE)
	body.position = Vector3(0, h * 0.5, 0)
	node.add_child(body)
	# Couronnement : corbeaux (anneau en encorbellement) puis, une tour sur deux, un toit
	# d'ardoise proportionné ; sinon une plate-forme crénelée.
	var stone := _textured("stone", STONE)
	var corbel := MeshInstance3D.new()
	var ring := CylinderMesh.new()
	ring.top_radius = r * 1.14
	ring.bottom_radius = r * 1.02
	ring.height = 1.4
	ring.radial_segments = 14
	corbel.mesh = ring
	corbel.material_override = stone
	corbel.position = Vector3(0, h - 0.2, 0)
	node.add_child(corbel)
	var roofed := int(absf(x * 7.0 + z * 13.0)) % 2 == 0
	if roofed:
		var parapet := MeshInstance3D.new()
		var wall_ring := CylinderMesh.new()
		wall_ring.top_radius = r * 1.14
		wall_ring.bottom_radius = r * 1.14
		wall_ring.height = 1.0
		wall_ring.radial_segments = 14
		parapet.mesh = wall_ring
		parapet.material_override = stone
		parapet.position = Vector3(0, h + 1.0, 0)
		node.add_child(parapet)
		var roof := MeshInstance3D.new()
		var cone := CylinderMesh.new()
		cone.top_radius = 0.0
		cone.bottom_radius = r * 1.12
		cone.height = r * 1.55
		cone.radial_segments = 14
		roof.mesh = cone
		roof.material_override = _textured("slate", Color(0.55, 0.56, 0.6), 0.75)
		roof.position = Vector3(0, h + 1.5 + r * 0.775, 0)
		node.add_child(roof)
	else:
		var merlon := BoxMesh.new()
		merlon.size = Vector3(0.9, 1.3, 0.7)
		var count := maxi(int(TAU * r * 1.1 / 2.0), 6)
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = merlon
		mm.instance_count = count
		for i in count:
			var a := TAU * float(i) / float(count)
			var basis := Basis(Vector3.UP, -a)
			mm.set_instance_transform(i, Transform3D(basis, Vector3(cos(a) * r * 1.08, h + 1.1, sin(a) * r * 1.08)))
		var merlons := MultiMeshInstance3D.new()
		merlons.multimesh = mm
		merlons.material_override = stone
		node.add_child(merlons)
	# Archères : fentes sombres sur le fût. Matière partagée entre toutes les tours (nécessaire
	# pour que `BattleSiegeBatcher` les regroupe en un seul `MultiMeshInstance3D`).
	if _slit_mat == null:
		_slit_mat = _material(Color(0.05, 0.05, 0.05))
	var slit_mat := _slit_mat
	for k in 3:
		var a := TAU * float(k) / 3.0 + 0.4
		var slit := MeshInstance3D.new()
		var slit_box := BoxMesh.new()
		slit_box.size = Vector3(0.25, 1.6, 0.3)
		slit.mesh = slit_box
		slit.material_override = slit_mat
		slit.position = Vector3(cos(a) * r * 1.01, h * 0.55, sin(a) * r * 1.01)
		slit.rotation.y = -a + PI * 0.5
		node.add_child(slit)


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
	disc.material_override = _textured("paving", Color(0.85, 0.82, 0.78))
	disc.position = Vector3(center.x, _ground(center.x, center.y) + 0.05, center.y)
	disc.name = "Square"
	add_child(disc)
	if BuildingKit.available():
		return  # BR3 : puits et marché du cœur (`_kit_props`).
	# Puits au centre.
	var well := MeshInstance3D.new()
	var ring := CylinderMesh.new()
	ring.top_radius = 1.4
	ring.bottom_radius = 1.4
	ring.height = 1.0
	well.mesh = ring
	well.material_override = _textured("stone", STONE_DARK)
	well.position = disc.position + Vector3(0, 0.5, 0)
	add_child(well)


## Maisons et église posées sur les emprises de la simulation (`get_siege().houses`, F5c ; BR3 :
## rectangles orientés `{x, z, length, depth, yaw, rows, church}`) : le rendu coïncide avec les
## obstacles du cheminement et des figurines. Les maisons (statiques) sont ensuite regroupées en
## `MultiMeshInstance3D` par `BattleSiegeBatcher` (V6, perf) ; l'église (unique) reste un nœud normal.
func _build_houses() -> void:
	var center: Vector2 = siege.get("center", Vector2(600, 560))
	house_sites = _house_sites()
	var church_index := -1
	for i in house_sites.size():
		if bool(house_sites[i]["church"]):
			church_index = i
	var houses_root := Node3D.new()
	houses_root.name = "Houses"
	add_child(houses_root)
	if BuildingKit.available():
		_build_kit_town(houses_root, center)
		return
	for i in house_sites.size():
		var site: Dictionary = house_sites[i]
		if i != church_index:
			var h := 4.0 + 3.0 * BuildingKit.hash01(i, 11)
			_house(houses_root, site["p"], float(site["length"]) * 0.95, float(site["depth"]) * 0.95, h, -float(site["yaw"]))
	BattleSiegeBatcher.batch_and_replace(houses_root)
	if church_index < 0:
		return
	# L'église, au fond de la ville.
	var church: Vector2 = house_sites[church_index]["p"]
	if _place_model("cathedral", church, 20.0):
		return
	_house(self, church, 18.0, 10.0, 11.0, -float(house_sites[church_index]["yaw"]))
	var spire := MeshInstance3D.new()
	var cone := CylinderMesh.new()
	cone.top_radius = 0.0
	cone.bottom_radius = 3.2
	cone.height = 14.0
	cone.radial_segments = 8
	spire.mesh = cone
	spire.material_override = _textured("slate", Color(0.62, 0.64, 0.7), 0.7)
	spire.position = Vector3(church.x - 8.0, _ground(church.x, church.y) + 11.0 + 7.0 + 4.0, church.y)
	add_child(spire)


## BR1 + BR3 : ville du kit Blender, posée sur les emprises du cœur. Un îlot de deux rangées
## (`rows` = 2) reçoit deux rangées dos à dos de maisons de ville mitoyennes (pignon sur rue, 2-3
## étages en encorbellement, boutiques), façades vers l'avant (rangée 0, vers la place) et vers
## l'arrière ; un îlot d'une rangée (le long du rempart ou d'une rue) une seule rangée tournée
## vers l'avant ; un faubourg une maison rurale ; l'église la grande église du kit. Le mobilier
## (étals, charrettes, tonneaux, bûches, puits, marché) vient aussi du cœur (`props`) : il est
## solide pour les figurines, le rendu ne fait que le poser.
func _build_kit_town(houses_root: Node3D, center: Vector2) -> void:
	_kit_batch = BuildingKit.Batch.new("", 2000.0)
	_kit_sites.clear()
	_kit_ruins.clear()
	var rng := RandomNumberGenerator.new()
	rng.seed = 1340
	for i in house_sites.size():
		var site: Dictionary = house_sites[i]
		var p: Vector2 = site["p"]
		var length := float(site["length"])
		var depth := float(site["depth"])
		var yaw := float(site["yaw"])
		var handles: Array = []
		if bool(site["church"]):
			handles.append(_kit_place("church", p, length, depth, yaw, rng))
		elif bool(site["suburb"]):
			var kind: String = ["cottage", "timber", "longere", "barn", "cottage"][rng.randi_range(0, 4)]
			handles.append(_kit_place(kind, p, length, depth, yaw, rng))
		else:
			var tangent := Vector2(cos(yaw), sin(yaw))
			var front := Vector2(-sin(yaw), cos(yaw))
			var rows := int(site["rows"])
			var row_depth := depth / float(rows)
			for row in rows:
				var facing := front if row == 0 else -front
				var row_yaw := yaw if row == 0 else yaw + PI
				var row_tangent := tangent if row == 0 else -tangent
				var row_center := p + facing * (row_depth * 0.5 if rows == 2 else 0.0)
				var n := clampi(int(round(length / 6.5)), 2, 5)
				var widths: Array = []
				var total := 0.0
				for k in n:
					var w := rng.randf_range(0.8, 1.25)
					widths.append(w)
					total += w
				var x := -length * 0.5
				for k in n:
					var w: float = length * float(widths[k]) / total
					var kind := "townhouse"
					if n <= 3 and rng.randf() < 0.3:
						kind = ["stonehouse", "timber"][rng.randi_range(0, 1)]
					# Façade alignée sur l'emprise, arrière parfois moins profond (cours).
					var d := row_depth * (rng.randf_range(0.85, 1.0) if kind == "townhouse" else 0.9)
					var q := row_center + row_tangent * (x + w * 0.5) - facing * (row_depth - d) * 0.5
					handles.append(_kit_place(kind, q, w, d, row_yaw, rng))
					x += w
		_kit_sites[i] = handles.filter(func(h: Array) -> bool: return not h.is_empty())
	_kit_props(center)
	_kit_batch.build(houses_root)


## BR3 : mobilier du cœur (`get_siege().props`) : façades des îlots et place du marché (puits,
## étals en couronne). Le mobilier d'une maison est rangé avec elle (`ruin_site` le cache quand
## elle brûle) ; celui de la place est posé sur le dallage.
func _kit_props(center: Vector2) -> void:
	var paving := _ground(center.x, center.y) + 0.2
	var props: Array = siege.get("props", [])
	for k in props.size():
		var prop: Dictionary = props[k]
		var model := BuildingKit.prop_model(str(prop["kind"]), k)
		if model == "":
			continue
		var house := int(prop["house"])
		var x := float(prop["x"])
		var z := float(prop["z"])
		var y := paving - (0.1 if str(prop["kind"]) == "well" else 0.0) if house < 0 else _ground(x, z) - 0.03
		var xform := BuildingKit.prop_transform(prop, y)
		var handle := _kit_batch.add(model, xform)
		if house >= 0:
			if not _kit_sites.has(house):
				_kit_sites[house] = []
			(_kit_sites[house] as Array).append([handle, "", xform])


## Pose un bâtiment du kit (emprise `length` le long de son axe X, façade vers +Z, lacet `yaw`).
## Renvoie [poignée, modèle de ruine, transformation] ou [] si aucun modèle.
func _kit_place(kind: String, p: Vector2, length: float, width: float, yaw: float, rng: RandomNumberGenerator) -> Array:
	var model := BuildingKit.pick(kind, length, width, rng)
	if model == "":
		return []
	var low := INF
	var high := -INF
	for corner in [Vector2(-length, -width), Vector2(length, -width), Vector2(length, width), Vector2(-length, width)]:
		var q: Vector2 = p + (corner * 0.5).rotated(yaw)
		low = minf(low, _ground(q.x, q.y))
		high = maxf(high, _ground(q.x, q.y))
	var basis := Basis(Vector3.UP, -yaw) * Basis.from_scale(BuildingKit.fit_scale(model, length, width))
	var xform := Transform3D(basis, Vector3(p.x, minf(high, low + 1.4) - 0.05, p.y))
	var ruin := BuildingKit.pick(kind, length, width, rng, true)
	return [_kit_batch.add(model, xform), ruin, xform]


## S2 + BR1 : la maison `index` a brûlé → ses bâtiments du kit cèdent la place à leurs ruines
## calcinées (murs éventrés, charpente effondrée). `false` hors kit (repli : affaissement).
func ruin_site(index: int) -> bool:
	if _kit_batch == null or not _kit_sites.has(index):
		return false
	if _kit_ruins.has(index):
		return true
	_kit_ruins[index] = true
	var houses_root := get_node_or_null("Houses") as Node3D
	for entry in _kit_sites[index]:
		_kit_batch.hide(entry[0])
		var ruin := str(entry[1])
		var mesh := BuildingKit.mesh(ruin) if ruin != "" else null
		if mesh == null or houses_root == null:
			continue
		var instance := MeshInstance3D.new()
		instance.name = "Ruin%d" % index
		instance.mesh = mesh
		instance.transform = entry[2]
		houses_root.add_child(instance)
	return true


## Modèle Blender de M10 (`assets/models/<name>.glb`) mis à l'échelle (plus grande dimension
## horizontale ≈ `size` m) ; `false` s'il manque (repli procédural).
func _place_model(model_name: String, p: Vector2, size: float) -> bool:
	if not ModelLibrary.has_model(model_name):
		return false
	var model := ModelLibrary.instantiate(model_name)
	if model == null:
		return false
	var box := AABB()
	var first := true
	for child in model.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := child as MeshInstance3D
		if mesh_instance.mesh == null:
			continue
		var local := _relative_transform(mesh_instance, model) * mesh_instance.mesh.get_aabb()
		box = local if first else box.merge(local)
		first = false
	var extent := maxf(box.size.x, box.size.z)
	if first or extent <= 0.0:
		model.queue_free()
		return false
	model.scale = Vector3.ONE * (size / extent)
	model.position = Vector3(p.x, _ground(p.x, p.y) - 0.3, p.y)
	add_child(model)
	return true


static func _relative_transform(node: Node3D, root: Node3D) -> Transform3D:
	var t := Transform3D.IDENTITY
	var current: Node = node
	while current != null and current != root:
		if current is Node3D:
			t = (current as Node3D).transform * t
		current = current.get_parent()
	return t


func _ring() -> PackedVector2Array:
	var ring := PackedVector2Array()
	for piece in siege.get("pieces", []):
		ring.append(piece["a"])
	return ring


func _house(parent: Node3D, p: Vector2, w: float, d: float, h: float, angle: float) -> void:
	var node := Node3D.new()
	# Posée sur le point le plus bas de son emprise ; le soubassement comble la pente.
	var low := INF
	for corner in [Vector2(-w, -d), Vector2(w, -d), Vector2(w, d), Vector2(-w, d)]:
		var q: Vector2 = p + (corner * 0.5).rotated(-angle)
		low = minf(low, _ground(q.x, q.y))
	node.position = Vector3(p.x, low, p.y)
	node.rotation.y = angle
	parent.add_child(node)
	var rng := RandomNumberGenerator.new()
	rng.seed = int(p.x * 31.0 + p.y * 17.0)
	var plinth := MeshInstance3D.new()
	var plinth_box := BoxMesh.new()
	plinth_box.size = Vector3(w + 0.3, 3.2, d + 0.3)
	plinth.mesh = plinth_box
	plinth.material_override = _house_mats()["stone"]
	plinth.position = Vector3(0, -0.9, 0)
	node.add_child(plinth)
	var body := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = Vector3(w, h, d)
	body.mesh = box
	body.material_override = _house_mats()["plaster%d" % rng.randi_range(0, 2)]
	body.position = Vector3(0, 0.7 + h * 0.5, 0)
	node.add_child(body)
	# Colombages : poutres sombres sur les pignons et les longs pans.
	var beam_mat: StandardMaterial3D = _house_mats()["beam"]
	for side in [-1.0, 1.0]:
		for k in 3:
			var post := MeshInstance3D.new()
			var post_box := BoxMesh.new()
			post_box.size = Vector3(0.22, h, 0.12)
			post.mesh = post_box
			post.material_override = beam_mat
			post.position = Vector3((float(k) - 1.0) * w * 0.45, 0.7 + h * 0.5, side * (d * 0.5 + 0.04))
			node.add_child(post)
		var rail := MeshInstance3D.new()
		var rail_box := BoxMesh.new()
		rail_box.size = Vector3(w, 0.22, 0.12)
		rail.mesh = rail_box
		rail.material_override = beam_mat
		rail.position = Vector3(0, 0.7 + h * 0.55, side * (d * 0.5 + 0.04))
		node.add_child(rail)
	var door := MeshInstance3D.new()
	var door_box := BoxMesh.new()
	door_box.size = Vector3(1.1, 2.0, 0.15)
	door.mesh = door_box
	door.material_override = _house_mats()["door"]
	door.position = Vector3(rng.randf_range(-w * 0.25, w * 0.25), 0.7 + 1.0, d * 0.5 + 0.06)
	node.add_child(door)
	var roof := MeshInstance3D.new()
	var prism := PrismMesh.new()
	var pitch := h * rng.randf_range(0.75, 0.95)
	prism.size = Vector3(d + 0.9, pitch, w + 0.7)
	roof.mesh = prism
	var roof_kind: String = ["tiles", "tiles", "slate", "thatch"][rng.randi_range(0, 3)]
	roof.material_override = _house_mats()[roof_kind]
	roof.position = Vector3(0, 0.7 + h + pitch * 0.5, 0)
	roof.rotation.y = PI * 0.5
	node.add_child(roof)
	if rng.randf() < 0.6:
		var chimney := MeshInstance3D.new()
		var chimney_box := BoxMesh.new()
		chimney_box.size = Vector3(0.7, pitch + 1.2, 0.7)
		chimney.mesh = chimney_box
		chimney.material_override = _house_mats()["stone"]
		chimney.position = Vector3(w * 0.3, 0.7 + h + pitch * 0.5 + 0.4, d * 0.15)
		node.add_child(chimney)


var _house_cache: Dictionary = {}


## Matières partagées des maisons (créées une fois par ville).
func _house_mats() -> Dictionary:
	if _house_cache.is_empty():
		_house_cache = {
			"stone": _textured("stone", Color(0.85, 0.8, 0.72)),
			"plaster0": _textured("plaster", Color(0.93, 0.88, 0.78)),
			"plaster1": _textured("plaster", Color(0.88, 0.82, 0.7)),
			"plaster2": _textured("plaster", Color(0.95, 0.93, 0.88)),
			"beam": _textured("wood", Color(0.38, 0.28, 0.2)),
			"door": _textured("wood", Color(0.45, 0.33, 0.24)),
			"tiles": _textured("tiles", Color(0.62, 0.45, 0.38), 0.85),
			"slate": _textured("slate", Color(0.55, 0.56, 0.6), 0.75),
			"thatch": _textured("thatch", Color(0.85, 0.75, 0.6), 1.0),
		}
	return _house_cache


## Dégâts des pans et machines/échelles des assiégeants (chaque image).
func update(p_siege: Dictionary, units: Array) -> void:
	if p_siege.is_empty():
		return
	var pieces: Array = p_siege.get("pieces", [])
	for i in mini(pieces.size(), _pieces.size()):
		var piece: Dictionary = pieces[i]
		var view: Dictionary = _pieces[i]
		var ratio := clampf(float(piece["hp"]) / maxf(float(piece["max_hp"]), 1.0), 0.0, 1.0)
		var intact := bool(piece["intact"])
		# Un changement d'état (brèche, porte tombée) passe même sous le seuil
		# de 0,01 : des dégâts continus (porte en feu) le franchiraient sinon.
		if absf(ratio - float(view["ratio"])) < 0.01 and intact == bool(view.get("intact", true)):
			continue
		view["ratio"] = ratio
		view["intact"] = intact
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
		_fx.sync_piece(i, view, ratio, intact)
	_fx.prime()
	if fire_fx != null:
		fire_fx.update(p_siege)
	_update_machines(units)


func _update_machines(units: Array) -> void:
	var seen := {}
	for unit in units:
		var id := int(unit["id"])
		var render := str(unit.get("render", ""))
		var present: bool = unit["present"]
		if render == "tower" or render == "ram":
			if not _machines.has(id):
				# SG2 : modèles Blender animés (`SiegeEnginesFx`), repli procédural.
				var model := _model_machine(render)
				_machines[id] = model if model != null else (_make_tower() if render == "tower" else _make_ram())
				add_child(_machines[id])
			var machine: Node3D = _machines[id]
			machine.visible = present
			if present:
				var x := float(unit["x"])
				var z := float(unit["z"])
				machine.position = Vector3(x, _ground(x, z), z)
				machine.rotation.y = float(unit["facing"])
		if bool(unit.get("ladders", false)) and present and not external_ladders:
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


## SG2 : bélier ou beffroi modélisé (`game/assets/models/siege/`) ; null si absent. La caisse du
## beffroi est mise à la hauteur du mur, son pont-levis juste au-dessus du chemin de ronde.
func _model_machine(render: String) -> Node3D:
	var node := SiegeEnginesFx.instantiate("siege_tower" if render == "tower" else "ram")
	if node == null or render != "tower":
		return node
	var c: Dictionary = SiegeEnginesFx.settings().get("siege_tower", {})
	var body := node.find_child("Body", true, false) as Node3D
	var pivot := node.find_child("BridgePivot", true, false) as Node3D
	if body != null:
		body.scale.y = (wall_height + 4.0) / float(c.get("model_height", 12.8))
	if pivot != null:
		pivot.position.y = wall_height + float(c.get("bridge_above_wall", 0.8))
		pivot.rotation.x = -PI * 0.5
	return node


## Tour de siège (beffroi) : caisse de bois sur roues, peaux, pont-levis ; face avant vers +Z local.
func _make_tower() -> Node3D:
	var node := Node3D.new()
	var h := wall_height + 4.0
	var body := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = Vector3(5.0, h, 5.0)
	body.mesh = box
	body.material_override = _textured("wood")
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
	bridge.material_override = _textured("wood", Color(0.6, 0.5, 0.42))
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
	roof.material_override = _textured("thatch", Color(0.8, 0.72, 0.6))
	roof.position = Vector3(0, 2.3, 0)
	node.add_child(roof)
	var beam := MeshInstance3D.new()
	var cylinder := CylinderMesh.new()
	cylinder.top_radius = 0.35
	cylinder.bottom_radius = 0.35
	cylinder.height = 9.0
	beam.mesh = cylinder
	beam.material_override = _textured("wood", Color(0.6, 0.5, 0.42))
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
	for x in [-0.8, 0.8]:
		BattleMeshes.add_box(st, Vector3(x, length * 0.5, 0), Vector3(0.4, length, 0.4), LADDER)
	var rungs := int(length / 0.7)
	for i in rungs:
		BattleMeshes.add_box(st, Vector3(0, 0.3 + i * 0.7, 0), Vector3(1.6, 0.25, 0.25), LADDER)
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


## Source des maisons (F5c, BR3) : emprises `{x, z, radius, length, depth, yaw, rows, church}` de
## `get_siege().houses`, obstacles de la simulation. Aucune position n'est inventée ici.
func _house_sites() -> Array:
	var sites: Array = []
	for house in siege.get("houses", []):
		var r := float(house["radius"])
		sites.append({
			"p": Vector2(float(house["x"]), float(house["z"])),
			"radius": r,
			"suburb": bool(house.get("suburb", false)),
			"length": float(house.get("length", r * 1.75)),
			"depth": float(house.get("depth", r * 1.85)),
			"yaw": float(house.get("yaw", 0.0)),
			"rows": int(house.get("rows", 2)),
			"church": bool(house.get("church", false)),
		})
	return sites
