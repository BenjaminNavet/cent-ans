extends TestCase

## LR-08 : colonie en ruines. (1) `RuinMarkers` pose un signe par ruine du pont ; (2) le grisé du recrutement
## vient du cœur (test Rust `capture_tests`) et de `PanelWidgets.fill_recruitable`, déjà couvert.
## Usage : godot --headless --path game --script res://tests/lr08_ruins_test.gd

class FakeSim:
	extends RefCounted

	func get_ruined_places() -> Array:
		return [{"id": "set_royaumont", "name": "Royaumont", "turns_left": 3}]


func _init() -> void:
	await process_frame
	var markers := RuinMarkers.new()
	root.add_child(markers)
	markers.refresh(FakeSim.new(), func(_id: String) -> Vector3: return Vector3(10, 0, 10))
	check(markers.marker_count() == 1, "one ruin sign expected, got %d" % markers.marker_count())
	finish()
