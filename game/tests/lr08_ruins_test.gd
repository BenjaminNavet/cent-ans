extends SceneTree

## LR-08 : colonie en ruines. (1) `RuinMarkers` pose un signe par ruine du pont ; (2) le grisé du recrutement
## vient du cœur (test Rust `capture_tests`) et de `PanelWidgets.fill_recruitable`, déjà couvert.
## Usage : godot --headless --path game --script res://tests/lr08_ruins_test.gd

class FakeSim:
	extends RefCounted

	func get_ruined_places() -> Array:
		return [{"id": "set_royaumont", "name": "Royaumont", "turns_left": 3}]


var _failures := 0


func _check(condition: bool, message: String) -> void:
	if not condition:
		_failures += 1
		push_error("lr08_ruins_test: " + message)


func _init() -> void:
	await process_frame
	var markers := RuinMarkers.new()
	root.add_child(markers)
	markers.refresh(FakeSim.new(), func(_id: String) -> Vector3: return Vector3(10, 0, 10))
	_check(markers.marker_count() == 1, "one ruin sign expected, got %d" % markers.marker_count())
	print("lr08_ruins_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)
