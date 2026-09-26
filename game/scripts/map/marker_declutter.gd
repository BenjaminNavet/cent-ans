class_name MarkerDeclutter
extends RefCounted

## Lot DA7d (ADR 0066) : placement glouton par priorité de rectangles écran (marqueurs de lieux et
## leurs noms), à la manière de Total War : on pose dans l'ordre de priorité ; un rectangle qui
## recouvre un rectangle déjà posé cède la place (sauf s'il est épinglé). Grille spatiale à cases
## fixes : chaque essai ne teste que les rectangles des cases qu'il touche (coût ~linéaire).
## Pur rendu (aucune règle de jeu) : la logique dépend de la caméra et reste côté Godot.

## Taille des cases de la grille (px) ; à régler sur la taille des plus grands rectangles.
var cell_px: float = 64.0

var _rects: Array[Rect2] = []
var _owners: PackedInt32Array = PackedInt32Array()
## Case (clé entière) → indices des rectangles posés qui la touchent.
var _grid: Dictionary = {}


func reset(cell: float = 64.0) -> void:
	cell_px = maxf(cell, 4.0)
	_rects.clear()
	_owners.clear()
	_grid.clear()


func placed_count() -> int:
	return _rects.size()


func placed_rects() -> Array[Rect2]:
	return _rects


## Vrai si `rect` recouvre un rectangle posé (hors ceux du propriétaire `ignore_owner`).
func overlaps(rect: Rect2, ignore_owner: int = -1) -> bool:
	var seen := {}
	for key in _keys(rect):
		for index: int in _grid.get(key, PackedInt32Array()):
			if seen.has(index):
				continue
			seen[index] = true
			if _owners[index] != ignore_owner and _rects[index].intersects(rect):
				return true
	return false


## Pose `rect` s'il est libre (ou si `force`) ; renvoie vrai s'il est posé.
func try_place(rect: Rect2, owner: int = -1, force: bool = false) -> bool:
	if not force and overlaps(rect, owner):
		return false
	var index := _rects.size()
	_rects.append(rect)
	_owners.append(owner)
	for key in _keys(rect):
		if not _grid.has(key):
			_grid[key] = PackedInt32Array()
		(_grid[key] as PackedInt32Array).append(index)
	return true


func _keys(rect: Rect2) -> PackedInt32Array:
	var keys := PackedInt32Array()
	var x0 := floori(rect.position.x / cell_px)
	var y0 := floori(rect.position.y / cell_px)
	var x1 := floori(rect.end.x / cell_px)
	var y1 := floori(rect.end.y / cell_px)
	for y in range(y0, y1 + 1):
		for x in range(x0, x1 + 1):
			# Clé entière : écran borné, décalage pour les coordonnées négatives.
			keys.append((y + 4096) * 8192 + (x + 4096))
	return keys


## Nombre de paires de rectangles qui se recouvrent (aire non nulle), en ignorant les paires d'un
## même propriétaire (un marqueur et son propre nom). Quadratique : mesures et tests seulement.
static func count_overlaps(rects: Array, owners: PackedInt32Array = PackedInt32Array()) -> int:
	var count := 0
	for a in rects.size():
		for b in range(a + 1, rects.size()):
			if owners.size() == rects.size() and owners[a] >= 0 and owners[a] == owners[b]:
				continue
			if (rects[a] as Rect2).intersects(rects[b] as Rect2):
				count += 1
	return count
