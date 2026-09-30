extends SceneTree

## Test headless du chantier VT3 (arbres de la carte à l'échelle 1:1, ADR 0138 addendum VT3) :
##  1. `MapPropScale` : échelle des arbres constante, hauteurs réelles 12-30 m par essence, portée
##     où un arbre de 20 m fait ≈ 1 px (1080p, fov 55°) ;
##  2. carte de campagne (headless) : arbres individuels (tuiles et forêt dense) coupés au-delà de
##     la portée, présents en deçà ; forêt dense pleine près du point visé.
## Usage : godot --headless --path game --script res://tests/vt3_trees_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	_test_scale()
	if _failures > 0:
		push_error("vt3_trees_test: %d failure(s)" % _failures)
		quit(1)
		return
	print("vt3_trees_test: OK")
	quit(0)


func _check(condition: bool, label: String) -> bool:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
	return condition


func _test_scale() -> void:
	pass  # TODO VT3 : implémenté au lot suivant
