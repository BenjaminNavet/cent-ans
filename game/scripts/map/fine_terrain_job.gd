class_name FineTerrainJob
extends RefCounted

## Construction hors fil principal (lot C6) du maillage « relief fin » d'une tuile de terrain à
## partir d'une tuile de relief 8192² (`data/map/height/h_{col}_{row}.png`, 512² pixels = une
## tuile de 256 unités monde, alignée sur le découpage 16 × 16 de `TerrainBuilder`).
##
## - Sommets tous les `step` pixels 8192 (0,5 unité monde pour `step` = 1) ; un sommet en X
##   (coordonnée carte 4096, convention « sommet = centre de pixel ») lit le pixel 8192 à
##   2 X + 0,5, soit la moyenne des 2 × 2 pixels voisins.
## - Bords : les sommets du pourtour reprennent le profil du LOD proche 4096 (pas `edge_step`,
##   interpolation linéaire le long du bord) : aucune fissure avec une tuile voisine proche ou
##   fine. Une jupe (`skirt_depth`) masque en plus les écarts avec une voisine lointaine.
## - Résultats : `vertices` (dont jupe), `heights` (grille `side` × `side` des hauteurs monde de
##   la surface affichée, pour poser les objets), sans accès à l'arbre de scène.

var tile_index: int = 0
var origin_px: Vector2i = Vector2i.ZERO  # coin nord-ouest (coordonnées carte 4096)
var chunk_px: int = 256
var step: int = 1
## Tuile 8192 : 2 octets par pixel, `tile_side` × `tile_side`.
var tile_bytes: PackedByteArray = PackedByteArray()
var tile_side: int = 512
var little_endian: bool = true
## Hauteurs normalisées → monde.
var h_min: float = -200.0
var h_range: float = 5000.0
var height_scale: float = 0.006
## Profil des bords : heightmap 4096 (MapData).
var map_bytes: PackedByteArray = PackedByteArray()
var map_bpp: int = 2
var map_little_endian: bool = true
var map_size: Vector2i = Vector2i(4096, 4096)
var edge_step: int = 4
var skirt_depth: float = 1.5

var vertices: PackedVector3Array = PackedVector3Array()
var heights: PackedFloat32Array = PackedFloat32Array()
var side: int = 0
var build_ms: float = 0.0
var ok: bool = false


func run() -> void:
	var t0 := Time.get_ticks_usec()
	var quads := tile_side / step
	side = quads + 1
	var unit := float(chunk_px) / float(quads)
	heights.resize(side * side)
	var k := 0
	for j in side:
		var sy := mini(j * step, tile_side - 1)
		var sy1 := mini(sy + 1, tile_side - 1)
		for i in side:
			var sx := mini(i * step, tile_side - 1)
			var sx1 := mini(sx + 1, tile_side - 1)
			var v := (_sample(sx, sy) + _sample(sx1, sy) + _sample(sx, sy1) + _sample(sx1, sy1)) * 0.25
			heights[k] = (h_min + v * h_range) * height_scale
			k += 1
	# Pourtour : profil du LOD proche 4096.
	for i in side:
		var t := i * unit
		heights[i] = _edge_height(float(origin_px.x) + t, float(origin_px.y))
		heights[(side - 1) * side + i] = _edge_height(float(origin_px.x) + t, float(origin_px.y + chunk_px))
		heights[i * side] = _edge_height(float(origin_px.x), float(origin_px.y) + t)
		heights[i * side + side - 1] = _edge_height(float(origin_px.x + chunk_px), float(origin_px.y) + t)
	vertices.resize(side * side + 4 * side)
	k = 0
	for j in side:
		for i in side:
			vertices[k] = Vector3(i * unit, heights[k], j * unit)
			k += 1
	# Jupe : copie de chaque bord abaissée de `skirt_depth` (nord, sud, ouest, est).
	var base := side * side
	for i in side:
		vertices[base + i] = vertices[i] - Vector3(0.0, skirt_depth, 0.0)
		vertices[base + side + i] = vertices[(side - 1) * side + i] - Vector3(0.0, skirt_depth, 0.0)
		vertices[base + 2 * side + i] = vertices[i * side] - Vector3(0.0, skirt_depth, 0.0)
		vertices[base + 3 * side + i] = vertices[i * side + side - 1] - Vector3(0.0, skirt_depth, 0.0)
	build_ms = (Time.get_ticks_usec() - t0) / 1000.0
	ok = true


func _sample(x: int, y: int) -> float:
	var o := (y * tile_side + x) * 2
	if little_endian:
		return float(tile_bytes[o] | (tile_bytes[o + 1] << 8)) / 65535.0
	return float((tile_bytes[o] << 8) | tile_bytes[o + 1]) / 65535.0


func _map_height(px: int, py: int) -> float:
	px = clampi(px, 0, map_size.x - 1)
	py = clampi(py, 0, map_size.y - 1)
	var o := py * map_size.x + px
	var v: float
	if map_bpp == 2:
		o *= 2
		if map_little_endian:
			v = float(map_bytes[o] | (map_bytes[o + 1] << 8)) / 65535.0
		else:
			v = float((map_bytes[o] << 8) | map_bytes[o + 1]) / 65535.0
	else:
		v = float(map_bytes[o]) / 255.0
	return (h_min + v * h_range) * height_scale


## Hauteur du LOD proche 4096 sur un bord de tuile (x ou y multiple de `edge_step`).
func _edge_height(x: float, y: float) -> float:
	var gx := x / edge_step
	var gy := y / edge_step
	var x0 := int(floor(gx))
	var y0 := int(floor(gy))
	var tx := gx - x0
	var ty := gy - y0
	var a := _map_height(x0 * edge_step, y0 * edge_step)
	if tx > 0.0001:
		return lerpf(a, _map_height((x0 + 1) * edge_step, y0 * edge_step), tx)
	if ty > 0.0001:
		return lerpf(a, _map_height(x0 * edge_step, (y0 + 1) * edge_step), ty)
	return a


## Index partagés par toutes les tuiles fines de même `side` (grille + jupe).
static func build_indices(grid_side: int) -> PackedInt32Array:
	var quads := grid_side - 1
	var indices := PackedInt32Array()
	indices.resize(quads * quads * 6 + 4 * quads * 12)
	var k := 0
	for j in quads:
		for i in quads:
			var a := j * grid_side + i
			var b := a + 1
			var c := a + grid_side
			var d := c + 1
			indices[k] = a
			indices[k + 1] = b
			indices[k + 2] = d
			indices[k + 3] = a
			indices[k + 4] = d
			indices[k + 5] = c
			k += 6
	var base := grid_side * grid_side
	# Faces de jupe dans les deux sens (visibles de part et d'autre de la couture).
	for i in quads:
		# Nord (vue depuis -Z) : bord j = 0.
		k = _skirt_quad(indices, k, i + 1, i, base + i + 1, base + i)
		# Sud (vue depuis +Z).
		var s0 := (grid_side - 1) * grid_side + i
		k = _skirt_quad(indices, k, s0, s0 + 1, base + grid_side + i, base + grid_side + i + 1)
		# Ouest (vue depuis -X).
		k = _skirt_quad(indices, k, i * grid_side, (i + 1) * grid_side, base + 2 * grid_side + i, base + 2 * grid_side + i + 1)
		# Est (vue depuis +X).
		var e0 := i * grid_side + grid_side - 1
		k = _skirt_quad(indices, k, e0 + grid_side, e0, base + 3 * grid_side + i + 1, base + 3 * grid_side + i)
	return indices


## Quad de jupe : (top_a, top_b) en haut, (low_a, low_b) en dessous, deux faces.
static func _skirt_quad(indices: PackedInt32Array, k: int, top_a: int, top_b: int, low_a: int, low_b: int) -> int:
	var order := [top_a, top_b, low_b, top_a, low_b, low_a, top_b, top_a, low_b, low_b, top_a, low_a]
	for n in 12:
		indices[k + n] = order[n]
	return k + 12
