extends SceneTree

## Lot TB5 (campagne façon Thrones of Britannia) : mer et côtes.
## Squelette : les contrôles arrivent avec chaque point du lot (type de côte par région, mers par
## bassin, ressac au trait de côte).
## Usage : godot --headless --path game --script res://tests/tb5_coast_test.gd


func _init() -> void:
	var ok := true
	if ok:
		print("TB5 coast test OK")
	quit(0 if ok else 1)
