class_name FineBedCarver
extends RefCounted

## Lit des fleuves fins creusé dans les pages du quadtree de relief (lot ZG5b, ADR 0036).
##
## Choix : **abaissement des pages** plutôt qu'une texture de lit à part. Chaque page de hauteurs
## (tuile 512² de la pyramide, étage ≥ E3) est retouchée dans un fil de travail avant son
## téléversement (`ReliefQuadtree.page_filter`) : sous chaque fleuve fin, la hauteur descend sous
## le niveau d'eau `z` de la tuile CAFV (profil parabolique, berges fondues sur `bank` mètres).
## Un seul état pour tout le monde : déplacement des patchs (vertex shader), normales d'ombrage
## (berges visibles dans le fragment), morphing vers la page parente (creusée à sa résolution) et
## surface côté processeur (`surface_height_at` : ponts, maquettes, rubans). Aucun coût par image.
##
## Largeur creusée : au moins `min_half_px` pixels de page de chaque côté (un ruisseau de 5 m sur
## E4 à 22 m/px creuse un sillon d'un pixel et demi). Rangs creusés (ordre de Strahler ou largeur) :
## ≥ 5 à E3, ≥ 3 au-delà. Pas de lit sous les
## emprises des colonies (l'eau passe sous les villes, comme `RiversRenderer`).

## Pixels de page minimaux de chaque côté de l'axe (sillon des cours d'eau étroits).
const MIN_HALF_PX := 0.75
## Étage minimal creusé : E3 (45 m). Les pages E1-E2 (vue moyenne, lointain de la vue comté) ne
## le sont pas : l'eau y flotte au-dessus de la surface (`FineRibbonJob.RIVER_LIFT_M`), invisible à
## cette distance, et le panoramique ne paie pas le creusement de centaines de pages E2.
const MIN_LEVEL := 3
## Largeur de berge : part de la largeur du lit, au moins `BANK_MIN_PX` pixels de page.
const BANK_FRACTION := 0.35
const BANK_MIN_PX := 1.5
const BANK_MAX_M := 250.0
## Demi-largeur + berge maximales (estuaires de 5 km : Gironde au Verdon), pour l'emprise.
const MAX_REACH_M := 2800.0
## Profondeur au milieu (m) : 0,6 m + 1,2 % de la largeur, bornée à 4 m ; au bord de l'eau, le
## fond est `EDGE_DEPTH` sous le niveau d'eau (l'eau touche la berge).
const DEPTH_BASE := 0.6
const DEPTH_PER_M := 0.012
const DEPTH_MAX := 4.0
const EDGE_DEPTH := 0.25

var store: FineGeoStore
var meters_per_unit: float = 719.0
var height_min_m: float = -200.0
var height_range_m: float = 5000.0
## Emprises des colonies (x, y, rayon, index) où l'on ne creuse pas.
var covers: PackedVector4Array = PackedVector4Array()
## Pages retouchées, pages sans fleuve, pixels abaissés (mesures).
var stats: Dictionary = {"pages": 0, "skipped": 0}


func _init(fine_store: FineGeoStore, m_per_unit: float, h_min: float, h_range: float) -> void:
	store = fine_store
	meters_per_unit = m_per_unit
	height_min_m = h_min
	height_range_m = h_range


static func min_order_for_level(level: int) -> int:
	return 5 if level <= 3 else 3


## Appelé par `ReliefQuadtree` sur le fil principal quand une page est décodée : rend une tâche
## (`apply(bytes)`, exécutée dans un fil : lecture des tuiles CAFV comprise) ou null s'il n'y a
## rien à creuser. Aucune entrée-sortie ici (fil principal).
func carve_job(key: int) -> Object:
	var level := ReliefPyramid.level_of_key(key)
	if level < MIN_LEVEL or store == null:
		return null
	var origin := ReliefPyramid.tile_origin(level, ReliefPyramid.col_of_key(key), ReliefPyramid.row_of_key(key))
	var units := ReliefPyramid.tile_units(level)
	var rect := Rect2(origin, Vector2(units, units)).grow(MAX_REACH_M / meters_per_unit)
	var task := CarveTask.new()
	var c0 := int(floor(rect.position.x / FineGeoStore.TILE_UNITS))
	var c1 := int(floor(rect.end.x / FineGeoStore.TILE_UNITS))
	var r0 := int(floor(rect.position.y / FineGeoStore.TILE_UNITS))
	var r1 := int(floor(rect.end.y / FineGeoStore.TILE_UNITS))
	for row in range(r0, r1 + 1):
		for col in range(c0, c1 + 1):
			if store.has_tile(CafvTile.LAYER_RIVERS, col, row):
				task.tile_keys.append(Vector2i(col, row))
	if task.tile_keys.is_empty():
		stats["skipped"] = int(stats["skipped"]) + 1
		return null
	for c in covers:
		if rect.grow(c.z).has_point(Vector2(c.x, c.y)):
			task.covers.append(c)
	task.store = store
	task.rect = rect
	task.min_order = min_order_for_level(level)
	task.origin = origin
	task.px_units = units / ReliefPyramid.TILE_PX
	task.meters_per_unit = meters_per_unit
	task.h_min = height_min_m
	task.h_range = height_range_m
	stats["pages"] = int(stats["pages"]) + 1
	return task


## Creusement d'une page (fil de travail) : octets 16 bits petit-boutistes, 512².
class CarveTask:
	extends RefCounted

	var store: FineGeoStore
	var tile_keys: Array[Vector2i] = []
	var rect: Rect2
	var min_order: int = 3
	var tiles: Array[CafvTile] = []
	var lines: Array[PackedInt32Array] = []
	var covers: PackedVector4Array = PackedVector4Array()
	var origin: Vector2 = Vector2.ZERO
	var px_units: float = 1.0
	var meters_per_unit: float = 719.0
	var h_min: float = -200.0
	var h_range: float = 5000.0
	var carved_px: int = 0

	func apply(bytes: PackedByteArray) -> PackedByteArray:
		var side := ReliefPyramid.TILE_PX
		if bytes.size() != side * side * 2:
			return bytes
		_gather()
		var out := bytes
		var px_m := px_units * meters_per_unit
		var to_code := 65535.0 / h_range
		for t in tiles.size():
			var tile := tiles[t]
			for li in lines[t]:
				var s := tile.line_start[li]
				var e := s + tile.line_count[li] - 1
				for k in range(s, e):
					_segment(out, side, tile, k, px_m, to_code)
		return out

	## Lignes des tuiles CAFV qui touchent la page (lecture dans ce fil).
	func _gather() -> void:
		tiles.clear()
		lines.clear()
		for k in tile_keys:
			var tile := store.fetch_threadsafe(CafvTile.LAYER_RIVERS, k.x, k.y)
			if tile == null:
				continue
			var picked := PackedInt32Array()
			for i in tile.lines():
				if tile.line_rank[i] < min_order:
					continue
				var b := tile.line_bounds[i]
				if b.z < rect.position.x or b.x > rect.end.x or b.w < rect.position.y or b.y > rect.end.y:
					continue
				picked.append(i)
			if not picked.is_empty():
				tiles.append(tile)
				lines.append(picked)

	func _segment(out: PackedByteArray, side: int, tile: CafvTile, k: int, px_m: float, to_code: float) -> void:
		var ax := tile.x[k]
		var ay := tile.y[k]
		var bx := tile.x[k + 1]
		var by := tile.y[k + 1]
		var za := tile.z[k]
		var zb := tile.z[k + 1]
		var half_a := maxf(tile.w[k] * 0.5, FineBedCarver.MIN_HALF_PX * px_m)
		var half_b := maxf(tile.w[k + 1] * 0.5, FineBedCarver.MIN_HALF_PX * px_m)
		var bank_a := maxf(minf(tile.w[k] * FineBedCarver.BANK_FRACTION, FineBedCarver.BANK_MAX_M), FineBedCarver.BANK_MIN_PX * px_m)
		var bank_b := maxf(minf(tile.w[k + 1] * FineBedCarver.BANK_FRACTION, FineBedCarver.BANK_MAX_M), FineBedCarver.BANK_MIN_PX * px_m)
		var reach := maxf(half_a + bank_a, half_b + bank_b) / meters_per_unit
		var i0 := maxi(int(floor((minf(ax, bx) - reach - origin.x) / px_units - 0.5)), 0)
		var i1 := mini(int(ceil((maxf(ax, bx) + reach - origin.x) / px_units - 0.5)), side - 1)
		var j0 := maxi(int(floor((minf(ay, by) - reach - origin.y) / px_units - 0.5)), 0)
		var j1 := mini(int(ceil((maxf(ay, by) + reach - origin.y) / px_units - 0.5)), side - 1)
		if i0 > i1 or j0 > j1:
			return
		var dx := bx - ax
		var dy := by - ay
		var len2 := maxf(dx * dx + dy * dy, 1e-12)
		for j in range(j0, j1 + 1):
			var cy := origin.y + (j + 0.5) * px_units
			for i in range(i0, i1 + 1):
				var cx := origin.x + (i + 0.5) * px_units
				var t := clampf(((cx - ax) * dx + (cy - ay) * dy) / len2, 0.0, 1.0)
				var qx := cx - ax - dx * t
				var qy := cy - ay - dy * t
				var d := sqrt(qx * qx + qy * qy) * meters_per_unit
				var half := lerpf(half_a, half_b, t)
				var bank := lerpf(bank_a, bank_b, t)
				if d >= half + bank:
					continue
				if not covers.is_empty() and _covered(cx, cy):
					continue
				var z := lerpf(za, zb, t)
				var o := (j * side + i) * 2
				var code := out.decode_u16(o)
				var h := h_min + code / to_code
				var target: float
				if d <= half:
					var wet := maxf(half * 2.0, 1.0)
					var depth := minf(FineBedCarver.DEPTH_BASE + FineBedCarver.DEPTH_PER_M * wet, FineBedCarver.DEPTH_MAX)
					var r := d / half
					target = z - FineBedCarver.EDGE_DEPTH - (depth - FineBedCarver.EDGE_DEPTH) * (1.0 - r * r)
				else:
					var u := (d - half) / bank
					u = u * u * (3.0 - 2.0 * u)
					target = lerpf(z - FineBedCarver.EDGE_DEPTH, h, u)
				if target >= h:
					continue
				out.encode_u16(o, clampi(int(round((target - h_min) * to_code)), 0, 65535))
				carved_px += 1

	func _covered(cx: float, cy: float) -> bool:
		for c in covers:
			var ddx := cx - c.x
			var ddy := cy - c.y
			if ddx * ddx + ddy * ddy < c.z * c.z:
				return true
		return false
