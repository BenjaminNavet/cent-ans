class_name SettlementCells
extends RefCounted

## FL3 : grille spatiale des colonies (ancres des noms), pour ne parcourir que celles qui peuvent
## être à l'écran (dé-encombrement) ou sous le curseur (survol). Avant : 2 134 colonies projetées
## à chaque passe, la plupart hors champ (survol jusqu'à 17 ms par image en panoramique).
## Chaque case non vide est une sphère englobante testée contre le tronc de vue, élargi d'une marge
## en pixels convertie en unités monde à la distance de la case.

## Côté d'une case (unités carte) : ≈ 200 cases non vides sur la carte.
const CELL_SIZE := 256.0

var _cells: Array[PackedInt32Array] = []
var _centers: PackedVector3Array = PackedVector3Array()
var _radii: PackedFloat32Array = PackedFloat32Array()
var _keys: Dictionary = {}  # Vector2i case -> indice dans `_cells`
var _count := 0
var _heights_dirty := false


## Range les points (`points[i]` : ancre monde de la colonie i) par case.
func build(points: PackedVector3Array) -> void:
	_cells.clear()
	_keys.clear()
	_count = points.size()
	for i in points.size():
		var key := Vector2i(floori(points[i].x / CELL_SIZE), floori(points[i].z / CELL_SIZE))
		var slot: int = _keys.get(key, -1)
		if slot < 0:
			slot = _cells.size()
			_keys[key] = slot
			_cells.append(PackedInt32Array())
		_cells[slot].append(i)
	_centers.resize(_cells.size())
	_radii.resize(_cells.size())
	refresh_heights(points)


## Hauteurs changées (échelle verticale, recalage des noms) : sphères recalculées.
func mark_heights_dirty() -> void:
	_heights_dirty = true


func refresh_heights(points: PackedVector3Array) -> void:
	_heights_dirty = false
	for slot in _cells.size():
		var low := INF
		var high := -INF
		for i in _cells[slot]:
			low = minf(low, points[i].y)
			high = maxf(high, points[i].y)
		# Centre de la case (x, z) : depuis le premier point (tous dans la même case).
		var first := points[_cells[slot][0]]
		var cx := (floorf(first.x / CELL_SIZE) + 0.5) * CELL_SIZE
		var cz := (floorf(first.z / CELL_SIZE) + 0.5) * CELL_SIZE
		_centers[slot] = Vector3(cx, (low + high) * 0.5, cz)
		_radii[slot] = Vector3(CELL_SIZE * 0.5, (high - low) * 0.5, CELL_SIZE * 0.5).length()


## Met `out[i]` à 1 pour les colonies des cases dans le tronc de vue de `camera` élargi de
## `margin_px` pixels (à la distance de chaque case), 0 sinon. Rend le nombre de colonies marquées.
func mark_in_view(camera: Camera3D, viewport_height: float, margin_px: float, points: PackedVector3Array, out: PackedByteArray) -> int:
	if _heights_dirty:
		refresh_heights(points)
	if out.size() != _count:
		out.resize(_count)
	out.fill(0)
	var planes := camera.get_frustum()
	var eye := camera.global_position
	var per_px := _world_per_px(camera, viewport_height)
	var marked := 0
	for slot in _cells.size():
		var center := _centers[slot]
		var reach := _radii[slot] + margin_px * per_px * maxf(eye.distance_to(center) - _radii[slot], 0.0)
		var inside := true
		for plane: Plane in planes:
			if plane.distance_to(center) > reach:
				inside = false
				break
		if inside:
			for i in _cells[slot]:
				out[i] = 1
			marked += _cells[slot].size()
	return marked


## Met `out[i]` à 1 pour les colonies des cases à moins de `margin_px` pixels (à leur distance)
## du rayon de vue passant par `screen_position`. Rend le nombre de colonies marquées.
func mark_near_ray(camera: Camera3D, viewport_height: float, screen_position: Vector2, margin_px: float, points: PackedVector3Array, out: PackedByteArray) -> int:
	if _heights_dirty:
		refresh_heights(points)
	if out.size() != _count:
		out.resize(_count)
	out.fill(0)
	var origin := camera.project_ray_origin(screen_position)
	var direction := camera.project_ray_normal(screen_position)
	var per_px := _world_per_px(camera, viewport_height)
	var marked := 0
	for slot in _cells.size():
		var to_center := _centers[slot] - origin
		var along := to_center.dot(direction)
		if along < -_radii[slot]:
			continue
		var gap := (to_center - direction * along).length()
		if gap > _radii[slot] + margin_px * per_px * maxf(along, 0.0):
			continue
		for i in _cells[slot]:
			out[i] = 1
		marked += _cells[slot].size()
	return marked


## Unités monde par pixel à une unité de distance (caméra perspective, hauteur de vue).
static func _world_per_px(camera: Camera3D, viewport_height: float) -> float:
	return 2.0 * tan(deg_to_rad(camera.fov) * 0.5) / maxf(viewport_height, 1.0)
