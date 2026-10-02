extends SceneTree

## Lot TB4 (campagne façon Thrones of Britannia) : conséquences visibles de la guerre et des
## fléaux. Squelette : les contrôles arrivent avec chaque point du lot (brûlis par-dessus la carte
## de couleur, peste, champ de bataille marqué quelques tours, engins de siège dans le camp).
## Usage : godot --headless --path game --script res://tests/tb4_scars_test.gd


func _init() -> void:
	var ok := true
	if ok:
		print("TB4 scars test OK")
	quit(0 if ok else 1)
