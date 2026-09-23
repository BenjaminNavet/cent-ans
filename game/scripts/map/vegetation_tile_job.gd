class_name VegetationTileJob
extends RefCounted

## Semis des arbres d'une tuile de végétation (lot V3), exécuté dans un `WorkerThreadPool`.
##
## 1. Grille grossière (`coarse_step` px) des masques (`VegetationMask.sample`).
## 2. Grille de candidats espacés de `spacing` px, décalés aléatoirement (RNG déterministe par
##    tuile) ; chaque candidat devient un feuillu, un conifère, un arbre de bosquet ou un
##    buisson de haie selon les densités interpolées.
## 3. Tampons `MultiMesh` (transform 3×4 + données custom : teinte RVB, graine A) triés par
##    graine décroissante : un préfixe du tampon est un sous-échantillon uniforme, ce qui permet
##    d'éclaircir une tuile lointaine avec `visible_instance_count`.
##
## Aucun accès à l'arbre de scène : sûr hors du fil principal.

enum Kind { DECIDUOUS, CONIFER, HEDGE }
const KIND_COUNT := 3
const FLOATS_PER_INSTANCE := 16

var mask: VegetationMask
var tile_index: int = 0
var origin_px: Vector2i = Vector2i.ZERO
var size_px: int = 256
var spacing: float = 1.5
var coarse_step: int = 4
var tree_scale: float = 1.0
## Cercles d'exclusion (villes) : Vector3(x, y, rayon) en pixels de carte.
var exclusions: PackedVector3Array = PackedVector3Array()

## Résultats : un tampon et un nombre d'instances par `Kind`.
var buffers: Array[PackedFloat32Array] = []
var counts: PackedInt32Array = PackedInt32Array()
var build_ms: float = 0.0

var _forest := PackedFloat32Array()
var _crops := PackedFloat32Array()
var _conifer := PackedFloat32Array()
var _hedge := PackedFloat32Array()
var _grove := PackedFloat32Array()
var _side: int = 0


func run() -> void:
	var t0 := Time.get_ticks_usec()
	var noise := VegetationMask.make_noise(1337)
	var grove_noise := VegetationMask.make_noise(4242)
	grove_noise.frequency = 1.0 / 9.0
	grove_noise.fractal_octaves = 2
	_sample_coarse(noise, grove_noise)
	var raw: Array = []
	for kind in KIND_COUNT:
		raw.append([])
	_scatter(raw)
	buffers.clear()
	counts.resize(KIND_COUNT)
	for kind in KIND_COUNT:
		var items: Array = raw[kind]
		counts[kind] = items.size()
		buffers.append(_pack(items))
	build_ms = (Time.get_ticks_usec() - t0) / 1000.0


func instance_total() -> int:
	var total := 0
	for count in counts:
		total += count
	return total


func _sample_coarse(noise: FastNoiseLite, grove_noise: FastNoiseLite) -> void:
	_side = size_px / coarse_step + 2
	var n := _side * _side
	for array in [_forest, _crops, _conifer, _hedge, _grove]:
		array.resize(n)
	var k := 0
	for j in _side:
		var y := float(origin_px.y + j * coarse_step)
		for i in _side:
			var x := float(origin_px.x + i * coarse_step)
			var s := mask.sample(x, y, noise)
			_forest[k] = s["forest"]
			_crops[k] = s["crops"]
			_conifer[k] = s["conifer"]
			_hedge[k] = s["hedge"]
			_grove[k] = smoothstep(0.28, 0.42, grove_noise.get_noise_2d(x, y))
			k += 1


func _lerp_grid(grid: PackedFloat32Array, gx: float, gy: float) -> float:
	var i0 := mini(int(gx), _side - 2)
	var j0 := mini(int(gy), _side - 2)
	var tx := gx - i0
	var ty := gy - j0
	var k := j0 * _side + i0
	var top := lerpf(grid[k], grid[k + 1], tx)
	var bottom := lerpf(grid[k + _side], grid[k + _side + 1], tx)
	return lerpf(top, bottom, ty)


func _excluded(x: float, y: float) -> bool:
	for e in exclusions:
		var dx := x - e.x
		var dy := y - e.y
		if dx * dx + dy * dy < e.z * e.z:
			return true
	return false


func _scatter(raw: Array) -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = hash(Vector2i(tile_index, 91711))
	var data := mask.map_data
	var cells := ceili(size_px / spacing)
	# Exclusions utiles à cette tuile seulement.
	var local := PackedVector3Array()
	var rect := Rect2(Vector2(origin_px), Vector2(size_px, size_px))
	for e in exclusions:
		if rect.grow(e.z).has_point(Vector2(e.x, e.y)):
			local.append(e)
	exclusions = local
	for cj in cells:
		for ci in cells:
			var x := origin_px.x + (ci + rng.randf()) * spacing
			var y := origin_px.y + (cj + rng.randf()) * spacing
			var roll := rng.randf()
			var roll_kind := rng.randf()
			var gx := (x - origin_px.x) / coarse_step
			var gy := (y - origin_px.y) / coarse_step
			var forest := _lerp_grid(_forest, gx, gy)
			var kind := -1
			var scale_factor := 1.0
			var yaw := rng.randf() * TAU
			if roll < forest * 0.9:
				kind = Kind.CONIFER if roll_kind < _lerp_grid(_conifer, gx, gy) else Kind.DECIDUOUS
			else:
				var crops := _lerp_grid(_crops, gx, gy)
				if crops > 0.15:
					var grove := _lerp_grid(_grove, gx, gy)
					if roll < crops * grove * 0.55:
						kind = Kind.DECIDUOUS
						scale_factor = 0.9
			if kind < 0 or (not exclusions.is_empty() and _excluded(x, y)):
				continue
			var ground := data.height_world_at(x, y)
			if ground <= 0.0:
				continue
			raw[kind].append(_make_instance(rng, kind, x, ground, y, yaw, scale_factor))
	_scatter_hedges(raw, rng)


## Haies : bords d'un parcellaire biaisé (u, v) = ((x + k y) / FIELD_U, (y − k x) / FIELD_V) ;
## chaque bord de parcelle est planté ou non (hasard par bord), les buissons s'y suivent tous
## les `HEDGE_STEP` px. Denses dans le bocage, rares ailleurs.
const FIELD_U := 5.0
const FIELD_V := 4.2
const FIELD_SKEW := 0.35
const HEDGE_STEP := 0.7


func _scatter_hedges(raw: Array, rng: RandomNumberGenerator) -> void:
	var data := mask.map_data
	var k := FIELD_SKEW
	var det := 1.0 + k * k
	# Bornes (u, v) couvrant la tuile.
	var corners := [Vector2(origin_px), Vector2(origin_px) + Vector2(size_px, 0), Vector2(origin_px) + Vector2(0, size_px), Vector2(origin_px) + Vector2(size_px, size_px)]
	var u_min := INF
	var u_max := -INF
	var v_min := INF
	var v_max := -INF
	for c: Vector2 in corners:
		u_min = minf(u_min, (c.x + k * c.y) / FIELD_U)
		u_max = maxf(u_max, (c.x + k * c.y) / FIELD_U)
		v_min = minf(v_min, (c.y - k * c.x) / FIELD_V)
		v_max = maxf(v_max, (c.y - k * c.x) / FIELD_V)
	var rect := Rect2(Vector2(origin_px), Vector2(size_px, size_px))
	for axis in 2:
		var line_min := floori(u_min if axis == 0 else v_min)
		var line_max := ceili(u_max if axis == 0 else v_max)
		var along_min := v_min if axis == 0 else u_min
		var along_max := v_max if axis == 0 else u_max
		var along_scale := FIELD_V if axis == 0 else FIELD_U
		var steps := ceili((along_max - along_min) * along_scale / HEDGE_STEP)
		# Lacet qui aligne l'axe X local du buisson sur la ligne : (−k, 1) pour u constant,
		# (1, k) pour v constant (Basis(UP, a) envoie X sur (cos a, 0, −sin a)).
		var line_yaw := atan2(-1.0, -k) if axis == 0 else atan2(-k, 1.0)
		for line in range(line_min, line_max + 1):
			var edge_roll := -1.0
			var edge_index := -1
			for step in steps:
				var along := along_min + step * HEDGE_STEP / along_scale
				var u: float = float(line) if axis == 0 else along
				var v: float = along if axis == 0 else float(line)
				# Inversion de (u, v) → (x, y).
				var a := u * FIELD_U
				var b := v * FIELD_V
				var x0 := (a - k * b) / det
				var y0 := (b + k * a) / det
				# Déformation douce : parcelles irrégulières, haies légèrement sinueuses.
				var x := x0 + 0.9 * sin(0.23 * y0 + 1.3 * sin(0.061 * x0))
				var y := y0 + 0.9 * sin(0.19 * x0 + 1.1 * sin(0.047 * y0))
				if not rect.has_point(Vector2(x, y)):
					continue
				# Un tirage par bord de parcelle (entre deux lignes transverses).
				var edge := floori(along)
				if edge != edge_index:
					edge_index = edge
					edge_roll = _hash01(line * 7919 + edge * 104729 + axis * 31)
				var gx := (x - origin_px.x) / coarse_step
				var gy := (y - origin_px.y) / coarse_step
				var crops := _lerp_grid(_crops, gx, gy)
				if crops < 0.2:
					continue
				var hedge := _lerp_grid(_hedge, gx, gy)
				if edge_roll > crops * lerpf(0.06, 0.8, hedge):
					continue
				if not exclusions.is_empty() and _excluded(x, y):
					continue
				var jx := x + rng.randf_range(-0.12, 0.12)
				var jy := y + rng.randf_range(-0.12, 0.12)
				var ground := data.height_world_at(jx, jy)
				if ground <= 0.0:
					continue
				raw[Kind.HEDGE].append(_make_instance(rng, Kind.HEDGE, jx, ground, jy, line_yaw + rng.randf_range(-0.2, 0.2), 1.0))


static func _hash01(n: int) -> float:
	var h := (n * 1103515245 + 12345) & 0x7fffffff
	h = (h ^ (h >> 13)) * 1274126177 & 0x7fffffff
	return float(h % 100000) / 100000.0


func _make_instance(rng: RandomNumberGenerator, kind: int, x: float, ground: float, y: float, yaw: float, scale_factor: float) -> Array:
	var height: float
	var width: float
	var tint: Color
	match kind:
		Kind.CONIFER:
			height = rng.randf_range(1.3, 2.1)
			width = height * rng.randf_range(0.85, 1.1)
			var b := rng.randf_range(0.8, 1.15)
			tint = Color(b * rng.randf_range(0.9, 1.05), b, b * rng.randf_range(0.95, 1.1))
		Kind.HEDGE:
			height = rng.randf_range(0.55, 0.75)
			width = rng.randf_range(0.9, 1.2)
			var b := rng.randf_range(0.85, 1.1)
			tint = Color(b * 1.05, b, b * 0.9)
		_:
			height = rng.randf_range(1.1, 1.7)
			width = height * rng.randf_range(0.9, 1.25)
			# Variété des essences : chênes sombres, hêtres clairs, quelques teintes dorées.
			var b := rng.randf_range(0.82, 1.18)
			var warm := rng.randf()
			if warm > 0.9:
				tint = Color(b * 1.35, b * 1.15, b * 0.7)
			elif warm > 0.7:
				tint = Color(b * 1.12, b * 1.08, b * 0.85)
			else:
				tint = Color(b * rng.randf_range(0.9, 1.02), b, b * rng.randf_range(0.9, 1.05))
	height *= tree_scale * scale_factor
	width *= tree_scale * scale_factor
	# Légère inclinaison aléatoire (arbres pas tous au garde-à-vous).
	var basis := Basis(Vector3.UP, yaw) * Basis(Vector3(1, 0, 0), rng.randf_range(-0.06, 0.06))
	basis = basis.scaled_local(Vector3(width, height, width if kind != Kind.HEDGE else width * 0.9))
	# Enfoncé un peu : le maillage du terrain (LOD) ne suit pas exactement l'interpolation bilinéaire.
	var origin := Vector3(x, ground - 0.08 * height, y)
	return [rng.randf(), Transform3D(basis, origin), tint]


## Tampon MultiMesh (TRANSFORM_3D + custom data), trié par graine décroissante ; la graine
## stockée est le rang normalisé (uniforme dans [0, 1]).
func _pack(items: Array) -> PackedFloat32Array:
	items.sort_custom(func(a: Array, b: Array) -> bool: return a[0] > b[0])
	var buffer := PackedFloat32Array()
	buffer.resize(items.size() * FLOATS_PER_INSTANCE)
	var count := items.size()
	var k := 0
	for i in count:
		var t: Transform3D = items[i][1]
		var tint: Color = items[i][2]
		var b := t.basis
		buffer[k] = b.x.x
		buffer[k + 1] = b.y.x
		buffer[k + 2] = b.z.x
		buffer[k + 3] = t.origin.x
		buffer[k + 4] = b.x.y
		buffer[k + 5] = b.y.y
		buffer[k + 6] = b.z.y
		buffer[k + 7] = t.origin.y
		buffer[k + 8] = b.x.z
		buffer[k + 9] = b.y.z
		buffer[k + 10] = b.z.z
		buffer[k + 11] = t.origin.z
		buffer[k + 12] = tint.r
		buffer[k + 13] = tint.g
		buffer[k + 14] = tint.b
		buffer[k + 15] = 1.0 - (i + 0.5) / count
		k += FLOATS_PER_INSTANCE
	return buffer
