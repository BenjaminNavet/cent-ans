extends SceneTree

## Lot TB1 (campagne façon Thrones of Britannia) : saisons visibles à tous les zooms.
## Squelette : les contrôles arrivent avec chaque point du lot (delta saisonnier par-dessus la carte
## de couleur, mer selon la saison, étalonnage par saison lu dans `data/ui/`, neige sur les toits
## des villes 1:1).
## Usage : godot --headless --path game --script res://tests/tb1_seasons_test.gd

const TERRAIN_SHADER := preload("res://shaders/terrain.gdshader")


func _init() -> void:
	var ok := true
	ok = _check_ground_delta() and ok
	ok = _check_sea() and ok
	ok = _check_grade() and ok
	ok = _check_roofs() and ok
	if ok:
		print("TB1 seasons test OK")
	quit(0 if ok else 1)


## Point 1 : delta saisonnier appliqué par-dessus la carte de couleur.
func _check_ground_delta() -> bool:
	return true


## Point 2 : mer selon la saison.
func _check_sea() -> bool:
	return true


## Point 3 : étalonnage par saison (`data/ui/`).
func _check_grade() -> bool:
	return true


## Point 4 : neige sur les toits des villes 1:1.
func _check_roofs() -> bool:
	return true
