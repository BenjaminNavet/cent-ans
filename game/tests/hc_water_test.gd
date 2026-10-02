extends SceneTree

## Test headless du lot HC2 (ADR 0161 §3, eaux lisibles) — squelette, désactivé.
## Usage : godot --headless --path game --script res://tests/hc_water_test.gd


func _init() -> void:
	await process_frame
	print("hc_water_test: SKIPPED (skeleton)")
	quit(0)
