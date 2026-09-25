class_name FineRibbonJob
extends RefCounted

## Maillage des rubans d'une tuile E2 (lot ZG5b) : fleuves fins et routes drapées, construit
## dans un fil de travail (`run`) puis installé sur le fil principal (`FineGeoLayer`).
##
## Hauteurs en **mètres** dans `VERTEX.y` : les shaders multiplient par l'échelle verticale
## courante (`height_scale`), donc un changement d'exagération (lot ZG4) ne demande aucun
## remaillage. Surface sous le ruban : instantané des pages chargées (`surface_snapshot`), relu
## quand les pages de la tuile changent.
##
## Fleuves : sommets doublés sur l'axe (le shader écarte de la demi-largeur, au moins un pixel
## écran) ; NORMAL = perpendiculaire signée ; UV = (largeur monde, côté ±1) ; UV2 = (abscisse
## vers l'aval, ordre de Strahler) ; COLOR = (divagant, marée, marais, intermittent).
## Hauteur : niveau d'eau `z`, relevé au-dessus de la surface là où le lit n'est pas creusé.
## Routes : même disposition ; UV2 = (abscisse, largeur réelle m) ; COLOR = (principale,
## chaussée en zone humide, pavée, calculée). Hauteur : max(surface, `z` de la tuile) + soulèvement.

## Soulèvement (m) de l'eau au-dessus de la surface là où le lit n'est pas creusé, et des routes.
const RIVER_LIFT_M := 0.35
const ROAD_LIFT_M := 0.25
## Pas maximal (unités monde) des routes densifiées : elles suivent les facettes du relief.
const ROAD_STEP := 0.04

# Entrées (fil principal)
var key: int = 0
var river_tile: CafvTile
var road_tile: CafvTile
## Instantané `ReliefQuadtree.surface_snapshot` (ou vide) et échelle verticale de ses hauteurs.
var snapshot: Dictionary = {}
var snapshot_scale: float = 0.006
var covers: PackedVector4Array = PackedVector4Array()
## Zones personnalisées (x, y, rayon) : rien dedans.
var zones: PackedVector3Array = PackedVector3Array()
## Villes et cités (x, y, rayon des rues pavées).
var towns: PackedVector3Array = PackedVector3Array()
var meters_per_unit: float = 719.0
var min_order: int = 3

# Sorties (fil de travail)
var river_arrays: Array = []
var road_arrays: Array = []
var river_aabb: AABB
var road_aabb: AABB
## Ponts-portes : {px: Vector2, dir: Vector2, width: float (unités), cover: int}.
var gates: Array[Dictionary] = []
var river_points: int = 0
var road_points: int = 0
var build_ms: float = 0.0


func run() -> void:
	var t0 := Time.get_ticks_usec()
	if river_tile != null:
		_build_rivers()
	if road_tile != null:
		_build_roads()
	build_ms = (Time.get_ticks_usec() - t0) / 1000.0


## Surface (m) sous un point, NAN hors pages.
func _surface_m(px: float, py: float) -> float:
	if snapshot.is_empty():
		return NAN
	var pages: Dictionary = snapshot["qt_pages"]
	var h := ReliefQuadtree.sample_pages(pages, snapshot["max_level"], snapshot["h_min"], snapshot["h_range"], px, py)
	return h / snapshot_scale if not is_nan(h) else NAN


func _cover_at(p: Vector2) -> int:
	for k in covers.size():
		var c := covers[k]
		if Vector2(c.x, c.y).distance_squared_to(p) < c.z * c.z:
			return k
	return -1


func _in_zone(p: Vector2) -> bool:
	for zone in zones:
		if Vector2(zone.x, zone.y).distance_squared_to(p) < zone.z * zone.z:
			return true
	return false


# --- Fleuves --------------------------------------------------------------------------


func _build_rivers() -> void:
	var out := _Arrays.new()
	var tile := river_tile
	for li in tile.lines():
		var flags := tile.line_flags[li]
		var rank := tile.line_rank[li]
		if rank < min_order:
			continue
		var color := Color(
			1.0 if flags & CafvTile.FLAG_DIVAGATING else 0.0,
			1.0 if flags & CafvTile.FLAG_TIDAL else 0.0,
			1.0 if flags & CafvTile.FLAG_WETLAND else 0.0,
			1.0 if flags & CafvTile.FLAG_INTERMITTENT else 0.0)
		var s := tile.line_start[li]
		var n := tile.line_count[li]
		# Morceaux hors des emprises des colonies et des zones personnalisées, coupés sur le bord
		# de l'emprise (l'eau passe sous la ville ; pont-porte au point de coupe).
		var piece := _Piece.new()
		var prev_blocked := false
		var prev_cover := -1
		for k in range(s, s + n):
			var p := Vector2(tile.x[k], tile.y[k])
			var cover := _cover_at(p)
			var blocked := cover >= 0 or (not zones.is_empty() and _in_zone(p))
			if k > s and blocked != prev_blocked:
				var inside_k := k if blocked else k - 1
				var outside_k := k - 1 if blocked else k
				var c := cover if blocked else prev_cover
				var t := _cut_t(tile, outside_k, inside_k, c)
				var cut := _Piece.lerp_point(tile, outside_k, inside_k, t)
				if blocked:
					piece.add_raw(cut)
					if c >= 0:
						_add_gate(tile, outside_k, inside_k, c, cut)
					_river_piece(out, piece, rank, color)
					piece = _Piece.new()
				else:
					piece.add_raw(cut)
					if c >= 0:
						_add_gate(tile, outside_k, inside_k, c, cut)
			if not blocked:
				piece.add(tile, k)
			prev_blocked = blocked
			prev_cover = cover
		_river_piece(out, piece, rank, color)
	river_arrays = out.commit()
	river_aabb = out.aabb()
	river_points = out.vertices.size() / 2


## Paramètre t du point de [outside → inside] sur le bord de l'emprise (dichotomie ; 1 pour une
## zone personnalisée : coupe au dernier point dehors).
func _cut_t(tile: CafvTile, outside: int, inside: int, cover: int) -> float:
	if cover < 0:
		return 0.0
	var c := covers[cover]
	var a := Vector2(tile.x[outside], tile.y[outside])
	var b := Vector2(tile.x[inside], tile.y[inside])
	var lo := 0.0
	var hi := 1.0
	var center := Vector2(c.x, c.y)
	for _i in 16:
		var m := (lo + hi) * 0.5
		if a.lerp(b, m).distance_squared_to(center) < c.z * c.z:
			hi = m
		else:
			lo = m
	return lo


func _river_piece(out: _Arrays, piece: _Piece, rank: int, color: Color) -> void:
	var count := piece.pts.size()
	if count < 2:
		return
	var base := out.vertices.size()
	var along := 0.0
	for i in count:
		var p := piece.pts[i]
		if i > 0:
			along += p.distance_to(piece.pts[i - 1])
		var dir := (piece.pts[mini(i + 1, count - 1)] - piece.pts[maxi(i - 1, 0)]).normalized()
		if dir == Vector2.ZERO:
			dir = Vector2.RIGHT
		var perp := Vector3(-dir.y, 0.0, dir.x)
		var h := piece.zs[i]
		var ground := _surface_m(p.x, p.y)
		if not is_nan(ground):
			h = maxf(h, ground + RIVER_LIFT_M)
		var width := piece.ws[i] / meters_per_unit
		for side in [1.0, -1.0]:
			out.vertices.append(Vector3(p.x, h, p.y))
			out.normals.append(perp * side)
			out.uvs.append(Vector2(width, side))
			out.uv2s.append(Vector2(along, rank))
			out.colors.append(color)
		out.grow(p, h, width * 1.5)
	for i in count - 1:
		var a := base + i * 2
		out.indices.append_array([a, a + 1, a + 3, a, a + 3, a + 2])


## Pont-porte au point `cut` où l'axe du fleuve coupe le bord de l'emprise `cover`.
func _add_gate(tile: CafvTile, outside: int, inside: int, cover: int, cut: Vector4) -> void:
	var a := Vector2(tile.x[outside], tile.y[outside])
	var b := Vector2(tile.x[inside], tile.y[inside])
	# Sens du courant : indices croissants vers l'aval.
	var dir := (b - a).normalized() if outside < inside else (a - b).normalized()
	gates.append({"px": Vector2(cut.x, cut.y), "dir": dir, "width": cut.w / meters_per_unit, "cover": int(covers[cover].w), "z": cut.z})


## Morceau de ligne : points, niveaux et largeurs (coupes interpolées comprises).
class _Piece:
	extends RefCounted

	var pts := PackedVector2Array()
	var zs := PackedFloat32Array()
	var ws := PackedFloat32Array()

	func add(tile: CafvTile, k: int) -> void:
		pts.append(Vector2(tile.x[k], tile.y[k]))
		zs.append(tile.z[k])
		ws.append(tile.w[k])

	## Point (x, y, z, w).
	func add_raw(p: Vector4) -> void:
		pts.append(Vector2(p.x, p.y))
		zs.append(p.z)
		ws.append(p.w)

	static func lerp_point(tile: CafvTile, a: int, b: int, t: float) -> Vector4:
		return Vector4(lerpf(tile.x[a], tile.x[b], t), lerpf(tile.y[a], tile.y[b], t), lerpf(tile.z[a], tile.z[b], t), lerpf(tile.w[a], tile.w[b], t))


# --- Routes ---------------------------------------------------------------------------


func _build_roads() -> void:
	var out := _Arrays.new()
	var tile := road_tile
	for li in tile.lines():
		var flags := tile.line_flags[li]
		var main := (flags & CafvTile.FLAG_MAIN_ROAD) != 0
		var computed := (flags & CafvTile.FLAG_COMPUTED_ROAD) != 0
		var causeway := (flags & CafvTile.FLAG_WETLAND) != 0
		var s := tile.line_start[li]
		var n := tile.line_count[li]
		if n < 2:
			continue
		# Densification : un point tous les ROAD_STEP (le ruban suit le relief affiché).
		var pts := PackedVector2Array()
		var zs := PackedFloat32Array()
		var ws := PackedFloat32Array()
		for k in range(s, s + n):
			var p := Vector2(tile.x[k], tile.y[k])
			if k > s:
				var q := pts[pts.size() - 1]
				var steps := int(ceil(q.distance_to(p) / ROAD_STEP))
				for m in range(1, steps):
					var t := float(m) / steps
					pts.append(q.lerp(p, t))
					zs.append(lerpf(tile.z[k - 1], tile.z[k], t))
					ws.append(lerpf(tile.w[k - 1], tile.w[k], t))
			pts.append(p)
			zs.append(tile.z[k])
			ws.append(tile.w[k])
		_road_piece(out, pts, zs, ws, Color(1.0 if main else 0.0, 1.0 if causeway else 0.0, 0.0, 1.0 if computed else 0.0))
	road_arrays = out.commit()
	road_aabb = out.aabb()
	road_points = out.vertices.size() / 2


func _road_piece(out: _Arrays, pts: PackedVector2Array, zs: PackedFloat32Array, ws: PackedFloat32Array, color: Color) -> void:
	var count := pts.size()
	var base := out.vertices.size()
	var along := 0.0
	for i in count:
		var p := pts[i]
		if i > 0:
			along += p.distance_to(pts[i - 1])
		var dir := (pts[mini(i + 1, count - 1)] - pts[maxi(i - 1, 0)]).normalized()
		if dir == Vector2.ZERO:
			dir = Vector2.RIGHT
		var perp := Vector3(-dir.y, 0.0, dir.x)
		var ground := _surface_m(p.x, p.y)
		var h := zs[i] if is_nan(ground) else maxf(ground, zs[i])
		h += ROAD_LIFT_M
		var c := color
		c.b = _paved(p)
		for side in [1.0, -1.0]:
			out.vertices.append(Vector3(p.x, h, p.y))
			out.normals.append(perp * side)
			out.uvs.append(Vector2(ws[i] / meters_per_unit, side))
			out.uv2s.append(Vector2(along, ws[i]))
			out.colors.append(c)
		out.grow(p, h, 0.2)
	for i in count - 1:
		var a := base + i * 2
		out.indices.append_array([a, a + 1, a + 3, a, a + 3, a + 2])


## Rue pavée dans et autour des villes (1 au cœur, fondu jusqu'au rayon).
func _paved(p: Vector2) -> float:
	var best := 0.0
	for t in towns:
		var d := Vector2(t.x, t.y).distance_to(p)
		if d < t.z:
			best = maxf(best, 1.0 - smoothstep(t.z * 0.6, t.z, d))
	return best


## Tableaux d'un maillage de rubans et leur boîte (y en mètres × 0,006, échelle maximale).
class _Arrays:
	extends RefCounted

	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var uvs := PackedVector2Array()
	var uv2s := PackedVector2Array()
	var colors := PackedColorArray()
	var indices := PackedInt32Array()
	var lo := Vector3(INF, INF, INF)
	var hi := Vector3(-INF, -INF, -INF)

	func grow(p: Vector2, h_m: float, margin: float) -> void:
		lo = Vector3(minf(lo.x, p.x - margin), minf(lo.y, h_m), minf(lo.z, p.y - margin))
		hi = Vector3(maxf(hi.x, p.x + margin), maxf(hi.y, h_m), maxf(hi.z, p.y + margin))

	func aabb() -> AABB:
		if vertices.is_empty():
			return AABB()
		# Hauteurs en mètres : boîte valable pour toute échelle verticale ≤ HEIGHT_SCALE.
		var y0 := minf(lo.y * MapData.HEIGHT_SCALE, 0.0) - 1.0
		var y1 := maxf(hi.y * MapData.HEIGHT_SCALE, 0.0) + 1.0
		return AABB(Vector3(lo.x, y0, lo.z), Vector3(hi.x - lo.x, y1 - y0, hi.z - lo.z))

	func commit() -> Array:
		if vertices.is_empty():
			return []
		var arrays := []
		arrays.resize(Mesh.ARRAY_MAX)
		arrays[Mesh.ARRAY_VERTEX] = vertices
		arrays[Mesh.ARRAY_NORMAL] = normals
		arrays[Mesh.ARRAY_TEX_UV] = uvs
		arrays[Mesh.ARRAY_TEX_UV2] = uv2s
		arrays[Mesh.ARRAY_COLOR] = colors
		arrays[Mesh.ARRAY_INDEX] = indices
		return arrays
