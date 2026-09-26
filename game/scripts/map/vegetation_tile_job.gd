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

## Lot V4 (A1-10) : essences — chêne (chênaies, bocage, arbres des champs), hêtre (hêtraies),
## conifère de montagne (sapin, épicéa), haie.
enum Kind { OAK, BEECH, CONIFER, HEDGE }
const KIND_COUNT := 4
## Lot V4 : au cœur des massifs, houppiers élargis jusqu'à se toucher (canopée continue).
const CANOPY_SPREAD := 0.4
## La tuile est rendue en PARTS_SIDE × PARTS_SIDE parties (culling et LOD plus fins).
const PARTS_SIDE := 2
const PARTS := PARTS_SIDE * PARTS_SIDE
const FLOATS_PER_INSTANCE := 16
## Enfoncement du pied (part de la hauteur) : le tronc ne flotte pas sur une pente.
const GROUND_SINK := 0.08
## Lot V4 : distance minimale (px carte) à la berge d'un fleuve pour planter.
const RIVER_CLEARANCE := 0.3

var mask: VegetationMask
var tile_index: int = 0
var origin_px: Vector2i = Vector2i.ZERO
var size_px: int = 256
var spacing: float = 1.5
var coarse_step: int = 4
var tree_scale: float = 1.0
## Cercles d'exclusion (villes) : Vector3(x, y, rayon) en pixels de carte.
var exclusions: PackedVector3Array = PackedVector3Array()
## Lot C7b : grille de hauteurs du maillage de terrain affiché (`TerrainBuilder.surface_grid`)
## au lancement du semis ; les arbres y sont posés (vide : heightmap 4096 bilinéaire). Le tri
## terre / mer reste fait sur la heightmap : même semis quel que soit le niveau de relief.
var ground_grid: Dictionary = {}
## Lot PB2 : seule la grille grossière est calculée ici (`run`) ; le semis, les haies et
## l'empaquetage sont faits par `VegetationScatter` (Rust, fils natifs) à partir de
## `native_params`, puis `apply_native` installe le résultat.
var coarse_only: bool = false

## Résultats : un tampon et un nombre d'instances par emplacement `part * KIND_COUNT + kind`.
var buffers: Array[PackedFloat32Array] = []
var counts: PackedInt32Array = PackedInt32Array()
var build_ms: float = 0.0

var _forest := PackedFloat32Array()
var _crops := PackedFloat32Array()
var _conifer := PackedFloat32Array()
var _beech := PackedFloat32Array()
var _hedge := PackedFloat32Array()
var _grove := PackedFloat32Array()
var _region := PackedFloat32Array()
var _side: int = 0


func run() -> void:
	var t0 := Time.get_ticks_usec()
	var noise := VegetationMask.make_noise(1337)
	var grove_noise := VegetationMask.make_noise(4242)
	grove_noise.frequency = 1.0 / 9.0
	grove_noise.fractal_octaves = 2
	_sample_coarse(noise, grove_noise)
	if coarse_only:
		build_ms = (Time.get_ticks_usec() - t0) / 1000.0
		return
	var raw: Array = []
	for slot in PARTS * KIND_COUNT:
		raw.append([])
	_scatter(raw)
	buffers.clear()
	counts.resize(PARTS * KIND_COUNT)
	for slot in PARTS * KIND_COUNT:
		var items: Array = raw[slot]
		counts[slot] = items.size()
		buffers.append(_pack(items))
	build_ms = (Time.get_ticks_usec() - t0) / 1000.0


## Lot PB2 : paramètres de `VegetationScatter.request` (après `run` en mode `coarse_only`).
func native_params() -> Dictionary:
	return {
		"tile_index": tile_index, "origin_x": float(origin_px.x), "origin_y": float(origin_px.y),
		"size_px": float(size_px), "spacing": spacing, "coarse_step": float(coarse_step),
		"tree_scale": tree_scale, "vertical_scale": MapData.vertical_scale(), "relief_gain": MapData.relief_gain(),
		"relief_squash": MapData.relief_squash(),
		"side": _side,
		"coarse": [_forest, _crops, _conifer, _beech, _hedge, _grove, _region],
		"exclusions": exclusions, "ground_grid": ground_grid,
	}


## Lot PB2 : résultat de `VegetationScatter.poll` (tampons et nombres par emplacement).
func apply_native(result: Dictionary) -> void:
	buffers.clear()
	for buffer: PackedFloat32Array in result["buffers"]:
		buffers.append(buffer)
	counts = result["counts"]
	build_ms += float(result["ms"])


## Lot SZ4b : grilles grossières gardées pour la forêt dense (`ForestDetail`), {} si absentes.
func coarse_params() -> Dictionary:
	if _side < 2:
		return {}
	return {
		"origin_x": float(origin_px.x), "origin_y": float(origin_px.y), "size_px": float(size_px),
		"coarse_step": float(coarse_step), "side": _side,
		"coarse": [_forest, _crops, _conifer, _beech, _hedge, _grove, _region],
	}


func instance_total() -> int:
	var total := 0
	for count in counts:
		total += count
	return total


func _sample_coarse(noise: FastNoiseLite, grove_noise: FastNoiseLite) -> void:
	_side = size_px / coarse_step + 2
	var n := _side * _side
	for array in [_forest, _crops, _conifer, _beech, _hedge, _grove, _region]:
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
			_beech[k] = s["beech"]
			_hedge[k] = s["hedge"]
			_grove[k] = smoothstep(0.28, 0.42, grove_noise.get_noise_2d(x, y))
			# Région (frontière des deux trames de parcelles) : variation bien plus lente que le
			# pas de la grille grossière (erreur d'interpolation très inférieure à la zone morte de
			# 0,02 dans `_hedge_point`) → on évite le coût des 4 sinus par candidat de haie.
			_region[k] = VegetationFields.region_value(x, y)
			k += 1


## Emplacement de résultat d'une instance (partie de la tuile contenant (x, y), essence).
func _slot(kind: int, x: float, y: float) -> int:
	var part_px := float(size_px) / PARTS_SIDE
	var px := clampi(int((x - origin_px.x) / part_px), 0, PARTS_SIDE - 1)
	var py := clampi(int((y - origin_px.y) / part_px), 0, PARTS_SIDE - 1)
	return (py * PARTS_SIDE + px) * KIND_COUNT + kind


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
				if roll_kind < _lerp_grid(_conifer, gx, gy):
					kind = Kind.CONIFER
				else:
					kind = Kind.BEECH if rng.randf() < _lerp_grid(_beech, gx, gy) else Kind.OAK
				# Canopée : houppiers plus larges au cœur du massif (lisières plus claires).
				scale_factor = 1.0 + CANOPY_SPREAD * smoothstep(0.45, 0.9, forest)
			else:
				var crops := _lerp_grid(_crops, gx, gy)
				if crops > 0.15:
					var grove := _lerp_grid(_grove, gx, gy)
					# Bosquets (bruit) et quelques arbres isolés dans les champs.
					if roll < crops * (grove * 0.55 + 0.012):
						kind = Kind.BEECH if rng.randf() < 0.2 else Kind.OAK
						scale_factor = 0.9
			if kind < 0 or (not exclusions.is_empty() and _excluded(x, y)):
				continue
			# La dernière colonne de la grille déborde (jusqu'à `spacing`) sur la tuile voisine : ces
			# candidats appartiennent à la voisine (sinon bande deux fois plus dense, et pied posé
			# sur le maillage d'une autre tuile, lot C7b).
			if not rect.has_point(Vector2(x, y)):
				continue
			# Lot V4 : pas d'arbre dans le lit ni sur la berge immédiate des fleuves.
			if data.river_sd_at(x, y) < RIVER_CLEARANCE:
				continue
			var ground := data.height_world_at(x, y)
			if ground <= 0.0:
				continue
			ground = _display_ground(x, y, ground)
			raw[_slot(kind, x, y)].append(_make_instance(rng, kind, x, ground, y, yaw, scale_factor))
	_scatter_hedges(raw, rng)


## Haies (lot V2b) : bords d'enclos du parcellaire partagé avec le shader de terrain
## (`VegetationFields`), plantés selon le même tirage que le pied de haie peint ; buissons tous
## les `HEDGE_STEP` px, trouées, quelques arbres de haie. Denses dans le bocage, rares ailleurs.
const HEDGE_STEP := 0.62
## Part de buissons manquants (trouées courtes) et d'arbres de haie.
const HEDGE_GAP := 0.1
const HEDGE_TREE := 0.07


func _scatter_hedges(raw: Array, rng: RandomNumberGenerator) -> void:
	var rect := Rect2(Vector2(origin_px), Vector2(size_px, size_px))
	# PB1 : borne sûre de `hedge_probability` sur la tuile (croissante en culture et en bocage, et
	# les valeurs interpolées restent entre celles de la grille) : un bord dont le tirage la
	# dépasse n'est jamais planté ; on le saute sans calculer sa déformation (~98 % des candidats
	# étaient rejetés après coup).
	var crops_max := 0.0
	var hedge_max := 0.0
	for k in _crops.size():
		crops_max = maxf(crops_max, _crops[k])
		hedge_max = maxf(hedge_max, _hedge[k])
	if crops_max < 0.2:
		return
	var p_max := VegetationFields.hedge_probability(crops_max, hedge_max)
	# La déformation du repère déplace les lignes de ±2,3 px au plus.
	var bounds := rect.grow(3.0)
	for layout in VegetationFields.LAYOUTS.size():
		var params: Array = VegetationFields.LAYOUTS[layout]
		var u_min := INF
		var u_max := -INF
		var v_min := INF
		var v_max := -INF
		for c: Vector2 in [bounds.position, Vector2(bounds.end.x, bounds.position.y), Vector2(bounds.position.x, bounds.end.y), bounds.end]:
			var uv := VegetationFields.to_uv(layout, c.x, c.y)
			u_min = minf(u_min, uv.x)
			u_max = maxf(u_max, uv.x)
			v_min = minf(v_min, uv.y)
			v_max = maxf(v_max, uv.y)
		var k: float = params[0]
		var fu: float = params[1]
		var fv: float = params[2]
		# `to_map` (avant déformation) est un plan affine de (u, v) : x0 = ax(u) + bx*v,
		# y0 = ay(u) + by*v. La boîte englobante ci-dessus (coins du rectangle projetés) est très
		# gonflée par le cisaillement des trames (jusqu'à ~3× de surface en trop pour la trame la
		# plus cisaillée) : on resserre par ligne la plage de `v` réellement utile par intersection
		# linéaire exacte avec `bounds`, sans changer la grille de tirage — les segments écartés
		# étaient de toute façon rejetés par `rect.has_point` avant tout tirage aléatoire (mêmes
		# valeurs de `v`, mêmes tirages consommés pour les segments conservés).
		var det := 1.0 + k * k
		var by := fv / det
		var bx := -k * fv / det
		var dv := HEDGE_STEP / fv
		for line in range(floori(u_min), ceili(u_max) + 1):
			var a0 := float(line) * fu
			var range_v := _clip_linear(by, k * a0 / det, bounds.position.y, bounds.end.y, v_min, v_max)
			range_v = _clip_linear(bx, a0 / det, bounds.position.x, bounds.end.x, range_v.x, range_v.y)
			if range_v.x >= range_v.y:
				continue
			var v := v_min
			while v < range_v.x:
				v += dv
			# Point courant réutilisé comme précédent du pas suivant ; recalculé seulement après un
			# segment sauté (`to_map` est pure : mêmes valeurs).
			var cur := Vector2.ZERO
			var cur_ok := false
			while v < range_v.y:
				var next_v := v + dv
				var roll := VegetationFields.roll_u_edge(layout, line, floori(v))
				if roll < p_max:
					if not cur_ok:
						cur = VegetationFields.to_map(layout, line, v)
					var nxt := VegetationFields.to_map(layout, line, next_v)
					_hedge_point(raw, rng, rect, layout, roll, cur, nxt)
					cur = nxt
					cur_ok = true
				else:
					cur_ok = false
				v = next_v
		# Bords de rangée : décalés d'une colonne à l'autre (jonctions en T). Même resserrement,
		# avec une marge couvrant la largeur de la colonne (le tirage `u` varie sur
		# `[column, column+1]`, pas un point unique comme pour les lignes ci-dessus).
		var du := HEDGE_STEP / fu
		var cx := fu / det
		var cy := k * fu / det
		var margin_x := 0.5 * absf(cx)
		var margin_y := 0.5 * absf(cy)
		for column in range(floori(u_min), ceili(u_max) + 1):
			var offset := VegetationFields.row_offset(layout, column)
			var u_mid := float(column) + 0.5
			var range_row := _clip_linear(by, cy * u_mid, bounds.position.y - margin_y, bounds.end.y + margin_y, v_min, v_max)
			range_row = _clip_linear(bx, cx * u_mid, bounds.position.x - margin_x, bounds.end.x + margin_x, range_row.x, range_row.y)
			if range_row.x >= range_row.y:
				continue
			var row_lo := floori(range_row.x + offset) - 1
			var row_hi := ceili(range_row.y + offset) + 2
			for row in range(row_lo, row_hi):
				var roll := VegetationFields.roll_v_edge(layout, row, column)
				if roll >= p_max:
					continue
				var v_row := row - offset
				var u := float(column) + du * 0.5
				var cur := VegetationFields.to_map(layout, u, v_row)
				while u < column + 1.0:
					var next_u := u + du
					var nxt := VegetationFields.to_map(layout, next_u, v_row)
					_hedge_point(raw, rng, rect, layout, roll, cur, nxt)
					u = next_u
					cur = nxt


## Bornes de `t` telles que `offset + coeff * t` reste dans `[lo, hi]`, croisées avec
## `[cur_lo, cur_hi]`. Intervalle vide (résultat x >= y) si aucune valeur de `t` ne convient.
static func _clip_linear(coeff: float, offset: float, lo: float, hi: float, cur_lo: float, cur_hi: float) -> Vector2:
	if coeff == 0.0:
		return Vector2(cur_lo, cur_hi) if (offset >= lo and offset <= hi) else Vector2(1.0, -1.0)
	var t_a := (lo - offset) / coeff
	var t_b := (hi - offset) / coeff
	if coeff > 0.0:
		return Vector2(maxf(cur_lo, t_a), minf(cur_hi, t_b))
	return Vector2(maxf(cur_lo, t_b), minf(cur_hi, t_a))


## Un buisson (ou un arbre de haie) en `pos` si le bord est planté ; `next` donne la direction.
func _hedge_point(raw: Array, rng: RandomNumberGenerator, rect: Rect2, layout: int, roll: float, pos: Vector2, next: Vector2) -> void:
	if not rect.has_point(pos):
		return
	var gx := (pos.x - origin_px.x) / coarse_step
	var gy := (pos.y - origin_px.y) / coarse_step
	# Région interpolée sur la grille grossière (au lieu des 4 sinus de `region_value` exact) :
	# la fonction varie sur des centaines de px, l'erreur d'interpolation est négligeable devant
	# la zone morte de 0,02 ci-dessous.
	var region := _lerp_grid(_region, gx, gy)
	# Chaque trame ne plante que dans sa région ; la limite (chemin) reste dégagée.
	if (region > 0.0) != (layout == 1) or absf(region) < 0.02:
		return
	var crops := _lerp_grid(_crops, gx, gy)
	if crops < 0.2:
		return
	if roll >= VegetationFields.hedge_probability(crops, _lerp_grid(_hedge, gx, gy)):
		return
	# PB1 : tirages après les tests déterministes (les bords plantés ne dépendent que du tirage
	# haché `roll`) ; trouées, arbres de haie et petites variations gardent leur loi.
	var gap := rng.randf()
	var tree := rng.randf()
	var jitter := Vector2(rng.randf_range(-0.07, 0.07), rng.randf_range(-0.07, 0.07))
	if gap < HEDGE_GAP:
		return
	if not exclusions.is_empty() and _excluded(pos.x, pos.y):
		return
	if mask.map_data.river_sd_at(pos.x, pos.y) < RIVER_CLEARANCE:
		return
	# Reste dans la tuile : le pied est posé sur le maillage de cette tuile (lot C7b).
	var p := (pos + jitter).clamp(rect.position, rect.end - Vector2(0.001, 0.001))
	var ground := mask.map_data.height_world_at(p.x, p.y)
	if ground <= 0.0:
		return
	ground = _display_ground(p.x, p.y, ground)
	var dir := next - pos
	# Basis(UP, a) envoie X sur (cos a, 0, −sin a) : aligne le buisson sur le bord.
	var yaw := atan2(-dir.y, dir.x) + rng.randf_range(-0.15, 0.15)
	if tree < HEDGE_TREE:
		raw[_slot(Kind.OAK, p.x, p.y)].append(_make_instance(rng, Kind.OAK, p.x, ground, p.y, rng.randf() * TAU, 0.78))
	else:
		raw[_slot(Kind.HEDGE, p.x, p.y)].append(_make_instance(rng, Kind.HEDGE, p.x, ground, p.y, yaw, 1.0))


## Hauteur de la surface affichée (grille du maillage) ou, à défaut, `fallback`.
func _display_ground(x: float, y: float, fallback: float) -> float:
	if ground_grid.is_empty():
		return fallback
	return maxf(TerrainBuilder.grid_height(ground_grid, x - origin_px.x, y - origin_px.y), 0.0)


## Lot C7b : repose les instances d'un tampon (`_pack`) sur la grille `grid` d'une tuile dont le
## coin est en `origin` (px carte) : même enfoncement que `_make_instance` (8 % de la hauteur,
## longueur de la colonne Y de la base). Rend un nouveau tampon ; sûr hors du fil principal.
static func reground(buffer: PackedFloat32Array, grid: Dictionary, origin: Vector2) -> PackedFloat32Array:
	var result := buffer.duplicate()
	if grid.is_empty():
		return result
	var k := 0
	var size := result.size()
	while k < size:
		var height := Vector3(result[k + 1], result[k + 5], result[k + 9]).length()
		var ground := maxf(TerrainBuilder.grid_height(grid, result[k + 3] - origin.x, result[k + 11] - origin.y), 0.0)
		result[k + 7] = ground - GROUND_SINK * height
		k += FLOATS_PER_INSTANCE
	return result


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
			# Haies basses et fines, teintes variées (aubépine, noisetier, ronces).
			height = rng.randf_range(0.3, 0.46)
			width = rng.randf_range(0.72, 0.95)
			var b := rng.randf_range(1.0, 1.3)
			var warm := rng.randf()
			tint = Color(b * (1.12 if warm > 0.7 else 1.0), b, b * (0.78 if warm > 0.7 else 0.9))
		Kind.BEECH:
			# Hêtre : fût élancé, houppier haut et rond (palette plus claire dans le maillage).
			height = rng.randf_range(1.35, 1.95)
			width = height * rng.randf_range(0.78, 0.98)
			var b := rng.randf_range(0.88, 1.12)
			tint = Color(b * rng.randf_range(0.95, 1.05), b, b * rng.randf_range(0.9, 1.0))
		_:
			height = rng.randf_range(1.1, 1.7)
			width = height * rng.randf_range(0.95, 1.3)
			# Chênes : teintes variées, quelques houppiers plus dorés.
			var b := rng.randf_range(0.82, 1.18)
			var warm := rng.randf()
			if warm > 0.9:
				tint = Color(b * 1.35, b * 1.15, b * 0.7)
			elif warm > 0.7:
				tint = Color(b * 1.12, b * 1.08, b * 0.85)
			else:
				tint = Color(b * rng.randf_range(0.9, 1.02), b, b * rng.randf_range(0.9, 1.05))
	# `scale_factor` > 1 (cœur de forêt) élargit surtout le houppier.
	height *= tree_scale * (scale_factor if scale_factor <= 1.0 else 1.0 + (scale_factor - 1.0) * 0.35)
	width *= tree_scale * scale_factor
	# Légère inclinaison aléatoire (arbres pas tous au garde-à-vous).
	var basis := Basis(Vector3.UP, yaw) * Basis(Vector3(1, 0, 0), rng.randf_range(-0.06, 0.06))
	basis = basis.scaled_local(Vector3(width, height, width if kind != Kind.HEDGE else width * 0.62))
	# Enfoncé un peu : le maillage du terrain (LOD) ne suit pas exactement l'interpolation bilinéaire.
	var origin := Vector3(x, ground - GROUND_SINK * height, y)
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
