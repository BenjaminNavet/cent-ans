extends SceneTree

## Test headless du chantier FK (carte vivante) : squelette désactivé, rempli par FK3-FK5.
## Usage : godot --headless --path game --script res://tests/fk_folk_test.gd


func _init() -> void:
	print("fk_folk_test: SKIPPED (FK0 skeleton)")
	quit(0)
