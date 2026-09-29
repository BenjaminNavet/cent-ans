class_name FineGeoStore
extends RefCounted

## Données fines du lot ZG5a lues pour le rendu (lot ZG5b, ADR 0036) :
## - index des tuiles CAFV des fleuves (`data/map/rivers_fine.json`) et des routes drapées
##   (`fine_anchors.json` → `roads`), tuiles binaires hors git sous `data/map/pyramid/` ;
## - ancrages affinés des colonies, hameaux et franchissements (`fine_anchors.json`).
## Chargement des tuiles à la demande dans `WorkerThreadPool` (`request` / `poll`), ou tout de
## suite (`load_sync`, lit creusé d'une page), cache LRU par couche. Aucune règle de jeu.

const RIVERS_FILE := "rivers_fine.json"
const ANCHORS_FILE := "fine_anchors.json"
const PYRAMID_FILE := "relief_pyramid.json"
const TILE_UNITS := 64.0

var max_cached_tiles: int = 64
var max_jobs: int = 3

var map_dir: String = ""
## Couche → ensemble des clés de tuiles présentes (clé = row << 12 | col).
var _index: Dictionary = {1: {}, 2: {}}
var _dirs: Dictionary = {1: "", 2: ""}
var _patterns: Dictionary = {1: "E2/{col}_{row}.bin", 2: "E2/{col}_{row}.bin"}
## Couche → {clé: CafvTile}, et dernière utilisation (image) de chaque tuile.
var _tiles: Dictionary = {1: {}, 2: {}}
var _used: Dictionary = {1: {}, 2: {}}
## (couche << 24 | clé) → {task, job}
var _jobs: Dictionary = {}
var _frame: int = 0
## Protège `_tiles`, `_used` et `_broken` : les tâches de lit creusé (`FineBedCarver`) lisent et
## remplissent le cache depuis des fils de travail (`fetch_threadsafe`).
var _mutex := Mutex.new()
var _broken: Dictionary = {1: {}, 2: {}}

## Décalage d'origine du cache en tuiles E2 (`relief_pyramid.json` `root_origin_tiles` × 4,
## ADR 0115) : tuiles et points CAFV sont écrits dans le cadre de la pyramide (carte 4096² d'avant
## OM) ; l'index, les clés et les points chargés sont en coordonnées monde, seul le chemin de
## fichier revient au cadre du cache. Les ancrages (`fine_anchors.json`) sont déjà en unités monde.
var origin_tiles: Vector2i = Vector2i.ZERO

## Ancrages : id de colonie → {px: Vector2, z}; hameaux dans l'ordre de `hamlets.json`
## (Vector4 x, y, z, déplacement) ; franchissements dans l'ordre de `crossings_px.json`.
var settlements: Dictionary = {}
var hamlets: PackedVector4Array = PackedVector4Array()
var crossings: Array[Dictionary] = []
var road_widths: Dictionary = {"main": 6.0, "secondary": 4.0, "computed": 3.0}
var stats: Dictionary = {}


static func key_of(col: int, row: int) -> int:
	return (row << 12) | col


static func tile_rect(col: int, row: int) -> Rect2:
	return Rect2(col * TILE_UNITS, row * TILE_UNITS, TILE_UNITS, TILE_UNITS)


## Charge les index et les ancrages ; vrai si au moins une couche a des tuiles sur le disque.
## `tiles_root` : dossier qui remplace `data/map` pour les tuiles (tests).
func load_from(dir: String, tiles_root: String = "") -> bool:
	map_dir = dir
	var root := tiles_root if tiles_root != "" else dir
	origin_tiles = Vector2i.ZERO
	var pyramid: Variant = _read_json(dir.path_join(PYRAMID_FILE))
	if pyramid is Dictionary:
		var origin: Variant = (pyramid as Dictionary).get("root_origin_tiles", [0, 0])
		if origin is Array and (origin as Array).size() == 2:
			var per_root := int(ReliefPyramid.ROOT_TILE_UNITS / TILE_UNITS)
			origin_tiles = Vector2i(int(origin[0]) * per_root, int(origin[1]) * per_root)
	var rivers: Variant = _read_json(dir.path_join(RIVERS_FILE))
	if rivers is Dictionary:
		_load_index(1, rivers, root)
	var anchors: Variant = _read_json(dir.path_join(ANCHORS_FILE))
	if anchors is Dictionary:
		_load_anchors(anchors)
		var roads: Variant = (anchors as Dictionary).get("roads")
		if roads is Dictionary:
			_load_index(2, roads, root)
			var widths: Variant = (roads as Dictionary).get("width_m")
			if widths is Dictionary:
				road_widths = widths
	stats = {
		"river_tiles": (_index[1] as Dictionary).size(),
		"road_tiles": (_index[2] as Dictionary).size(),
		"settlements": settlements.size(),
		"hamlets": hamlets.size(),
		"crossings": crossings.size(),
	}
	return available(1) or available(2)


## Vrai si la couche a un index et que son dossier de tuiles existe (cache hors git).
func available(layer: int) -> bool:
	return not (_index[layer] as Dictionary).is_empty() and DirAccess.dir_exists_absolute(str(_dirs[layer]))


## Index immuable après `load_from` : lisible depuis un fil.
func has_tile(layer: int, col: int, row: int) -> bool:
	return (_index[layer] as Dictionary).has(key_of(col, row))


## Fichier de la tuile monde (col, row) : coordonnées du cache (décalage `origin_tiles` retiré).
func tile_path(layer: int, col: int, row: int) -> String:
	var c := col - origin_tiles.x
	var r := row - origin_tiles.y
	return str(_dirs[layer]).path_join(str(_patterns[layer]).replace("{col}", str(c)).replace("{row}", str(r)))


## Tuile chargée (null sinon) ; marque son utilisation (LRU).
func get_tile(layer: int, col: int, row: int) -> CafvTile:
	var key := key_of(col, row)
	_mutex.lock()
	var tile: CafvTile = (_tiles[layer] as Dictionary).get(key)
	if tile != null:
		(_used[layer] as Dictionary)[key] = _frame
	_mutex.unlock()
	return tile


func is_loaded(layer: int, col: int, row: int) -> bool:
	_mutex.lock()
	var loaded := (_tiles[layer] as Dictionary).has(key_of(col, row)) or (_broken[layer] as Dictionary).has(key_of(col, row))
	_mutex.unlock()
	return loaded


## Tuile depuis n'importe quel fil : cache, sinon lecture du fichier (hors verrou) puis mise en
## cache. Null si absente ou illisible.
func fetch_threadsafe(layer: int, col: int, row: int) -> CafvTile:
	if not has_tile(layer, col, row):
		return null
	var key := key_of(col, row)
	_mutex.lock()
	var tile: CafvTile = (_tiles[layer] as Dictionary).get(key)
	var broken := (_broken[layer] as Dictionary).has(key)
	_mutex.unlock()
	if tile != null or broken:
		return tile
	tile = CafvTile.load_file(tile_path(layer, col, row), origin_tiles)
	_store(layer, key, tile)
	return tile


## Charge une tuile tout de suite sur le fil appelant (si elle n'est pas déjà là).
func load_sync(layer: int, col: int, row: int) -> CafvTile:
	if not has_tile(layer, col, row):
		return null
	var tile := get_tile(layer, col, row)
	if tile != null:
		return tile
	var jkey := (layer << 24) | key_of(col, row)
	if _jobs.has(jkey):
		var entry: Dictionary = _jobs[jkey]
		WorkerThreadPool.wait_for_task_completion(entry["task"])
		_jobs.erase(jkey)
		_store(layer, (entry["job"] as TileJob).key, (entry["job"] as TileJob).tile)
		return get_tile(layer, col, row)
	tile = CafvTile.load_file(tile_path(layer, col, row), origin_tiles)
	_store(layer, key_of(col, row), tile)
	return tile


## Demande une tuile (chargement dans un fil) ; sans effet si elle est chargée ou demandée.
func request(layer: int, col: int, row: int) -> void:
	if not has_tile(layer, col, row) or is_loaded(layer, col, row):
		return
	var jkey := (layer << 24) | key_of(col, row)
	if _jobs.has(jkey) or _jobs.size() >= max_jobs:
		return
	var job := TileJob.new()
	job.path = tile_path(layer, col, row)
	job.offset = origin_tiles
	job.key = key_of(col, row)
	_jobs[jkey] = {"task": WorkerThreadPool.add_task(job.run, false, "fine geo tile"), "job": job, "layer": layer}


func pending() -> int:
	return _jobs.size()


## Récupère les chargements terminés (tous avec `block`) ; rend le nombre de tuiles arrivées.
func poll(block: bool = false) -> int:
	_frame += 1
	var arrived := 0
	for jkey: int in _jobs.keys():
		var entry: Dictionary = _jobs[jkey]
		if not block and not WorkerThreadPool.is_task_completed(entry["task"]):
			continue
		WorkerThreadPool.wait_for_task_completion(entry["task"])
		_jobs.erase(jkey)
		var job: TileJob = entry["job"]
		_store(int(entry["layer"]), job.key, job.tile)
		arrived += 1
	return arrived


func _store(layer: int, key: int, tile: CafvTile) -> void:
	_mutex.lock()
	if tile == null:
		# Illisible : plus de nouvelle demande.
		(_broken[layer] as Dictionary)[key] = true
	else:
		(_tiles[layer] as Dictionary)[key] = tile
		(_used[layer] as Dictionary)[key] = _frame
		_evict(layer)
	_mutex.unlock()


func _evict(layer: int) -> void:
	var tiles: Dictionary = _tiles[layer]
	var used: Dictionary = _used[layer]
	while tiles.size() > max_cached_tiles:
		var oldest := -1
		var oldest_frame := _frame + 1
		for key: int in tiles:
			var f: int = used.get(key, 0)
			if f < oldest_frame:
				oldest_frame = f
				oldest = key
		if oldest < 0 or oldest_frame >= _frame:
			return  # tout sert à l'image courante : on déborde plutôt que de jeter
		tiles.erase(oldest)
		used.erase(oldest)


func wait_all() -> void:
	poll(true)


func _load_index(layer: int, manifest: Dictionary, root: String) -> void:
	_dirs[layer] = root.path_join(str(manifest.get("dir", "")))
	_patterns[layer] = str(manifest.get("pattern", _patterns[layer]))
	var index := {}
	for tile: Dictionary in manifest.get("tiles", []):
		index[key_of(int(tile.get("col", 0)) + origin_tiles.x, int(tile.get("row", 0)) + origin_tiles.y)] = true
	_index[layer] = index


func _load_anchors(anchors: Dictionary) -> void:
	settlements.clear()
	var raw_settlements: Variant = anchors.get("settlements")
	if raw_settlements is Dictionary:
		for id: String in raw_settlements:
			var entry: Dictionary = raw_settlements[id]
			var px: Array = entry.get("px", [0, 0])
			settlements[id] = {"px": Vector2(float(px[0]), float(px[1])), "z": float(entry.get("z", 0.0)), "moved_m": float(entry.get("moved_m", 0.0))}
	hamlets.clear()
	var raw_hamlets: Variant = anchors.get("hamlets")
	if raw_hamlets is Dictionary:
		for item: Array in (raw_hamlets as Dictionary).get("items", []):
			hamlets.append(Vector4(float(item[0]), float(item[1]), float(item[2]), float(item[3]) if item.size() > 3 else 0.0))
	crossings.clear()
	for raw: Dictionary in anchors.get("crossings", []):
		var px: Array = raw.get("px", [0, 0])
		var dir: Array = raw.get("dir", [1, 0])
		crossings.append({
			"id": str(raw.get("id", "")),
			"px": Vector2(float(px[0]), float(px[1])),
			"snapped": bool(raw.get("snapped", false)),
			"dir": Vector2(float(dir[0]), float(dir[1])).normalized(),
			"width_m": float(raw.get("width_m", 0.0)),
			"z_water": float(raw.get("z_water", raw.get("z_ground", 0.0))),
			"z_deck": float(raw.get("z_deck", raw.get("z_ground", 0.0))),
		})


static func _read_json(path: String) -> Variant:
	if not FileAccess.file_exists(path):
		return null
	return JSON.parse_string(FileAccess.get_file_as_string(path))


class TileJob:
	extends RefCounted

	var path: String = ""
	var key: int = 0
	var offset: Vector2i = Vector2i.ZERO
	var tile: CafvTile

	func run() -> void:
		tile = CafvTile.load_file(path, offset)
