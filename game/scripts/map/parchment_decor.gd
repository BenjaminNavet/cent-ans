class_name ParchmentDecor
extends RefCounted

## Lot CM2 : emplacements des ornements de la mer des portulans (roses des vents, navires,
## monstres marins), choisis au démarrage à partir de la distance à la côte (aucune donnée
## codée en dur) : roses et monstres au large, navires près des côtes. Purement visuel.

## Pas de la grille d'échantillonnage (px carte).
const GRID := 24
## Distance à la côte au-delà de laquelle la mer compte comme « large » (px carte).
const OPEN_SEA_PX := 45.0

var roses: Array[Vector3] = []  # x, y, rayon (px carte)
var ships: Array[Vector3] = []  # x, y, cap (radians, 0 = vers l'est)
var monsters: Array[Vector3] = []  # x, y, variante


## `coast_dist` : raster L8 signé (valeur × 255 − 128) / `scale` px, < 0 en mer.
static func build(map: MapData, coast_scale: float = 2.0) -> ParchmentDecor:
	var decor := ParchmentDecor.new()
	var img := map.coast_dist_image
	if img == null or img.is_empty():
		return decor
	var sx := float(img.get_width()) / float(map.size.x)
	var sy := float(img.get_height()) / float(map.size.y)
	var open_sea: Array = []  # [score, Vector2]
	var near_sea: Array = []
	var margin := GRID * 4
	for y in range(margin, map.size.y - margin, GRID):
		for x in range(margin, map.size.x - margin, GRID):
			var c := (img.get_pixel(int(x * sx), int(y * sy)).r * 255.0 - 128.0) / coast_scale
			var jitter := _hash(x, y)
			if c <= -OPEN_SEA_PX:
				open_sea.append([-c + jitter * 8.0, Vector2(x, y)])
			elif c < -10.0 and c > -30.0:
				near_sea.append([jitter, Vector2(x, y)])
	open_sea.sort_custom(func(a: Array, b: Array) -> bool: return a[0] > b[0])
	near_sea.sort_custom(func(a: Array, b: Array) -> bool: return a[0] > b[0])
	# Roses : les points les plus au large, espacés ; privilégie le cœur de la carte.
	var center := Vector2(map.size) * 0.5
	var rose_candidates := open_sea.duplicate()
	rose_candidates.sort_custom(func(a: Array, b: Array) -> bool:
		return a[0] - a[1].distance_to(center) * 0.02 > b[0] - b[1].distance_to(center) * 0.02)
	var taken: Array[Vector2] = []
	for entry in rose_candidates:
		var p: Vector2 = entry[1]
		if _far_from(p, taken, 850.0):
			taken.append(p)
			decor.roses.append(Vector3(p.x, p.y, 62.0))
			if decor.roses.size() >= 4:
				break
	# Monstres marins : au large, loin des roses.
	var monster_taken: Array[Vector2] = taken.duplicate()
	for entry in open_sea:
		var p: Vector2 = entry[1]
		if _far_from(p, monster_taken, 520.0):
			monster_taken.append(p)
			decor.monsters.append(Vector3(p.x, p.y, float(decor.monsters.size() % 2)))
			if decor.monsters.size() >= 4:
				break
	# Navires : près des côtes, espacés, cap tiré au sort.
	var ship_taken: Array[Vector2] = monster_taken.duplicate()
	for entry in near_sea:
		var p: Vector2 = entry[1]
		if _far_from(p, ship_taken, 330.0):
			ship_taken.append(p)
			decor.ships.append(Vector3(p.x, p.y, (1.0 if _hash(int(p.x), int(p.y) + 7) > 0.5 else -1.0)))
			if decor.ships.size() >= 9:
				break
	return decor


static func _far_from(p: Vector2, points: Array[Vector2], spacing: float) -> bool:
	for q in points:
		if p.distance_to(q) < spacing:
			return false
	return true


static func _hash(x: int, y: int) -> float:
	var h := (x * 73856093) ^ (y * 19349663)
	h = (h ^ (h >> 13)) * 1274126177
	return float(absi(h) % 10007) / 10007.0
