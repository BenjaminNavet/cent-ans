class_name ReliefPyramid
extends RefCounted

## Pyramide de relief streamée (chantier ZG, ADR 0036) : lit `data/map/relief_pyramid.json` et
## localise les tuiles 512² 16 bits de `data/map/pyramid/E{k}/{col}_{row}.png` (hors git).
## Étage k : une tuile couvre `256 / 2^k` unités monde. E0 = `data/map/height/` (map.json
## `height_tiles`, versionné). Sans cache (aucune tuile E1+ sur disque), `is_available()` est
## faux et la carte se comporte comme avant (relief fin `FineTerrainJob`).
##
## Géoréférencement (ADR 0086, lot SZ2b ; même convention que les outils `geo` et toutes les
## données vectorielles, x = (E − minx) / m) : le pixel global j de l'étage k (taille
## `0,5 / 2^k` unité) est centré en `(j + 0,5) × taille` ; la tuile (k, col, row) couvre
## `[col × T, (col + 1) × T]` avec T = `tile_units(k)`. Avant SZ2b, la grille était décalée de
## −0,5 unité (pixel de la heightmap centré en x = i) : relief affiché 360 m au nord-ouest des
## fleuves fins, colonies et villes 1:1.
## Rendu seulement : aucune règle de jeu ne lit la pyramide.

const TILE_PX := 512
const ROOT_TILE_UNITS := 256.0
## Décalage de la grille des tuiles par rapport aux coordonnées carte (0 depuis SZ2b, ADR 0086).
const GRID_OFFSET := 0.0
const MAX_LEVEL := 7

var map_dir: String = ""
## Dossier des tuiles E1+ (`<map_dir>/pyramid` par défaut, ou `--pyramid-dir=` pour les essais).
var tiles_dir: String = ""
var pattern: String = "E{level}/{col}_{row}.png"
var e0_dir: String = ""
var e0_pattern: String = "h_{col}_{row}.png"
var height_min_m: float = -200.0
var height_range_m: float = 5000.0
## Étage le plus fin présent (0 sans cache).
var max_level: int = 0
var load_error: String = ""
## Tuiles listées par le manifeste mais absentes du disque (cache partiel, ZG7c).
var missing_tiles: int = 0

## Par étage (index = étage) : ensemble des clés `row * cols + col` des tuiles présentes.
var _tiles: Array[Dictionary] = []
## Ancêtres : clé (étage, col, row) → étage le plus fin présent dans le sous-arbre de la tuile.
var _max_under: Dictionary = {}
## Tuiles déclarées mais illisibles (décodage raté) : jamais redemandées.
var _broken: Dictionary = {}
var _tile_count: int = 0


## Nombre de tuiles par côté à l'étage k (16 à E0).
static func tiles_per_side(level: int) -> int:
	return 16 << level


static func tile_units(level: int) -> float:
	return ROOT_TILE_UNITS / float(1 << level)


## Taille d'un pixel de l'étage k en unités monde (0,5 à E0).
static func pixel_units(level: int) -> float:
	return tile_units(level) / TILE_PX


## Clé entière d'une tuile (étage sur 3 bits, ligne et colonne sur 12 bits chacune).
static func key_of(level: int, col: int, row: int) -> int:
	return (level << 24) | (row << 12) | col


static func level_of_key(key: int) -> int:
	return key >> 24


static func col_of_key(key: int) -> int:
	return key & 0xfff


static func row_of_key(key: int) -> int:
	return (key >> 12) & 0xfff


## Coin nord-ouest (x, z) monde de la tuile.
static func tile_origin(level: int, col: int, row: int) -> Vector2:
	var t := tile_units(level)
	return Vector2(col * t + GRID_OFFSET, row * t + GRID_OFFSET)


## Tuile de l'étage k contenant le point carte (x, y) (hors bornes possibles).
static func tile_at(level: int, x: float, y: float) -> Vector2i:
	var t := tile_units(level)
	return Vector2i(int(floor((x - GRID_OFFSET) / t)), int(floor((y - GRID_OFFSET) / t)))


## Charge le manifeste `<dir>/relief_pyramid.json` (ou `manifest_path`) et, si `tiles_override`
## est non vide, lit les tuiles E1+ dans ce dossier. Faux si aucune tuile E1+ n'est utilisable.
func load_manifest(dir: String, manifest_path: String = "", tiles_override: String = "") -> bool:
	map_dir = dir
	max_level = 0
	_tiles.clear()
	_max_under.clear()
	_broken.clear()
	_tile_count = 0
	missing_tiles = 0
	for level in MAX_LEVEL + 1:
		_tiles.append({})
	_load_e0()
	var path := manifest_path if manifest_path != "" else dir.path_join("relief_pyramid.json")
	if not FileAccess.file_exists(path):
		load_error = "manifest missing: %s" % path
		return false
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not (parsed is Dictionary):
		load_error = "invalid manifest: %s" % path
		return false
	var manifest: Dictionary = parsed
	if int(manifest.get("tile_px", TILE_PX)) != TILE_PX or float(manifest.get("root_tile_units", ROOT_TILE_UNITS)) != ROOT_TILE_UNITS:
		load_error = "unsupported tile geometry in %s" % path
		return false
	height_min_m = float(manifest.get("height_min_m", height_min_m))
	height_range_m = float(manifest.get("height_range_m", height_range_m))
	pattern = str(manifest.get("pattern", pattern))
	if tiles_override != "":
		tiles_dir = tiles_override
	else:
		tiles_dir = path.get_base_dir().path_join(str(manifest.get("dir", "pyramid")))
	if not DirAccess.dir_exists_absolute(tiles_dir):
		load_error = "pyramid cache missing: %s" % tiles_dir
		return false
	for entry: Variant in manifest.get("levels", []):
		if not (entry is Dictionary):
			continue
		var level := int(entry.get("level", 0))
		if level < 1 or level > MAX_LEVEL:
			continue
		var set_: Dictionary = _tiles[level]
		var cols := tiles_per_side(level)
		for row_entry: Variant in entry.get("tiles_rle", []):
			var row := int(row_entry.get("row", -1))
			if row < 0 or row >= cols:
				continue
			for run: Variant in row_entry.get("runs", []):
				var start := int(run[0])
				var length := int(run[1])
				for col in range(maxi(start, 0), mini(start + length, cols)):
					set_[row * cols + col] = true
		if not set_.is_empty():
			# ZG7c : un manifeste en avance sur le cache (cuisson interrompue, cache partiel) ne garde
			# que les tuiles présentes sur disque ; les trous retombent tuile par tuile sur l'ancêtre
			# le plus fin présent (`finest_ancestor`). Un seul listage de dossier par étage.
			var missing := _drop_missing_tiles(level, set_)
			if missing > 0:
				missing_tiles += missing
				push_warning("ReliefPyramid: level %d, %d of %d listed tiles missing on disk (%s), ancestors used" % [level, missing, missing + set_.size(), tiles_dir])
			if set_.is_empty():
				continue
			max_level = maxi(max_level, level)
			_tile_count += set_.size()
	_index_ancestors()
	if max_level == 0:
		load_error = "no tile above E0"
	return max_level > 0


## Retire de `set_` (clés `row * cols + col` de l'étage) les tuiles absentes du disque ; rend leur
## nombre. Les fichiers du dossier de l'étage sont listés une fois (≈ 7 000 tuiles à E4) au lieu
## d'un `file_exists` par tuile.
func _drop_missing_tiles(level: int, set_: Dictionary) -> int:
	var cols := tiles_per_side(level)
	var relative := pattern.replace("{level}", str(level))
	var level_dir := tiles_dir.path_join(relative.get_base_dir())
	var file_pattern := relative.get_file()
	var present := {}
	if DirAccess.dir_exists_absolute(level_dir):
		for name in DirAccess.get_files_at(level_dir):
			present[name] = true
	var missing: Array[int] = []
	for key: int in set_:
		var name := file_pattern.replace("{col}", str(key % cols)).replace("{row}", str(key / cols))
		if not present.has(name):
			missing.append(key)
	for key in missing:
		set_.erase(key)
	return missing.size()


func is_available() -> bool:
	return max_level > 0


func tile_count() -> int:
	return _tile_count


func _load_e0() -> void:
	e0_dir = ""
	var meta_path := map_dir.path_join("map.json")
	if not FileAccess.file_exists(meta_path):
		return
	var meta: Variant = JSON.parse_string(FileAccess.get_file_as_string(meta_path))
	if not (meta is Dictionary) or not (meta as Dictionary).has("height_tiles"):
		return
	var tiles: Dictionary = meta["height_tiles"]
	if int(tiles.get("tile_px", 0)) != TILE_PX or int(tiles.get("size_px", 0)) != TILE_PX * 16:
		return
	var dir := map_dir.path_join(str(tiles.get("dir", "height")))
	if not DirAccess.dir_exists_absolute(dir):
		return
	e0_dir = dir
	e0_pattern = str(tiles.get("pattern", e0_pattern))
	var set_: Dictionary = _tiles[0]
	for row in 16:
		for col in 16:
			if FileAccess.file_exists(tile_path(0, col, row)):
				set_[row * 16 + col] = true


## Pour chaque tuile, remonte ses ancêtres : `max_level_under` sans parcours d'arbre.
func _index_ancestors() -> void:
	for level in range(1, max_level + 1):
		var cols := tiles_per_side(level)
		for key: int in _tiles[level]:
			var col := key % cols
			var row := key / cols
			for up in range(level - 1, -1, -1):
				var shift := level - up
				var akey := key_of(up, col >> shift, row >> shift)
				if int(_max_under.get(akey, -1)) >= level:
					break
				_max_under[akey] = level


## Vrai si la tuile existe dans le manifeste (E1+) ou dans `data/map/height/` (E0).
func has_tile(level: int, col: int, row: int) -> bool:
	if level < 0 or level > MAX_LEVEL or level >= _tiles.size():
		return false
	var cols := tiles_per_side(level)
	if col < 0 or row < 0 or col >= cols or row >= cols:
		return false
	if _broken.has(key_of(level, col, row)):
		return false
	return _tiles[level].has(row * cols + col)


## PB3g : indices `row × tiles_per_side + col` des tuiles de l'étage (pour `ReliefLod`).
func tile_indices(level: int) -> PackedInt32Array:
	var out := PackedInt32Array()
	if level >= 0 and level < _tiles.size():
		for index: int in _tiles[level]:
			out.append(index)
	return out


## PB3g : clés des tuiles marquées illisibles.
func broken_keys() -> PackedInt64Array:
	var out := PackedInt64Array()
	for key: int in _broken:
		out.append(key)
	return out


## Marque une tuile illisible (fichier absent ou corrompu) : `has_tile` devient faux.
func mark_broken(level: int, col: int, row: int) -> void:
	_broken[key_of(level, col, row)] = true


## Étage le plus fin présent dans le sous-arbre de la tuile (elle-même comprise), -1 si aucun.
func max_level_under(level: int, col: int, row: int) -> int:
	var best := level if has_tile(level, col, row) else -1
	return maxi(best, int(_max_under.get(key_of(level, col, row), -1)))


func tile_path(level: int, col: int, row: int) -> String:
	if level == 0:
		return e0_dir.path_join(e0_pattern.replace("{col}", str(col)).replace("{row}", str(row)))
	return tiles_dir.path_join(pattern.replace("{level}", str(level)).replace("{col}", str(col)).replace("{row}", str(row)))


## Étage le plus fin disponible au point carte (x, y) en unités monde (0 = E0, -1 hors tuiles).
func finest_level_at(x: float, y: float) -> int:
	for level in range(max_level, -1, -1):
		var t := tile_at(level, x, y)
		if has_tile(level, t.x, t.y):
			return level
	return -1


## Étage le plus fin présent ≤ `level` couvrant la tuile (level, col, row) (-1 si aucun).
func finest_ancestor(level: int, col: int, row: int) -> int:
	for up in range(mini(level, max_level), -1, -1):
		var shift := level - up
		if has_tile(up, col >> shift, row >> shift):
			return up
	return -1
