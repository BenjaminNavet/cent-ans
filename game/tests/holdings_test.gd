extends SceneTree

## Test headless de la liste « Colonies » (touche B), lot HL2. Désactivé le temps du squelette.
## Usage : godot --headless --path game --script res://tests/holdings_test.gd


func _init() -> void:
	print("holdings_test: SKIPPED (squelette HL2, à activer avec le panneau)")
	quit(0)
