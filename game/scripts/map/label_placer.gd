class_name LabelPlacer
extends RefCounted

## Lot UX1 (audit A3, C8) : placement des plaques d'effectif d'armée hors des noms de ville et
## des autres plaques. Rendu seulement, logique pure sur des rectangles écran (testable sans
## scène).
##
## Chaque plaque a une position de base (au-dessus de l'étendard) et quelques positions de
## repli (plus haut, à droite, à gauche, encore plus haut, dessous) ; on garde la première libre.
## Les obstacles (noms de ville visibles) et les plaques déjà posées sont rangés dans une
## grille spatiale simple (cases de `CELL` px) : coût linéaire pour des dizaines d'armées.
##
## Stabilité (pas de clignotement) : une plaque déplacée garde sa position de repli tant
## qu'elle reste libre ; elle ne revient à sa base que si la base, élargie de `HYSTERESIS_PX`,
## est libre. L'appelant ne recalcule que quand la caméra bouge (ou à intervalle lent).

const CELL := 64.0
## Écart entre une plaque déplacée et la position qu'elle quitte.
const GAP := 3.0
## Décalage latéral en plus de la demi-largeur (l'étendard est au milieu de la base).
const SIDE_CLEARANCE := 14.0
## Marge exigée autour de la base avant d'y revenir.
const HYSTERESIS_PX := 6.0

## id → index du candidat retenu au dernier placement.
var _choice: Dictionary = {}


## Décalages candidats (par rapport à la base) pour une plaque de taille `size`, par ordre de
## préférence : base, au-dessus, à droite, à gauche, deux crans au-dessus, dessous.
static func candidate_offsets(size: Vector2) -> PackedVector2Array:
	var up := size.y + GAP
	var side := size.x + SIDE_CLEARANCE
	return PackedVector2Array([
		Vector2.ZERO,
		Vector2(0.0, -up),
		Vector2(side * 0.5 + SIDE_CLEARANCE * 0.5, up * 0.5),
		Vector2(-(side * 0.5 + SIDE_CLEARANCE * 0.5), up * 0.5),
		Vector2(0.0, -2.0 * up),
		Vector2(0.0, up),
	])


## Place les plaques. `items` : [{id: String, rect: Rect2 (position de base)}], par ordre de
## priorité (les premières gardent leur base). `obstacles` : rectangles écran à éviter.
## Renvoie id → décalage écran (Vector2) à ajouter à la position de base.
func place(items: Array, obstacles: Array) -> Dictionary:
	var grid := SpatialGrid.new()
	for rect in obstacles:
		grid.insert(rect)
	var offsets := {}
	var seen := {}
	for item in items:
		var id := str(item["id"])
		var base: Rect2 = item["rect"]
		var candidates := candidate_offsets(base.size)
		var previous: int = _choice.get(id, 0)
		var chosen := -1
		if previous != 0 and previous < candidates.size():
			# Hystérésis : retour à la base seulement avec de la marge, sinon on reste.
			if not grid.hits(base.grow(HYSTERESIS_PX)):
				chosen = 0
			elif not grid.hits(Rect2(base.position + candidates[previous], base.size)):
				chosen = previous
		if chosen < 0:
			chosen = _first_free(grid, base, candidates, previous)
		_choice[id] = chosen
		seen[id] = true
		offsets[id] = candidates[chosen]
		grid.insert(Rect2(base.position + candidates[chosen], base.size))
	for id in _choice.keys():
		if not seen.has(id):
			_choice.erase(id)
	return offsets


## Premier candidat libre ; à défaut, celui qui recouvre le moins (le précédent à égalité).
static func _first_free(grid: SpatialGrid, base: Rect2, candidates: PackedVector2Array, previous: int) -> int:
	var best := 0
	var best_overlap := INF
	for i in candidates.size():
		var rect := Rect2(base.position + candidates[i], base.size)
		var overlap := grid.overlap_area(rect)
		if overlap <= 0.0:
			return i
		if overlap < best_overlap - 0.5 or (absf(overlap - best_overlap) <= 0.5 and i == previous):
			best_overlap = overlap
			best = i
	return best


## Candidat retenu pour `id` au dernier placement (0 = base), -1 si inconnu (tests).
func choice_of(id: String) -> int:
	return int(_choice.get(id, -1))


func reset() -> void:
	_choice.clear()


## Grille spatiale de rectangles (cases carrées de `CELL` px).
class SpatialGrid:
	extends RefCounted

	var _rects: Array[Rect2] = []
	var _cells: Dictionary = {}  # Vector2i → PackedInt32Array d'indices

	func insert(rect: Rect2) -> void:
		var index := _rects.size()
		_rects.append(rect)
		for key in _keys(rect):
			var list: PackedInt32Array = _cells.get(key, PackedInt32Array())
			list.append(index)
			_cells[key] = list

	## Vrai si `rect` coupe un rectangle de la grille.
	func hits(rect: Rect2) -> bool:
		for index in _candidates(rect):
			if _rects[index].intersects(rect):
				return true
		return false

	## Aire totale recouverte par les rectangles de la grille.
	func overlap_area(rect: Rect2) -> float:
		var total := 0.0
		for index in _candidates(rect):
			var other := _rects[index]
			if other.intersects(rect):
				total += other.intersection(rect).get_area()
		return total

	func _candidates(rect: Rect2) -> Dictionary:
		var found := {}
		for key in _keys(rect):
			for index in _cells.get(key, PackedInt32Array()):
				found[index] = true
		return found

	static func _keys(rect: Rect2) -> Array[Vector2i]:
		var keys: Array[Vector2i] = []
		var x0 := floori(rect.position.x / LabelPlacer.CELL)
		var y0 := floori(rect.position.y / LabelPlacer.CELL)
		var x1 := floori(rect.end.x / LabelPlacer.CELL)
		var y1 := floori(rect.end.y / LabelPlacer.CELL)
		for y in range(y0, y1 + 1):
			for x in range(x0, x1 + 1):
				keys.append(Vector2i(x, y))
		return keys
