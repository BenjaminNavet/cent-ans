extends SceneTree

## LR-08 : colonie en ruines. (1) `RuinMarkers` pose un signe par ruine du pont ; (2) la ligne de
## recrutement d'une colonie en ruines est grisée avec la raison du cœur (`PanelWidgets`).
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
	var list := VBoxContainer.new()
	root.add_child(list)
	var rows := [{"unit_type": "unit_men_at_arms", "name": "Hommes d'armes", "cost": 100, "upkeep": 5,
		"available": false, "reason": "la colonie est en ruines"}]
	PanelWidgets.fill_recruitable(list, rows, func(_u: String) -> void: pass)
	var button: Button = null
	var reason_shown := false
	for node in list.find_children("*", "", true, false):
		if node is Button and button == null:
			button = node
		if node is Label and str(node.text).contains("ruines"):
			reason_shown = true
	_check(button != null and button.disabled, "recruit button must be disabled")
	_check(reason_shown, "ruin reason must be shown under the row")
	print("lr08_ruins_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)
