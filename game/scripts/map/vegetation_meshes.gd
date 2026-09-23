class_name VegetationMeshes
extends RefCounted

## Maillages procéduraux des arbres de la carte (lot V3) : feuillu (houppier en trois
## masses bosselées sur un tronc) et conifère (trois cônes étagés). Hauteur totale 1,0 :
## l'instance fixe la taille réelle. Couleur de sommet : RVB = albédo de base (teinté par
## instance dans `foliage.gdshader`), A = poids du balancement au vent (0 au pied, 1 en haut).
## Normales « gonflées » (depuis le centre de chaque masse) : ombrage doux de feuillage.

const BARK := Color(0.23, 0.17, 0.11)
const LEAF_DARK := Color(0.06, 0.11, 0.035)
const LEAF_LIGHT := Color(0.17, 0.25, 0.07)
const NEEDLE_DARK := Color(0.03, 0.07, 0.04)
const NEEDLE_LIGHT := Color(0.08, 0.14, 0.07)

static var _cache: Dictionary = {}


static func clear_cache() -> void:
	_cache.clear()


static func deciduous() -> ArrayMesh:
	if not _cache.has("deciduous"):
		_cache["deciduous"] = _build_deciduous()
	return _cache["deciduous"]


static func conifer() -> ArrayMesh:
	if not _cache.has("conifer"):
		_cache["conifer"] = _build_conifer()
	return _cache["conifer"]


## Variantes lointaines (≈ 20 triangles) : une masse, pas de tronc.
static func deciduous_low() -> ArrayMesh:
	if not _cache.has("deciduous_low"):
		var st := SurfaceTool.new()
		st.begin(Mesh.PRIMITIVE_TRIANGLES)
		_add_blob(st, Vector3(0.0, 0.6, 0.0), Vector3(0.48, 0.38, 0.48), 11, false)
		_cache["deciduous_low"] = st.commit()
	return _cache["deciduous_low"]


static func conifer_low() -> ArrayMesh:
	if not _cache.has("conifer_low"):
		var st := SurfaceTool.new()
		st.begin(Mesh.PRIMITIVE_TRIANGLES)
		_add_cone(st, 0.1, 1.0, 0.30, 5, 0.0)
		_cache["conifer_low"] = st.commit()
	return _cache["conifer_low"]


## Buisson de haie : masse basse et allongée (ligne de haie), sans tronc visible.
static func hedge() -> ArrayMesh:
	if not _cache.has("hedge"):
		_cache["hedge"] = _build_hedge()
	return _cache["hedge"]


static func _build_deciduous() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	_add_trunk(st, 0.055, 0.42, 5)
	# Trois masses : centrale haute, deux latérales plus basses (silhouette irrégulière).
	_add_blob(st, Vector3(0.0, 0.66, 0.0), Vector3(0.42, 0.33, 0.42), 11, true)
	_add_blob(st, Vector3(0.2, 0.52, 0.09), Vector3(0.28, 0.22, 0.28), 23, false)
	_add_blob(st, Vector3(-0.17, 0.50, -0.14), Vector3(0.27, 0.21, 0.27), 37, false)
	return st.commit()


static func _build_conifer() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	_add_trunk(st, 0.04, 0.2, 5)
	_add_cone(st, 0.12, 0.62, 0.30, 7, 0.0)
	_add_cone(st, 0.36, 0.86, 0.23, 7, 0.45)
	_add_cone(st, 0.58, 1.0, 0.15, 7, 0.9)
	return st.commit()


static func _build_hedge() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	_add_blob(st, Vector3(0.0, 0.3, 0.0), Vector3(0.55, 0.3, 0.26), 51, false)
	return st.commit()


static func _add_trunk(st: SurfaceTool, radius: float, height: float, sides: int) -> void:
	for i in sides:
		var a0 := TAU * i / sides
		var a1 := TAU * (i + 1) / sides
		var p0 := Vector3(cos(a0), 0.0, sin(a0))
		var p1 := Vector3(cos(a1), 0.0, sin(a1))
		var b0 := p0 * radius
		var b1 := p1 * radius
		var t0 := p0 * radius * 0.7 + Vector3.UP * height
		var t1 := p1 * radius * 0.7 + Vector3.UP * height
		# Face avant en sens horaire vu de l'extérieur.
		for v in [[b0, p0, 0.0], [t1, p1, 0.2], [b1, p1, 0.0], [b0, p0, 0.0], [t0, p0, 0.2], [t1, p1, 0.2]]:
			st.set_color(Color(BARK.r, BARK.g, BARK.b, v[2]))
			st.set_normal(v[1])
			st.add_vertex(v[0] - Vector3.UP * 0.05)


## Icosaèdre (subdivisé une fois si `fine`), sommets bosselés (hash déterministe), normales sphériques.
static func _add_blob(st: SurfaceTool, center: Vector3, radii: Vector3, seed_value: int, fine: bool) -> void:
	var faces := _icosphere_faces(fine)
	for face in faces:
		for corner in [0, 2, 1]:
			var dir: Vector3 = face[corner]
			var bump := 0.85 + 0.3 * _hash01(dir * 7.0 + Vector3.ONE * seed_value)
			var p := center + Vector3(dir.x * radii.x, dir.y * radii.y, dir.z * radii.z) * bump
			# Occlusion : le dessous et l'intérieur du houppier sont plus sombres.
			var light := clampf(0.5 + 0.5 * dir.y, 0.0, 1.0)
			var color := LEAF_DARK.lerp(LEAF_LIGHT, light * 0.8 + 0.2 * _hash01(dir * 3.0 + Vector3.ONE * seed_value))
			color.a = clampf(p.y, 0.0, 1.0)
			st.set_color(color)
			st.set_normal((dir + Vector3.UP * 0.25).normalized())
			st.add_vertex(p)


static func _add_cone(st: SurfaceTool, y0: float, y1: float, radius: float, sides: int, twist: float) -> void:
	var apex := Vector3(0.0, y1, 0.0)
	var slope := radius / (y1 - y0)
	for i in sides:
		var a0 := TAU * i / sides + twist
		var a1 := TAU * (i + 1) / sides + twist
		var am := (a0 + a1) * 0.5
		var r0 := radius * (0.9 + 0.2 * _hash01(Vector3(a0, y0, 1.0)))
		var r1 := radius * (0.9 + 0.2 * _hash01(Vector3(a1, y0, 1.0)))
		var b0 := Vector3(cos(a0) * r0, y0, sin(a0) * r0)
		var b1 := Vector3(cos(a1) * r1, y0, sin(a1) * r1)
		var n0 := Vector3(cos(a0), slope, sin(a0)).normalized()
		var n1 := Vector3(cos(a1), slope, sin(a1)).normalized()
		var nm := Vector3(cos(am), slope, sin(am)).normalized()
		var dark := NEEDLE_DARK
		var light := NEEDLE_LIGHT
		for v in [[b0, n0, dark], [apex, nm, light], [b1, n1, dark]]:
			var c: Color = v[2]
			c.a = clampf((v[0] as Vector3).y, 0.0, 1.0)
			st.set_color(c)
			st.set_normal(v[1])
			st.add_vertex(v[0])
		# Dessous de la jupe (visible en vue rasante) : un triangle vers l'axe.
		var inner := Vector3(0.0, y0 + (y1 - y0) * 0.2, 0.0)
		for v in [[b0, b1, inner]]:
			for p in v:
				var c2 := dark.darkened(0.35)
				c2.a = clampf((p as Vector3).y, 0.0, 1.0)
				st.set_color(c2)
				st.set_normal(Vector3.DOWN)
				st.add_vertex(p)


static func _hash01(p: Vector3) -> float:
	var h := sin(p.dot(Vector3(12.9898, 78.233, 37.719))) * 43758.5453
	return h - floorf(h)


## Faces d'un icosaèdre (20 triangles) ou subdivisé une fois (80), sommets unitaires.
static func _icosphere_faces(fine: bool) -> Array:
	var key := "ico_faces_%d" % int(fine)
	if _cache.has(key):
		return _cache[key]
	var t := (1.0 + sqrt(5.0)) / 2.0
	var v: Array[Vector3] = [
		Vector3(-1, t, 0), Vector3(1, t, 0), Vector3(-1, -t, 0), Vector3(1, -t, 0),
		Vector3(0, -1, t), Vector3(0, 1, t), Vector3(0, -1, -t), Vector3(0, 1, -t),
		Vector3(t, 0, -1), Vector3(t, 0, 1), Vector3(-t, 0, -1), Vector3(-t, 0, 1),
	]
	for i in v.size():
		v[i] = v[i].normalized()
	var idx := [
		[0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
		[1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
		[3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
		[4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
	]
	var faces: Array = []
	for f in idx:
		var a: Vector3 = v[f[0]]
		var b: Vector3 = v[f[1]]
		var c: Vector3 = v[f[2]]
		if not fine:
			faces.append([a, b, c])
			continue
		var ab := ((a + b) * 0.5).normalized()
		var bc := ((b + c) * 0.5).normalized()
		var ca := ((c + a) * 0.5).normalized()
		faces.append([a, ab, ca])
		faces.append([b, bc, ab])
		faces.append([c, ca, bc])
		faces.append([ab, bc, ca])
	_cache[key] = faces
	return faces
