extends SceneTree

## Test headless du lot HC1 (ADR 0161, arbres généralisés). Squelette : contrôles à venir.
## Usage : godot --headless --path game --script res://tests/hc_forest_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	print("hc_forest_test: SKIPPED (skeleton)")
	quit(0)
