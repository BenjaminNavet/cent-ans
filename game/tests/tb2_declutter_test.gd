extends SceneTree

## Test headless du lot TB2 (désencombrement de la carte de campagne, plan
## `docs/design/2026-10-02-campagne-tob.md` § 3) : un seul signe par ville et par palier de zoom,
## étiquettes entières dans l'écran, noms de région en vue moyenne, frontières discrètes hors
## sélection, brouillard de guerre en voile de parchemin, nuages réservés à la météo réelle.
## Usage : godot --headless --path game --script res://tests/tb2_declutter_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	# Squelette : les vérifications arrivent point par point (voir docs/wip/tb2.md).
	if _failures == 0:
		print("tb2_declutter OK")
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("tb2_declutter: " + message)
	return condition
