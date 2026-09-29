class_name CafvTile
extends RefCounted

## Tuile binaire CAFV v1 (lot ZG5a, `docs/geo.md` § « Hydrographie fine ») : lignes du réseau
## fin (couche 1, fleuves) ou des routes drapées (couche 2) d'une tuile E2 (64 unités monde).
## Lecture seule, rendu seulement (lot ZG5b). Une fois lue, la tuile n'est plus modifiée :
## lisible depuis un fil de travail (lit creusé des pages, maillage des rubans).
##
## En-tête petit-boutiste `4s H H H H I I I I` (28 octets : la table de `docs/geo.md` annonce
## 32 o, le code Python `fine_tiles.HEADER` en écrit 28), puis `u32 × 4` par ligne (entité,
## premier sommet, nombre de sommets, drapeaux), puis `x`, `y`, `z`, `w` en `f32 × n`.

const MAGIC := "CAFV"
const HEADER_BYTES := 28
const LAYER_RIVERS := 1
const LAYER_ROADS := 2
## Côté d'une tuile E2 en unités monde.
const TILE_UNITS := 64.0

## Drapeaux (`rivers_fine.json` → `flags`).
const FLAG_DIVAGATING := 1
const FLAG_TIDAL := 2
const FLAG_INTERMITTENT := 4
const FLAG_RECTIFIED := 8
const FLAG_COARSE := 16
const FLAG_WETLAND := 32  # fleuve : marais ; route : chaussée en zone humide
const FLAG_MAIN_ROAD := 64
const FLAG_COMPUTED_ROAD := 128
const ORDER_SHIFT := 24
const SOURCE_SHIFT := 28

var layer: int = 0
var level: int = 0
var col: int = 0
var row: int = 0
## Par ligne : entité, premier sommet, nombre de sommets, drapeaux.
var line_feature: PackedInt32Array = PackedInt32Array()
var line_start: PackedInt32Array = PackedInt32Array()
var line_count: PackedInt32Array = PackedInt32Array()
var line_flags: PackedInt64Array = PackedInt64Array()
## Emprise de chaque ligne (x0, y0, x1, y1), unités monde.
var line_bounds: PackedVector4Array = PackedVector4Array()
## Rang d'affichage : max(ordre de Strahler, ordre équivalent à la largeur maximale de la ligne).
## L'ordre calculé par ZG5a sous-estime les grands fleuves (Seine à Rouen : ordre 4, 1 000 m).
var line_rank: PackedInt32Array = PackedInt32Array()
var x: PackedFloat32Array = PackedFloat32Array()
var y: PackedFloat32Array = PackedFloat32Array()
## Niveau d'eau ou surface de route (m, non exagéré) et largeur (m).
var z: PackedFloat32Array = PackedFloat32Array()
var w: PackedFloat32Array = PackedFloat32Array()


func lines() -> int:
	return line_start.size()


func points() -> int:
	return x.size()


## Ordre de Strahler équivalent à une largeur (`river_widths.json` → `strahler_width_m`).
static func rank_of_width(width_m: float) -> int:
	if width_m >= 220.0:
		return 9
	if width_m >= 120.0:
		return 8
	if width_m >= 60.0:
		return 7
	if width_m >= 30.0:
		return 6
	if width_m >= 16.0:
		return 5
	if width_m >= 9.0:
		return 4
	return 3 if width_m >= 5.0 else 2


static func order_of(flags: int) -> int:
	return (flags >> ORDER_SHIFT) & 0xF


static func source_of(flags: int) -> int:
	return (flags >> SOURCE_SHIFT) & 0xF


## Lit une tuile (null si absente ou illisible). Sûr dans un fil de travail.
## `offset_tiles` : décalage d'origine du cache en tuiles E2 (`root_origin_tiles` × 4, ADR 0115) ;
## les points et l'en-tête passent du cadre du cache au cadre monde.
static func load_file(path: String, offset_tiles: Vector2i = Vector2i.ZERO) -> CafvTile:
	if not FileAccess.file_exists(path):
		return null
	return parse(FileAccess.get_file_as_bytes(path), offset_tiles)


## Décode les octets d'une tuile (null si le format est inattendu). `offset_tiles` : voir
## `load_file` (unités monde ajoutées = `offset_tiles` × 64).
static func parse(bytes: PackedByteArray, offset_tiles: Vector2i = Vector2i.ZERO) -> CafvTile:
	if bytes.size() < HEADER_BYTES or bytes.slice(0, 4).get_string_from_ascii() != MAGIC:
		return null
	if bytes.decode_u16(4) != 1:
		return null
	var tile := CafvTile.new()
	tile.layer = bytes.decode_u16(6)
	tile.level = bytes.decode_u16(8)
	tile.col = bytes.decode_u32(12)
	tile.row = bytes.decode_u32(16)
	var n_lines := bytes.decode_u32(20)
	var n_points := bytes.decode_u32(24)
	var expected := HEADER_BYTES + n_lines * 16 + n_points * 16
	if bytes.size() < expected:
		return null
	var table := bytes.slice(HEADER_BYTES, HEADER_BYTES + n_lines * 16).to_int32_array()
	tile.line_feature.resize(n_lines)
	tile.line_start.resize(n_lines)
	tile.line_count.resize(n_lines)
	tile.line_flags.resize(n_lines)
	for i in n_lines:
		tile.line_feature[i] = table[i * 4]
		tile.line_start[i] = table[i * 4 + 1]
		tile.line_count[i] = table[i * 4 + 2]
		# Drapeaux non signés (bits 28-31 : source).
		tile.line_flags[i] = table[i * 4 + 3] & 0xFFFFFFFF
	var offset := HEADER_BYTES + n_lines * 16
	var span := n_points * 4
	tile.x = bytes.slice(offset, offset + span).to_float32_array()
	tile.y = bytes.slice(offset + span, offset + span * 2).to_float32_array()
	tile.z = bytes.slice(offset + span * 2, offset + span * 3).to_float32_array()
	tile.w = bytes.slice(offset + span * 3, offset + span * 4).to_float32_array()
	if offset_tiles != Vector2i.ZERO:
		tile.col += offset_tiles.x
		tile.row += offset_tiles.y
		var dx := float(offset_tiles.x) * TILE_UNITS
		var dy := float(offset_tiles.y) * TILE_UNITS
		for k in n_points:
			tile.x[k] += dx
			tile.y[k] += dy
	tile.line_bounds.resize(n_lines)
	tile.line_rank.resize(n_lines)
	for i in n_lines:
		var s := tile.line_start[i]
		var e := mini(s + tile.line_count[i], n_points)
		var b := Vector4(INF, INF, -INF, -INF)
		var widest := 0.0
		for k in range(s, e):
			b.x = minf(b.x, tile.x[k])
			b.y = minf(b.y, tile.y[k])
			b.z = maxf(b.z, tile.x[k])
			b.w = maxf(b.w, tile.y[k])
			widest = maxf(widest, tile.w[k])
		tile.line_bounds[i] = b
		tile.line_rank[i] = maxi(order_of(tile.line_flags[i]), rank_of_width(widest)) if tile.layer == LAYER_RIVERS else 9
	return tile
