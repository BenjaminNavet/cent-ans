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
## - Résultats : `block_vertices` (blocs avec jupe), `block_errors`, `heights` (grille `side` × `side` des hauteurs monde de
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

## PF1 : la tuile est découpée en blocs de `block_quads` × `block_quads` quads (un
## `MeshInstance3D` chacun : les blocs hors champ ou hors cascade d'ombre sont écartés), chacun
## avec ses niveaux de détail (pas × 2, × 4, × 8 : `LOD_STRIDES`). Clé de LOD de Godot (unités
## monde ; un niveau est pris quand sa clé fait moins de `mesh_lod_threshold` ≈ 1 pixel à
## l'écran) : le plus petit de (a) l'erreur de hauteur maximale du niveau et (b) l'arête du
## niveau plus fin divisée par `lod_triangle_px`. Un niveau plus grossier est donc pris dès que
## son erreur passe sous le pixel, ou dès que les triangles du niveau plus fin font moins de
## `lod_triangle_px` pixels : des triangles de 1 à 3 pixels ne montrent pas plus de relief (les
## normales viennent de la heightmap, dans le shader) mais font ombrer chaque pixel plusieurs
## fois (quads 2 × 2, MSAA) : c'est l'essentiel du coût du relief fin au zoom comté.
const LOD_STRIDES: Array[int] = [2, 4, 8]
var lod_triangle_px: float = 6.0
var block_quads: int = 64
var blocks_per_side: int = 0
## Par bloc (ligne par ligne) : sommets (grille `block_quads + 1` au carré puis jupe) et erreurs.
var block_vertices: Array[PackedVector3Array] = []
var block_errors: Array[PackedFloat32Array] = []
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
	_build_blocks(unit)
	build_ms = (Time.get_ticks_usec() - t0) / 1000.0
	ok = true


func _build_blocks(unit: float) -> void:
	var quads := side - 1
	var n := mini(block_quads, quads)
	block_quads = n
	blocks_per_side = quads / n
	var bside := n + 1
	var down := Vector3(0.0, skirt_depth, 0.0)
	block_vertices.clear()
	block_errors.clear()
	for by in blocks_per_side:
		for bx in blocks_per_side:
			var i0 := bx * n
			var j0 := by * n
			var verts := PackedVector3Array()
			verts.resize(bside * bside + 4 * bside)
			var local := PackedFloat32Array()
			local.resize(bside * bside)
			var k := 0
			for j in bside:
				var row := (j0 + j) * side + i0
				for i in bside:
					var h := heights[row + i]
					local[k] = h
					verts[k] = Vector3((i0 + i) * unit, h, (j0 + j) * unit)
					k += 1
			var base := bside * bside
			for i in bside:
				verts[base + i] = verts[i] - down
				verts[base + bside + i] = verts[(bside - 1) * bside + i] - down
				verts[base + 2 * bside + i] = verts[i * bside] - down
				verts[base + 3 * bside + i] = verts[i * bside + bside - 1] - down
			block_vertices.append(verts)
			var errors := PackedFloat32Array()
			var previous := 0.0
			var finer_edge := unit
			for stride in LOD_STRIDES:
				var key := minf(lod_error(local, bside, stride), finer_edge / lod_triangle_px)
				finer_edge = stride * unit
				# Clés strictement croissantes (exigence de Godot), jamais nulles.
				var e := maxf(key, previous * 1.05 + 0.0005)
				errors.append(e)
				previous = e
			block_errors.append(errors)


## Écart de hauteur maximal entre la grille `bside` × `bside` et sa version au pas `stride`
## (mêmes triangles que `block_indices` : diagonale a-d).
static func lod_error(h: PackedFloat32Array, bside: int, stride: int) -> float:
	var worst := 0.0
	var n := bside - 1
	if stride > n:
		return INF
	for j in bside:
		var j0 := mini((j / stride) * stride, n - stride)
		var ty := float(j - j0) / stride
		for i in bside:
			var i0 := mini((i / stride) * stride, n - stride)
			var tx := float(i - i0) / stride
			var ha := h[j0 * bside + i0]
			var hb := h[j0 * bside + i0 + stride]
			var hc := h[(j0 + stride) * bside + i0]
			var hd := h[(j0 + stride) * bside + i0 + stride]
			var v: float
			if tx >= ty:
				v = ha + tx * (hb - ha) + ty * (hd - hb)
			else:
				v = ha + ty * (hc - ha) + tx * (hd - hc)
			worst = maxf(worst, absf(v - h[j * bside + i]))
	return worst


## Index d'un bloc de `n` quads au pas `stride` (grille + jupe), partagés par tous les blocs.
static func block_indices(n: int, stride: int) -> PackedInt32Array:
	var bside := n + 1
	var cells := n / stride
	var indices := PackedInt32Array()
	indices.resize(cells * cells * 6 + 4 * cells * 12)
	var k := 0
	for cj in cells:
		for ci in cells:
			var a := cj * stride * bside + ci * stride
			var b := a + stride
			var c := a + stride * bside
			var d := c + stride
			indices[k] = a
			indices[k + 1] = b
			indices[k + 2] = d
			indices[k + 3] = a
			indices[k + 4] = d
			indices[k + 5] = c
			k += 6
	var base := bside * bside
	for ci in cells:
		var i := ci * stride
		var i1 := i + stride
		k = _skirt_quad(indices, k, i1, i, base + i1, base + i)
		var s0 := (bside - 1) * bside
		k = _skirt_quad(indices, k, s0 + i, s0 + i1, base + bside + i, base + bside + i1)
		k = _skirt_quad(indices, k, i * bside, i1 * bside, base + 2 * bside + i, base + 2 * bside + i1)
		k = _skirt_quad(indices, k, i1 * bside + bside - 1, i * bside + bside - 1, base + 3 * bside + i1, base + 3 * bside + i)
	return indices


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


## Quad de jupe : (top_a, top_b) en haut, (low_a, low_b) en dessous, deux faces.
static func _skirt_quad(indices: PackedInt32Array, k: int, top_a: int, top_b: int, low_a: int, low_b: int) -> int:
	var order := [top_a, top_b, low_b, top_a, low_b, low_a, top_b, top_a, low_b, low_b, top_a, low_a]
	for n in 12:
		indices[k + n] = order[n]
	return k + 12
