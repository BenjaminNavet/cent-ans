class_name TreeClearance
extends RefCounted

## Lot HC1 (ADR 0161) : dégagements des arbres généralisés. Un arbre grossi (houppier d'environ
## une demi-unité de rayon) ne doit pas déborder sur l'eau ni sur une route principale : après le
## semis (natif ou GDScript, qui n'écarte que le pied des arbres), les instances dont le houppier
## touche un fleuve, un lac (`data/map/lakes.json`), la mer ou une eau raster (`coast_dist.png`) ou dont le pied est
## sur une route principale sont retirées des tampons.
##
## - `setup` (fil principal, une fois) : contours des lacs élargis du rayon nominal, boîtes des
##   routes principales.
## - `for_tile` (fil principal, par tuile) : extrait ce qui touche la tuile.
## - `TileFilter.apply` (fil de travail) : filtre les tampons d'une tuile. Lecture seule des images
##   partagées de `MapData`, comme le semis GDScript.
## Purement visuel : aucune règle de jeu.

const LAKES_FILE := "lakes.json"
## Côté (px carte) des cellules de la table des segments de route d'une tuile.
const ROAD_CELL := 4.0
## Niveaux 8 bits par px carte de `coast_dist.png` (`coast_dist_scale` de `terrain.gdshader`).
const COAST_DIST_SCALE := 2.0

var map_data: MapData
## Rayon nominal d'un houppier (unités monde) : élargissement des contours de lac.
var crown_radius: float = 0.4
## Facteur monde / modèle des arbres (`MapPropScale.generalised_scale()`) et part du rayon de
## houppier dégagée (`generalised_crown_clearance`).
var tree_scale: float = 0.5
var crown_clearance: float = 1.0
## Demi-largeur dégagée des routes principales (0 : routes ignorées).
var road_clearance: float = 0.0
var stats: Dictionary = {"lakes": 0, "roads": 0, "removed": 0, "filter_ms_max": 0.0}

var _lakes: Array = []  # [Rect2, PackedVector2Array] : contour élargi et sa boîte
var _roads: Array = []  # [Rect2, PackedVector2Array] : route principale et sa boîte élargie
var _coast_scale := Vector2.ONE


## `lake_polygons` : contours (px carte) ; vide → lus dans `lakes.json`. `road_lines` : polylignes
## des routes principales.
func setup(data: MapData, lake_polygons: Array, road_lines: Array) -> void:
	map_data = data
	_lakes.clear()
	_roads.clear()
	var polygons := lake_polygons if not lake_polygons.is_empty() else load_lakes(data.map_dir.path_join(LAKES_FILE))
	for polygon: PackedVector2Array in polygons:
		var grown := polygon
		if crown_radius > 0.0:
			# Plus grand morceau si l'élargissement en rend plusieurs (comme `LakesRenderer`).
			var best := 0.0
			for piece: PackedVector2Array in Geometry2D.offset_polygon(polygon, crown_radius, Geometry2D.JOIN_MITER):
				var area := _bounds(piece).get_area()
				if area > best and not Geometry2D.is_polygon_clockwise(piece):
					best = area
					grown = piece
		_lakes.append([_bounds(grown), grown])
	if road_clearance > 0.0:
		for line: PackedVector2Array in road_lines:
			if line.size() >= 2:
				_roads.append([_bounds(line).grow(road_clearance), line])
	if data.coast_dist_image != null and not data.coast_dist_image.is_empty():
		_coast_scale = Vector2(data.coast_dist_image.get_size()) / Vector2(data.size)
	stats["lakes"] = _lakes.size()
	stats["roads"] = _roads.size()


## Contours des lacs de `path` (px carte), [] si le fichier manque.
static func load_lakes(path: String) -> Array:
	var result: Array = []
	if not FileAccess.file_exists(path):
		return result
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	if not parsed is Dictionary:
		return result
	for entry: Dictionary in (parsed as Dictionary).get("lakes", []):
		var coords: Array = entry.get("polygon_px", [])
		if coords.size() < 3:
			continue
		var polygon := PackedVector2Array()
		polygon.resize(coords.size())
		for i in coords.size():
			polygon[i] = Vector2(float(coords[i][0]), float(coords[i][1]))
		result.append(polygon)
	return result


static func _bounds(points: PackedVector2Array) -> Rect2:
	var rect := Rect2(points[0], Vector2.ZERO)
	for p in points:
		rect = rect.expand(p)
	return rect


## Vrai si (x, y) est dans un lac élargi (tests).
func in_lake(x: float, y: float) -> bool:
	var p := Vector2(x, y)
	for lake: Array in _lakes:
		if (lake[0] as Rect2).has_point(p) and Geometry2D.is_point_in_polygon(p, lake[1]):
			return true
	return false


## Filtre de la tuile `rect` (px carte) : lacs et routes qui la touchent.
func for_tile(rect: Rect2) -> TileFilter:
	var filter := TileFilter.new()
	filter.owner = self
	filter.rect = rect
	for lake: Array in _lakes:
		if rect.intersects(lake[0]):
			filter.lakes.append(lake)
	for road: Array in _roads:
		if rect.intersects(road[0]):
			filter.roads.append(road[1])
	return filter


## Distance signée à la côte (px carte, < 0 en mer), grande sans `coast_dist.png`.
func coast_distance(x: float, y: float) -> float:
	var image := map_data.coast_dist_image
	if image == null or image.is_empty():
		return 1e6
	var px := clampi(int(x * _coast_scale.x), 0, image.get_width() - 1)
	var py := clampi(int(y * _coast_scale.y), 0, image.get_height() - 1)
	return (image.get_pixel(px, py).r * 255.0 - 128.0) / COAST_DIST_SCALE


## Filtre d'une tuile, appliqué hors du fil principal.
class TileFilter:
	extends RefCounted

	var owner: TreeClearance
	var rect: Rect2
	var lakes: Array = []
	var roads: Array = []
	var removed: int = 0
	var filter_ms: float = 0.0
	var _road_cells: Dictionary = {}
	var _cells_x: int = 0

	## Retire des tampons MultiMesh (`VegetationTileJob._pack`, 16 flottants par instance) les
	## arbres dont le houppier déborde ; `counts` est mis à jour. L'ordre (graines décroissantes)
	## est gardé.
	func apply(buffers: Array[PackedFloat32Array], counts: PackedInt32Array) -> void:
		var t0 := Time.get_ticks_usec()
		_index_roads()
		var data := owner.map_data
		var scale := owner.tree_scale * 0.5 * owner.crown_clearance
		var road_sq := owner.road_clearance * owner.road_clearance
		var stride := VegetationTileJob.FLOATS_PER_INSTANCE
		for slot in buffers.size():
			var buffer := buffers[slot]
			var count := buffer.size() / stride
			if count == 0:
				continue
			var kept := 0
			for i in count:
				var k := i * stride
				var x := buffer[k + 3]
				var y := buffer[k + 11]
				# Rayon du houppier : demi-largeur de l'instance (colonne X de la base) à l'échelle.
				var radius := Vector3(buffer[k], buffer[k + 4], buffer[k + 8]).length() * scale
				if _blocked(data, x, y, radius, road_sq):
					continue
				if kept != i:
					var to := kept * stride
					for f in stride:
						buffer[to + f] = buffer[k + f]
				kept += 1
			if kept != count:
				removed += count - kept
				buffer.resize(kept * stride)
				buffers[slot] = buffer
				counts[slot] = kept
		filter_ms = (Time.get_ticks_usec() - t0) / 1000.0

	func _blocked(data: MapData, x: float, y: float, radius: float, road_sq: float) -> bool:
		if data.river_sd_at(x, y) < radius:
			return true
		if owner.coast_distance(x, y) < radius:
			return true
		if not lakes.is_empty():
			var p := Vector2(x, y)
			for lake: Array in lakes:
				if (lake[0] as Rect2).has_point(p) and Geometry2D.is_point_in_polygon(p, lake[1]):
					return true
		if _cells_x > 0:
			var key := int((y - rect.position.y) / TreeClearance.ROAD_CELL) * _cells_x + int((x - rect.position.x) / TreeClearance.ROAD_CELL)
			var segments: Variant = _road_cells.get(key)
			if segments != null:
				var p := Vector2(x, y)
				var points: PackedVector2Array = segments
				for s in range(0, points.size(), 2):
					if Geometry2D.get_closest_point_to_segment(p, points[s], points[s + 1]).distance_squared_to(p) < road_sq:
						return true
		return false

	## Table cellule → segments de route proches (extrémités par paires).
	func _index_roads() -> void:
		if roads.is_empty():
			return
		var cell := TreeClearance.ROAD_CELL
		_cells_x = ceili(rect.size.x / cell)
		var cells_y := ceili(rect.size.y / cell)
		var reach := owner.road_clearance
		for line: PackedVector2Array in roads:
			for s in line.size() - 1:
				var a := line[s] - rect.position
				var b := line[s + 1] - rect.position
				var x0 := maxi(int((minf(a.x, b.x) - reach) / cell), 0)
				var x1 := mini(int((maxf(a.x, b.x) + reach) / cell), _cells_x - 1)
				var y0 := maxi(int((minf(a.y, b.y) - reach) / cell), 0)
				var y1 := mini(int((maxf(a.y, b.y) + reach) / cell), cells_y - 1)
				for cy in range(y0, y1 + 1):
					for cx in range(x0, x1 + 1):
						var key := cy * _cells_x + cx
						var points: PackedVector2Array = _road_cells.get(key, PackedVector2Array())
						points.append(line[s])
						points.append(line[s + 1])
						_road_cells[key] = points
