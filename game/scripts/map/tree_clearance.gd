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
## Majorant du rayon d'un houppier généralisé (unités monde) pour la table des cellules humides.
const MAX_CROWN_RADIUS := 1.2

var map_data: MapData
## Rayon nominal d'un houppier (unités monde) : élargissement des contours de lac.
var crown_radius: float = 0.4
## Facteur monde / modèle des arbres (`MapPropScale.generalised_scale()`) et part du rayon de
## houppier dégagée (`generalised_crown_clearance`).
var tree_scale: float = 0.5
var crown_clearance: float = 1.0
## Élargissement des houppiers appliqué aux instances gardées (`generalised_crown_widen`).
var crown_widen: float = 1.0
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
				if area > best:
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
	## 1 par cellule de `ROAD_CELL` px où un fleuve ou une côte est assez proche pour toucher un
	## houppier ; les autres cellules ne lisent pas les rasters.
	var _wet := PackedByteArray()

	## Retire des tampons MultiMesh (`VegetationTileJob._pack`, 16 flottants par instance) les
	## arbres dont le houppier déborde ; `counts` est mis à jour. L'ordre (graines décroissantes)
	## est gardé.
	func apply(buffers: Array[PackedFloat32Array], counts: PackedInt32Array) -> void:
		var t0 := Time.get_ticks_usec()
		var cell := TreeClearance.ROAD_CELL
		_cells_x = ceili(rect.size.x / cell)
		var cells_y := ceili(rect.size.y / cell)
		_index_roads(cells_y)
		_index_water(cells_y)
		var data := owner.map_data
		var widen := owner.crown_widen
		var scale := owner.tree_scale * 0.5 * owner.crown_clearance
		var road_sq := owner.road_clearance * owner.road_clearance
		var stride := VegetationTileJob.FLOATS_PER_INSTANCE
		var has_lakes := not lakes.is_empty()
		var has_roads := not _road_cells.is_empty()
		var origin := rect.position
		var last_cell := _cells_x * cells_y - 1
		for slot in buffers.size():
			var buffer := buffers[slot]
			var count := buffer.size() / stride
			if count == 0:
				continue
			# Tampon de sortie construit par plages gardées, seulement si une instance est retirée.
			var out := PackedFloat32Array()
			var run_start := 0
			var dropped := 0
			for i in count:
				var k := i * stride
				var x := buffer[k + 3]
				var y := buffer[k + 11]
				var key := clampi(int((y - origin.y) / cell) * _cells_x + int((x - origin.x) / cell), 0, last_cell)
				var blocked := false
				if widen != 1.0:
					# Colonnes X et Z de la base : houppier élargi, hauteur inchangée.
					for f: int in [0, 2, 4, 6, 8, 10]:
						buffer[k + f] *= widen
				if _wet[key] != 0:
					# Rayon du houppier : demi-largeur de l'instance (colonne X de la base) à l'échelle.
					var radius := Vector3(buffer[k], buffer[k + 4], buffer[k + 8]).length() * scale
					blocked = data.river_sd_at(x, y) < radius or owner.coast_distance(x, y) < radius
				if not blocked and has_lakes:
					var p := Vector2(x, y)
					for lake: Array in lakes:
						if (lake[0] as Rect2).has_point(p) and Geometry2D.is_point_in_polygon(p, lake[1]):
							blocked = true
							break
				if not blocked and has_roads:
					var segments: Variant = _road_cells.get(key)
					if segments != null:
						var p := Vector2(x, y)
						var points: PackedVector2Array = segments
						for s in range(0, points.size(), 2):
							if Geometry2D.get_closest_point_to_segment(p, points[s], points[s + 1]).distance_squared_to(p) < road_sq:
								blocked = true
								break
				if blocked:
					if i > run_start:
						out.append_array(buffer.slice(run_start * stride, k))
					run_start = i + 1
					dropped += 1
			if dropped > 0:
				if run_start < count:
					out.append_array(buffer.slice(run_start * stride))
				removed += dropped
				buffers[slot] = out
				counts[slot] = count - dropped
			elif widen != 1.0:
				buffers[slot] = buffer
		filter_ms = (Time.get_ticks_usec() - t0) / 1000.0

	## Cellules proches d'un fleuve ou d'une côte (distance au centre < demi-diagonale + houppier).
	func _index_water(cells_y: int) -> void:
		var cell := TreeClearance.ROAD_CELL
		var reach := cell * 0.7072 + TreeClearance.MAX_CROWN_RADIUS
		var data := owner.map_data
		_wet.resize(_cells_x * cells_y)
		var k := 0
		for cy in cells_y:
			var y := rect.position.y + (cy + 0.5) * cell
			for cx in _cells_x:
				var x := rect.position.x + (cx + 0.5) * cell
				_wet[k] = 1 if data.river_sd_at(x, y) < reach or owner.coast_distance(x, y) < reach else 0
				k += 1

	## Table cellule → segments de route proches (extrémités par paires).
	func _index_roads(cells_y: int) -> void:
		if roads.is_empty():
			return
		var cell := TreeClearance.ROAD_CELL
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
