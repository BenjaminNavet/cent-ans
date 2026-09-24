class_name BridgeMeshes
extends RefCounted

## Maillages procéduraux des franchissements de la carte de campagne (lot V4, A1-11) : pont de
## pierre à arches (avec avant-becs), pont de bois sur pilotis, pont de bateaux, bac (barge et
## pontons), gué (pierres et perches de balisage), pont-porte des murailles (pierre crénelée).
##
## Repère local : X en travers du fleuve (d'une berge à l'autre), Z dans le sens du courant,
## Y vers le haut, origine sur la ligne médiane au niveau de l'eau (hauteur du relief non creusé).
## Deux surfaces par maillage : 0 = pierre, 1 = bois. Rendu seulement.

const STONE := 0
const WOOD := 1
## Fond des piles et culées, sous le lit creusé le plus profond.
const FOOT := -0.42

static var _materials: Array[StandardMaterial3D] = []


## Matériaux partagés : pierre (texture de muraille) et bois (planches), en triplanaire monde.
static func materials() -> Array[StandardMaterial3D]:
	if _materials.is_empty():
		var stone := StandardMaterial3D.new()
		stone.albedo_texture = load("res://assets/textures/battle/castle_wall_varriation_diff.jpg")
		stone.normal_enabled = true
		stone.normal_texture = load("res://assets/textures/battle/castle_wall_varriation_nor.jpg")
		stone.albedo_color = Color(0.92, 0.86, 0.76)
		stone.roughness = 0.92
		stone.uv1_triplanar = true
		stone.uv1_world_triplanar = true
		stone.uv1_scale = Vector3.ONE * 3.0
		var wood := StandardMaterial3D.new()
		wood.albedo_texture = load("res://assets/textures/battle/wood_planks_diff.jpg")
		wood.normal_enabled = true
		wood.normal_texture = load("res://assets/textures/battle/wood_planks_nor.jpg")
		wood.albedo_color = Color(0.62, 0.50, 0.40)
		wood.roughness = 0.85
		wood.uv1_triplanar = true
		wood.uv1_world_triplanar = true
		wood.uv1_scale = Vector3.ONE * 6.0
		_materials = [stone, wood]
	return _materials


## Maillage d'un franchissement. `structure` : stone, wood, boats, ferry, ford, gate ; `width` :
## largeur du fleuve (unités monde) ; `seed_value` : variations déterministes.
static func build(structure: String, width: float, seed_value: int) -> ArrayMesh:
	var tools: Array[SurfaceTool] = [SurfaceTool.new(), SurfaceTool.new()]
	for st in tools:
		st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value
	match structure:
		"wood":
			_wood_bridge(tools, width, rng)
		"boats":
			_boat_bridge(tools, width, rng)
		"ferry":
			_ferry(tools, width, rng)
		"ford":
			_ford(tools, width, rng)
		"gate":
			_stone_bridge(tools, width, rng, true)
		_:
			_stone_bridge(tools, width, rng, false)
	var mesh := ArrayMesh.new()
	var mats := materials()
	for kind in 2:
		var arrays := tools[kind].commit_to_arrays()
		var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		if vertices.is_empty():
			continue
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
		mesh.surface_set_material(mesh.get_surface_count() - 1, mats[kind])
	return mesh


## Longueur totale (culées comprises) pour une largeur de fleuve.
static func span_length(width: float) -> float:
	return width + 0.5 + 0.25 * width


static func deck_width(width: float) -> float:
	return 0.15 + 0.05 * width


# --- Formes de base -------------------------------------------------------------------------


static func _quad(st: SurfaceTool, a: Vector3, b: Vector3, c: Vector3, d: Vector3) -> void:
	var n := (b - a).cross(c - a).normalized()
	st.set_normal(n)
	for v in [a, b, c, a, c, d]:
		st.add_vertex(v)


## Boîte centrée en `center`, demi-tailles `half`, tournée de `yaw` autour de Y.
static func _box(st: SurfaceTool, center: Vector3, half: Vector3, yaw: float = 0.0) -> void:
	var basis := Basis(Vector3.UP, yaw)
	var c := func(sx: float, sy: float, sz: float) -> Vector3:
		return center + basis * Vector3(sx * half.x, sy * half.y, sz * half.z)
	var p000: Vector3 = c.call(-1, -1, -1)
	var p100: Vector3 = c.call(1, -1, -1)
	var p010: Vector3 = c.call(-1, 1, -1)
	var p110: Vector3 = c.call(1, 1, -1)
	var p001: Vector3 = c.call(-1, -1, 1)
	var p101: Vector3 = c.call(1, -1, 1)
	var p011: Vector3 = c.call(-1, 1, 1)
	var p111: Vector3 = c.call(1, 1, 1)
	_quad(st, p010, p110, p111, p011)  # dessus
	_quad(st, p001, p101, p100, p000)  # dessous
	_quad(st, p001, p011, p111, p101)  # +Z
	_quad(st, p100, p110, p010, p000)  # −Z
	_quad(st, p101, p111, p110, p100)  # +X
	_quad(st, p000, p010, p011, p001)  # −X


## Extrusion le long de Z (de −half_z à +half_z) d'un contour simple dans le plan XY.
static func _extrude(st: SurfaceTool, outline: PackedVector2Array, half_z: float) -> void:
	var triangles := Geometry2D.triangulate_polygon(outline)
	if triangles.is_empty():
		return
	var clockwise := Geometry2D.is_polygon_clockwise(outline)
	for side in [1.0, -1.0]:
		st.set_normal(Vector3(0, 0, side))
		for k in range(0, triangles.size(), 3):
			var ids := [triangles[k], triangles[k + 1], triangles[k + 2]]
			# Faces avant (+Z) dans le sens trigonométrique vu de +Z.
			var a := outline[ids[0]]
			var b := outline[ids[1]]
			var c := outline[ids[2]]
			var ccw := (b - a).cross(c - a) > 0.0
			if ccw != (side > 0.0):
				var tmp := b
				b = c
				c = tmp
			for p: Vector2 in [a, b, c]:
				st.add_vertex(Vector3(p.x, p.y, side * half_z))
	var count := outline.size()
	for i in count:
		var a := outline[i]
		var b := outline[(i + 1) % count]
		var edge := b - a
		if edge.length_squared() < 1e-10:
			continue
		# Normale extérieure : à droite du bord pour un contour trigonométrique.
		var out := Vector2(edge.y, -edge.x).normalized()
		if clockwise:
			out = -out
		var va := Vector3(a.x, a.y, half_z)
		var vb := Vector3(b.x, b.y, half_z)
		var vc := Vector3(b.x, b.y, -half_z)
		var vd := Vector3(a.x, a.y, -half_z)
		st.set_normal(Vector3(out.x, out.y, 0.0))
		var face_n := (vb - va).cross(vc - va)
		if face_n.dot(Vector3(out.x, out.y, 0.0)) > 0.0:
			for v in [va, vb, vc, va, vc, vd]:
				st.add_vertex(v)
		else:
			for v in [va, vc, vb, va, vd, vc]:
				st.add_vertex(v)


# --- Ponts ----------------------------------------------------------------------------------


## Pont de pierre : contour en élévation (tablier, arches en arc brisé, piles) extrudé, avant-becs
## triangulaires en amont et en aval, parapets ; `gate` : parapets plus hauts et crénelés
## (continuité de la muraille).
static func _stone_bridge(tools: Array[SurfaceTool], width: float, rng: RandomNumberGenerator, gate: bool) -> void:
	var st := tools[STONE]
	var length := span_length(width)
	var half_w := deck_width(width) * (1.35 if gate else 1.0) * 0.5
	var deck_top := 0.05 + 0.02 * width
	var span := width * 1.02 + 0.06
	var arches := clampi(int(round(span / 0.26)), 1, 13)
	var pier := clampf(span * 0.16 / arches, 0.025, 0.07)
	var opening := (span - pier * (arches - 1)) / arches
	var spring := -0.035
	var crown := deck_top - 0.022
	var outline := PackedVector2Array()
	outline.append(Vector2(-length * 0.5, deck_top))
	outline.append(Vector2(0.0, deck_top + 0.012))  # dos d'âne léger
	outline.append(Vector2(length * 0.5, deck_top))
	outline.append(Vector2(length * 0.5, FOOT))
	var x := span * 0.5
	outline.append(Vector2(x, FOOT))
	for i in arches:
		var right := x
		var left := x - opening
		outline.append(Vector2(right, spring))
		# Arc brisé : deux quarts d'ellipse qui se rejoignent à la clé.
		var segments := 6
		for s in range(1, segments):
			var t := float(s) / segments
			var angle := t * PI
			var px := lerpf(right, left, 0.5 - 0.5 * cos(angle))
			var rise := sin(angle)
			rise = pow(rise, 0.8)
			outline.append(Vector2(px, lerpf(spring, crown, rise)))
		outline.append(Vector2(left, spring))
		outline.append(Vector2(left, FOOT))
		x = left - pier
		if i < arches - 1:
			outline.append(Vector2(x, FOOT))
	outline.append(Vector2(-length * 0.5, FOOT))
	_extrude(st, outline, half_w)
	# Avant-becs (amont et aval) au pied de chaque pile.
	x = span * 0.5 - opening
	for i in arches - 1:
		var cx := x - pier * 0.5
		for side in [1.0, -1.0]:
			var tip := Vector3(cx, 0.0, side * (half_w + pier * 1.3))
			var a := Vector3(cx - pier * 0.5, 0.0, side * half_w)
			var b := Vector3(cx + pier * 0.5, 0.0, side * half_w)
			var top := spring + 0.03
			_quad(st, Vector3(a.x, FOOT, a.z), Vector3(a.x, top, a.z), Vector3(tip.x, top, tip.z), Vector3(tip.x, FOOT, tip.z))
			_quad(st, Vector3(tip.x, FOOT, tip.z), Vector3(tip.x, top, tip.z), Vector3(b.x, top, b.z), Vector3(b.x, FOOT, b.z))
			st.set_normal(Vector3.UP)
			for v in [Vector3(a.x, top, a.z), Vector3(b.x, top, b.z), Vector3(tip.x, top, tip.z)]:
				st.add_vertex(v)
		x -= opening + pier
	# Parapets.
	var parapet_h := 0.05 if gate else 0.022
	var parapet_t := 0.012 if not gate else 0.02
	for side in [1.0, -1.0]:
		var z: float = side * (half_w - parapet_t)
		_box(st, Vector3(0.0, deck_top + parapet_h * 0.5, z), Vector3(length * 0.5, parapet_h * 0.5, parapet_t))
		if gate:
			var merlons := int(length / 0.07)
			for m in merlons:
				var mx := -length * 0.5 + (m + 0.5) * length / merlons
				if m % 2 == 0:
					_box(st, Vector3(mx, deck_top + parapet_h + 0.012, z), Vector3(length * 0.25 / merlons, 0.012, parapet_t))
	if gate:
		# Tours carrées aux deux extrémités (entrée de l'eau sous la muraille).
		for end in [1.0, -1.0]:
			_box(st, Vector3(end * (length * 0.5 - 0.06), deck_top * 0.5 + 0.06, 0.0), Vector3(0.07, deck_top * 0.5 + 0.12, half_w + 0.03))
	var _unused := rng.randf()


## Pont de bois : tablier de planches sur des palées de pieux, garde-corps.
static func _wood_bridge(tools: Array[SurfaceTool], width: float, rng: RandomNumberGenerator) -> void:
	var st := tools[WOOD]
	var length := span_length(width)
	var half_w := deck_width(width) * 0.45
	var deck_top := 0.045 + 0.015 * width
	_box(st, Vector3(0.0, deck_top - 0.007, 0.0), Vector3(length * 0.5, 0.007, half_w))
	# Culées de pierre aux deux bouts.
	for end in [1.0, -1.0]:
		_box(tools[STONE], Vector3(end * (length * 0.5 - 0.1), (deck_top - 0.014 + FOOT) * 0.5, 0.0), Vector3(0.1, (deck_top - 0.014 - FOOT) * 0.5, half_w * 1.1))
	var bents := maxi(2, int(width / 0.13))
	for b in bents:
		var bx := -width * 0.5 + (b + 0.5) * width / bents + rng.randf_range(-0.01, 0.01)
		for pz in [-0.7, 0.0, 0.7]:
			_box(st, Vector3(bx, (deck_top - 0.014 + FOOT) * 0.5, pz * half_w), Vector3(0.007, (deck_top - 0.014 - FOOT) * 0.5, 0.007))
		_box(st, Vector3(bx, deck_top - 0.018, 0.0), Vector3(0.008, 0.005, half_w * 1.05))
	for side in [1.0, -1.0]:
		var z: float = side * (half_w - 0.004)
		_box(st, Vector3(0.0, deck_top + 0.018, z), Vector3(length * 0.5, 0.003, 0.003))
		var posts := int(length / 0.09)
		for p in posts + 1:
			var px := -length * 0.5 + p * length / maxf(posts, 1)
			_box(st, Vector3(px, deck_top + 0.009, z), Vector3(0.003, 0.009, 0.003))


## Pont de bateaux : barques amarrées bord à bord sous un tablier de planches.
static func _boat_bridge(tools: Array[SurfaceTool], width: float, rng: RandomNumberGenerator) -> void:
	var st := tools[WOOD]
	var length := span_length(width)
	var half_w := deck_width(width) * 0.45
	var deck_top := 0.02
	_box(st, Vector3(0.0, deck_top - 0.005, 0.0), Vector3(length * 0.5, 0.005, half_w))
	var boats := maxi(3, int(width / 0.075))
	for b in boats:
		var bx := -width * 0.5 + (b + 0.5) * width / boats
		_hull(st, Vector3(bx, -0.012, rng.randf_range(-0.01, 0.01)), 0.026, half_w * 1.6, 0.02)
	for side in [1.0, -1.0]:
		_box(st, Vector3(0.0, deck_top + 0.012, side * (half_w - 0.003)), Vector3(length * 0.5, 0.0025, 0.0025))
	for end in [1.0, -1.0]:
		_box(tools[STONE], Vector3(end * (length * 0.5 - 0.08), (deck_top + FOOT) * 0.5, 0.0), Vector3(0.08, (deck_top - FOOT) * 0.5, half_w * 1.1))


## Coque de barque : prisme hexagonal allongé selon Z (pointes aux deux bouts).
static func _hull(st: SurfaceTool, center: Vector3, half_x: float, half_z: float, height: float) -> void:
	var top := center.y + height * 0.5
	var bottom := center.y - height * 0.5
	var ring_top := [Vector3(0, top, half_z), Vector3(half_x, top, half_z * 0.6), Vector3(half_x, top, -half_z * 0.6),
		Vector3(0, top, -half_z), Vector3(-half_x, top, -half_z * 0.6), Vector3(-half_x, top, half_z * 0.6)]
	var ring_bottom := [Vector3(0, bottom, half_z * 0.8), Vector3(half_x * 0.6, bottom, half_z * 0.5), Vector3(half_x * 0.6, bottom, -half_z * 0.5),
		Vector3(0, bottom, -half_z * 0.8), Vector3(-half_x * 0.6, bottom, -half_z * 0.5), Vector3(-half_x * 0.6, bottom, half_z * 0.5)]
	var offset := Vector3(center.x, 0.0, center.z)
	for i in 6:
		var a: Vector3 = ring_top[i] + offset
		var b: Vector3 = ring_top[(i + 1) % 6] + offset
		var c: Vector3 = ring_bottom[(i + 1) % 6] + offset
		var d: Vector3 = ring_bottom[i] + offset
		_quad(st, a, d, c, b)
	st.set_normal(Vector3.UP)
	for i in range(1, 5):
		for v in [ring_top[0] + offset, ring_top[i + 1] + offset, ring_top[i] + offset]:
			st.add_vertex(v)


## Bac : barge à fond plat au bord d'une rive, câble tendu, pontons sur pilotis des deux côtés.
static func _ferry(tools: Array[SurfaceTool], width: float, rng: RandomNumberGenerator) -> void:
	var st := tools[WOOD]
	var side := 1.0 if rng.randf() < 0.5 else -1.0
	var barge_x := side * width * rng.randf_range(0.18, 0.32)
	_box(st, Vector3(barge_x, 0.004, 0.0), Vector3(0.09, 0.012, 0.05))
	_box(st, Vector3(barge_x, 0.02, 0.0), Vector3(0.09, 0.004, 0.003))
	_box(st, Vector3(0.0, 0.035, 0.0), Vector3(width * 0.5 + 0.12, 0.0015, 0.0015))
	for end in [1.0, -1.0]:
		var jx: float = end * (width * 0.5 + 0.02)
		_box(st, Vector3(jx, 0.012, 0.0), Vector3(0.07, 0.004, 0.035))
		for p in [-1.0, 1.0]:
			_box(st, Vector3(jx - end * 0.05, (0.012 + FOOT) * 0.5, p * 0.028), Vector3(0.004, (0.012 - FOOT) * 0.5, 0.004))
		_box(st, Vector3(end * (width * 0.5 + 0.12), 0.03, 0.0), Vector3(0.004, 0.04, 0.004))


## Gué : pierres plates en travers du courant et perches de balisage sur les deux rives.
static func _ford(tools: Array[SurfaceTool], width: float, rng: RandomNumberGenerator) -> void:
	var stones := maxi(4, int(width / 0.07))
	for s in stones:
		var sx := -width * 0.5 + (s + 0.5) * width / stones
		var size := rng.randf_range(0.014, 0.024)
		_box(tools[STONE], Vector3(sx, -0.012, rng.randf_range(-0.03, 0.03)), Vector3(size, 0.012, size * rng.randf_range(0.7, 1.2)), rng.randf() * TAU)
	for end in [1.0, -1.0]:
		for p in [-1.0, 1.0]:
			_box(tools[WOOD], Vector3(end * (width * 0.5 + 0.05), 0.02, p * 0.05), Vector3(0.004, 0.05, 0.004))
