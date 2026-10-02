class_name WarScarMeshes
extends RefCounted

## Lot TB4 : maillages procéduraux des traces de guerre et de peste de la carte de campagne
## (`WarScars`). Boîtes et calottes aux couleurs par sommet, un seul matériau mat partagé ; aucune
## texture, aucun fichier. Unités : mètres pour la peste (fosse, porte marquée), unités du repère
## des figurines d'armée pour le champ de bataille et les engins de siège.

const EARTH := Color(0.27, 0.2, 0.14)
const EARTH_DARK := Color(0.12, 0.09, 0.07)
const LIME := Color(0.78, 0.76, 0.7)
const SHROUD := Color(0.7, 0.67, 0.6)
const WOOD := Color(0.33, 0.24, 0.15)
const WOOD_DARK := Color(0.2, 0.14, 0.09)
const WOOD_FRESH := Color(0.62, 0.48, 0.3)
const IRON := Color(0.3, 0.3, 0.32)
const CROW := Color(0.03, 0.03, 0.035)
const SHIELD_COLORS := [Color(0.55, 0.1, 0.08), Color(0.12, 0.2, 0.5), Color(0.8, 0.76, 0.66), Color(0.7, 0.55, 0.15)]

static var _cache: Dictionary = {}
static var _material: StandardMaterial3D = null


static func clear_cache() -> void:
	_cache.clear()
	_material = null


## Matériau commun : couleur par sommet, mat, deux faces (ailes des corbeaux, planches fines).
static func material() -> StandardMaterial3D:
	if _material == null:
		_material = StandardMaterial3D.new()
		_material.vertex_color_use_as_albedo = true
		_material.roughness = 1.0
		_material.cull_mode = BaseMaterial3D.CULL_DISABLED
	return _material


static func _begin() -> SurfaceTool:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	return st


static func _commit(st: SurfaceTool) -> ArrayMesh:
	var mesh := st.commit()
	if mesh.get_surface_count() > 0:
		mesh.surface_set_material(0, material())
	return mesh


## Boîte de taille `size` centrée en `center`, tournée par `basis`.
static func box(st: SurfaceTool, center: Vector3, size: Vector3, color: Color, basis: Basis = Basis.IDENTITY) -> void:
	var h := size * 0.5
	var faces := [
		[Vector3.RIGHT, Vector3.UP, Vector3.BACK],
		[Vector3.LEFT, Vector3.UP, Vector3.FORWARD],
		[Vector3.UP, Vector3.BACK, Vector3.RIGHT],
		[Vector3.DOWN, Vector3.FORWARD, Vector3.RIGHT],
		[Vector3.BACK, Vector3.UP, Vector3.LEFT],
		[Vector3.FORWARD, Vector3.UP, Vector3.RIGHT],
	]
	for face in faces:
		var n: Vector3 = face[0]
		var u: Vector3 = face[1]
		var v: Vector3 = face[2]
		var corners := [n - u - v, n + u - v, n + u + v, n - u + v]
		st.set_normal((basis * n).normalized())
		st.set_color(color)
		for k in [0, 1, 2, 0, 2, 3]:
			st.add_vertex(center + basis * ((corners[k] as Vector3) * h))


## Barre de section `thickness` entre deux points.
static func beam(st: SurfaceTool, from: Vector3, to: Vector3, thickness: float, color: Color) -> void:
	var axis := to - from
	var length := axis.length()
	if length < 1e-5:
		return
	var y := axis / length
	var x := y.cross(Vector3.FORWARD if absf(y.dot(Vector3.FORWARD)) < 0.9 else Vector3.RIGHT).normalized()
	box(st, (from + to) * 0.5, Vector3(thickness, length, thickness), color, Basis(x, y, x.cross(y)))


## Croix de bois plantée en `foot` (hauteur `height`).
static func cross(st: SurfaceTool, foot: Vector3, height: float, color: Color, yaw: float = 0.0) -> void:
	var t := height * 0.09
	var side := Vector3(cos(yaw), 0.0, sin(yaw))
	beam(st, foot, foot + Vector3.UP * height, t, color)
	var bar := foot + Vector3.UP * height * 0.7
	beam(st, bar - side * height * 0.25, bar + side * height * 0.25, t, color)


## Calotte de terre de rayon `radius` et de hauteur `height`, bosselée (`seed_value`).
static func dome(st: SurfaceTool, center: Vector3, radius: float, height: float, color: Color, seed_value: int = 0) -> void:
	var rings := 4
	var segments := 12
	var rows: Array = []
	for r in rings + 1:
		var t := float(r) / rings  # 0 au sommet, 1 au pied
		var row := PackedVector3Array()
		for s in segments:
			var a := TAU * s / segments
			var wobble := 1.0 + 0.16 * (_h(seed_value, r * 31 + s) - 0.5) * (1.0 if r > 0 else 0.0)
			var rr := radius * sin(t * PI * 0.5) * wobble
			row.append(center + Vector3(cos(a) * rr, height * cos(t * PI * 0.5), sin(a) * rr))
		rows.append(row)
	for r in rings:
		var top: PackedVector3Array = rows[r]
		var bottom: PackedVector3Array = rows[r + 1]
		for s in segments:
			var s2 := (s + 1) % segments
			var shade := 0.85 + 0.3 * _h(seed_value, 500 + r * 17 + s)
			var quad := [top[s], top[s2], bottom[s2], bottom[s]]
			var normal := ((quad[2] as Vector3) - (quad[0] as Vector3)).cross((quad[1] as Vector3) - (quad[0] as Vector3)).normalized()
			if normal.y < 0.0:
				normal = -normal
			st.set_normal(normal)
			st.set_color(Color(color.r * shade, color.g * shade, color.b * shade))
			for k in [0, 1, 2, 0, 2, 3]:
				st.add_vertex(quad[k])


static func _h(a: int, b: int) -> float:
	return float(absi(hash(Vector2i(a, b))) % 10007) / 10007.0


# --- Peste (mètres) --------------------------------------------------------------------


## Fosse commune : terre remuée en bourrelets, fond sombre, corps en linceul, chaux, croix au
## chevet. Longueur le long de +X (≈ 6,5 m), origine au centre, au sol.
static func plague_pit() -> ArrayMesh:
	if _cache.has("plague_pit"):
		return _cache["plague_pit"]
	var st := _begin()
	box(st, Vector3(0.0, 0.06, 0.0), Vector3(5.6, 0.12, 2.4), EARTH_DARK)
	dome(st, Vector3(0.0, 0.0, 1.9), 1.5, 0.75, EARTH, 11)
	dome(st, Vector3(-1.9, 0.0, 1.8), 1.3, 0.6, EARTH, 12)
	dome(st, Vector3(1.9, 0.0, 1.8), 1.3, 0.6, EARTH, 13)
	dome(st, Vector3(0.6, 0.0, -1.9), 1.4, 0.5, EARTH, 14)
	for k in 4:
		var x := -2.0 + 1.3 * k
		var skew := Basis(Vector3.UP, 0.25 * (_h(21, k) - 0.5))
		box(st, Vector3(x, 0.2, -0.15 + 0.5 * (_h(22, k) - 0.5)), Vector3(0.5, 0.26, 1.75), SHROUD, skew)
	box(st, Vector3(1.1, 0.135, 0.5), Vector3(2.2, 0.03, 0.9), LIME)
	cross(st, Vector3(-3.4, 0.0, 0.0), 1.7, WOOD, PI * 0.5)
	_cache["plague_pit"] = _commit(st)
	return _cache["plague_pit"]


## Porte condamnée : vantail de planches barré, croix peinte à la chaux. Face vers +Z, pied à
## l'origine (le bas descend sous le sol : terrain en pente).
static func marked_door() -> ArrayMesh:
	if _cache.has("marked_door"):
		return _cache["marked_door"]
	var st := _begin()
	box(st, Vector3(0.0, 0.85, 0.0), Vector3(1.15, 2.7, 0.08), WOOD_DARK)
	box(st, Vector3(0.0, 1.05, 0.06), Vector3(0.2, 1.5, 0.04), LIME)
	box(st, Vector3(0.0, 1.3, 0.06), Vector3(0.8, 0.2, 0.04), LIME)
	box(st, Vector3(0.0, 0.35, 0.07), Vector3(1.35, 0.12, 0.05), WOOD_FRESH, Basis(Vector3.BACK, 0.12))
	_cache["marked_door"] = _commit(st)
	return _cache["marked_door"]


# --- Champ de bataille (unités du repère des figurines) ---------------------------------


## Tertre des morts : terre fraîche, deux croix au sommet.
static func mound(radius: float, height: float) -> ArrayMesh:
	var key := "mound|%.3f|%.3f" % [radius, height]
	if _cache.has(key):
		return _cache[key]
	var st := _begin()
	dome(st, Vector3.ZERO, radius, height, EARTH, 3)
	cross(st, Vector3(0.0, height * 0.96, 0.0), height * 1.5, WOOD, 0.3)
	cross(st, Vector3(radius * 0.45, height * 0.72, radius * 0.2), height * 1.1, WOOD, -0.5)
	_cache[key] = _commit(st)
	return _cache[key]


## Débris de bataille, variante `variant` (0 lance fichée, 1 écu brisé, 2 flèches, 3 étendard en
## lambeaux, 4 roue de chariot), à l'échelle `unit` (≈ hauteur d'un homme).
static func debris(variant: int, unit: float) -> ArrayMesh:
	var key := "debris|%d|%.3f" % [variant, unit]
	if _cache.has(key):
		return _cache[key]
	var st := _begin()
	match variant % 5:
		0:
			beam(st, Vector3.ZERO, Vector3(0.28, 1.25, 0.1) * unit, 0.05 * unit, WOOD)
			box(st, Vector3(0.3, 1.33, 0.11) * unit, Vector3(0.07, 0.2, 0.03) * unit, IRON, Basis(Vector3.BACK, -0.22))
		1:
			var tilt := Basis(Vector3.RIGHT, -1.2) * Basis(Vector3.BACK, 0.2)
			box(st, Vector3(0.0, 0.1, 0.0) * unit, Vector3(0.55, 0.7, 0.05) * unit, SHIELD_COLORS[0], tilt)
			box(st, Vector3(0.5, 0.06, 0.3) * unit, Vector3(0.3, 0.42, 0.05) * unit, SHIELD_COLORS[2], Basis(Vector3.RIGHT, -1.45))
		2:
			for k in 4:
				var foot := Vector3(0.3 * (k - 1.5), 0.0, 0.2 * _h(31, k)) * unit
				beam(st, foot, foot + Vector3(0.12 * (_h(32, k) - 0.5), 0.62, 0.2 * (_h(33, k) - 0.5)) * unit, 0.025 * unit, WOOD_FRESH)
		3:
			beam(st, Vector3.ZERO, Vector3(-0.35, 1.5, 0.0) * unit, 0.05 * unit, WOOD)
			box(st, Vector3(-0.12, 1.2, 0.0) * unit, Vector3(0.5, 0.36, 0.02) * unit, SHIELD_COLORS[1], Basis(Vector3.BACK, 0.23))
		4:
			var lean := Basis(Vector3.RIGHT, -1.15)
			for k in 8:
				var a := TAU * k / 8.0
				var a2 := TAU * (k + 1) / 8.0
				var p1 := lean * (Vector3(cos(a), sin(a), 0.0) * 0.5 * unit) + Vector3(0.0, 0.2 * unit, 0.0)
				var p2 := lean * (Vector3(cos(a2), sin(a2), 0.0) * 0.5 * unit) + Vector3(0.0, 0.2 * unit, 0.0)
				beam(st, p1, p2, 0.06 * unit, WOOD_DARK)
				if k % 2 == 0:
					beam(st, Vector3(0.0, 0.2 * unit, 0.0), p1, 0.035 * unit, WOOD)
	_cache[key] = _commit(st)
	return _cache[key]


## Corbeau en vol (envergure `span`), ailes en V ouvert ; nez vers +Z.
static func crow(span: float) -> ArrayMesh:
	var key := "crow|%.3f" % span
	if _cache.has(key):
		return _cache[key]
	var st := _begin()
	var half := span * 0.5
	st.set_color(CROW)
	st.set_normal(Vector3.UP)
	for side in [-1.0, 1.0]:
		st.add_vertex(Vector3(0.0, 0.0, 0.18 * span))
		st.add_vertex(Vector3(side * half, 0.12 * span, -0.05 * span))
		st.add_vertex(Vector3(0.0, 0.0, -0.16 * span))
	st.add_vertex(Vector3(-0.05 * span, 0.0, 0.1 * span))
	st.add_vertex(Vector3(0.05 * span, 0.0, 0.1 * span))
	st.add_vertex(Vector3(0.0, 0.0, -0.36 * span))
	_cache[key] = _commit(st)
	return _cache[key]


# --- Engins de siège (boîte unité : plus grande dimension = 1) --------------------------


## Tas de bois d'œuvre (madriers empilés), dans une emprise `size`.
static func timber_pile(size: Vector3) -> ArrayMesh:
	var key := "timber|%s" % size
	if _cache.has(key):
		return _cache[key]
	var st := _begin()
	_timber(st, Vector3.ZERO, size)
	_cache[key] = _commit(st)
	return _cache[key]


static func _timber(st: SurfaceTool, at: Vector3, size: Vector3) -> void:
	var layers := 3
	var t := size.y / layers
	for layer in layers:
		var count := 4 - layer
		for k in count:
			var z := (float(k) - (count - 1) * 0.5) * size.z / 4.0
			box(st, at + Vector3(0.0, t * (layer + 0.5), z), Vector3(size.x, t * 0.85, size.z / 4.6), WOOD_FRESH if (k + layer) % 2 == 0 else WOOD)


## Charpente d'un engin en chantier : quatre poteaux, traverses, contreventement et tas de bois
## au pied, dans une emprise `size` (x longueur, y hauteur, z largeur).
static func engine_frame(size: Vector3) -> ArrayMesh:
	var key := "frame|%s" % size
	if _cache.has(key):
		return _cache[key]
	var st := _begin()
	var t := maxf(size.x, size.z) * 0.07
	var hx := size.x * 0.5 - t
	var hz := size.z * 0.5 - t
	for sx in [-1.0, 1.0]:
		for sz in [-1.0, 1.0]:
			beam(st, Vector3(sx * hx, 0.0, sz * hz), Vector3(sx * hx, size.y, sz * hz), t, WOOD_FRESH)
	for level in [0.12, 0.55, 1.0]:
		var y: float = size.y * level
		for sz in [-1.0, 1.0]:
			beam(st, Vector3(-hx, y, sz * hz), Vector3(hx, y, sz * hz), t * 0.8, WOOD)
		for sx in [-1.0, 1.0]:
			beam(st, Vector3(sx * hx, y, -hz), Vector3(sx * hx, y, hz), t * 0.8, WOOD)
	beam(st, Vector3(-hx, size.y * 0.12, hz), Vector3(hx, size.y * 0.55, hz), t * 0.6, WOOD_FRESH)
	beam(st, Vector3(hx, size.y * 0.12, -hz), Vector3(-hx, size.y * 0.55, -hz), t * 0.6, WOOD_FRESH)
	_timber(st, Vector3(0.0, 0.0, size.z * 0.5 + size.z * 0.35), Vector3(size.x * 0.8, size.y * 0.16, size.z * 0.5))
	_cache[key] = _commit(st)
	return _cache[key]


## Échafaudage autour d'un engin presque achevé : perches et deux rangs de lisses.
static func scaffold(size: Vector3) -> ArrayMesh:
	var key := "scaffold|%s" % size
	if _cache.has(key):
		return _cache[key]
	var st := _begin()
	var t := maxf(size.x, size.z) * 0.035
	var hx := size.x * 0.5 + t * 3.0
	var hz := size.z * 0.5 + t * 3.0
	for sx in [-1.0, 0.0, 1.0]:
		for sz in [-1.0, 1.0]:
			beam(st, Vector3(sx * hx, 0.0, sz * hz), Vector3(sx * hx, size.y * 1.05, sz * hz), t, WOOD_FRESH)
	for level in [0.45, 0.85]:
		var y: float = size.y * level
		for sz in [-1.0, 1.0]:
			beam(st, Vector3(-hx, y, sz * hz), Vector3(hx, y, sz * hz), t, WOOD)
	_cache[key] = _commit(st)
	return _cache[key]


## Échelles d'escalade : `count` échelles couchées sur deux tréteaux (longueur 1 le long de +X).
static func ladders(count: int) -> ArrayMesh:
	var key := "ladders|%d" % count
	if _cache.has(key):
		return _cache[key]
	var st := _begin()
	for sx in [-0.3, 0.3]:
		beam(st, Vector3(sx, 0.0, -0.3), Vector3(sx, 0.16, -0.3), 0.03, WOOD)
		beam(st, Vector3(sx, 0.0, 0.3), Vector3(sx, 0.16, 0.3), 0.03, WOOD)
		beam(st, Vector3(sx, 0.16, -0.34), Vector3(sx, 0.16, 0.34), 0.03, WOOD)
	for k in count:
		var z := (float(k) - (count - 1) * 0.5) * 0.2
		var y := 0.19 + 0.012 * k
		for rail in [-0.055, 0.055]:
			beam(st, Vector3(-0.5, y, z + rail), Vector3(0.5, y, z + rail), 0.018, WOOD_FRESH)
		for rung in 9:
			var x := -0.44 + 0.11 * rung
			beam(st, Vector3(x, y, z - 0.055), Vector3(x, y, z + 0.055), 0.012, WOOD_FRESH)
	_cache[key] = _commit(st)
	return _cache[key]


## Engin de repli (maquette absente) : caisse de charpente à l'échelle de `size`, bélier (poutre
## sous un toit) ou beffroi (tour à étages).
static func engine_fallback(kind: String, size: Vector3) -> ArrayMesh:
	var key := "fallback|%s|%s" % [kind, size]
	if _cache.has(key):
		return _cache[key]
	var st := _begin()
	if kind == "tower":
		for level in 3:
			var shrink := 1.0 - 0.12 * level
			box(st, Vector3(0.0, size.y * (level + 0.5) / 3.0, 0.0), Vector3(size.x * shrink, size.y / 3.0 * 0.96, size.z * shrink), WOOD if level % 2 == 0 else WOOD_DARK)
	else:
		box(st, Vector3(0.0, size.y * 0.75, 0.0), Vector3(size.x * 0.8, size.y * 0.12, size.z), WOOD_DARK)
		beam(st, Vector3(-size.x * 0.55, size.y * 0.4, 0.0), Vector3(size.x * 0.55, size.y * 0.4, 0.0), size.z * 0.3, WOOD)
		for sx in [-1.0, 1.0]:
			for sz in [-1.0, 1.0]:
				beam(st, Vector3(sx * size.x * 0.36, 0.0, sz * size.z * 0.42), Vector3(sx * size.x * 0.36, size.y * 0.72, sz * size.z * 0.42), size.z * 0.12, WOOD)
	_cache[key] = _commit(st)
	return _cache[key]
