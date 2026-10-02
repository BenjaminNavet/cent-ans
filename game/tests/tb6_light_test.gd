extends SceneTree

## Lot TB6 : lumière et atmosphère de la carte de campagne. Vérifie les paramètres appliqués par
## saison et par météo, et que la brume du matin épargne la province sélectionnée.
## Usage : godot --headless --path game --script res://tests/tb6_light_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	print("TB6 light test: skeleton")
	quit(0)


func _check(ok: bool, label: String) -> void:
	if not ok:
		_failures += 1
		push_error("TB6 FAIL: %s" % label)
	else:
		print("TB6 ok: %s" % label)
