extends TestCase

## Lot WH armyb : destination des recrues du panneau de colonie (garnison ou armée présente),
## signaux `recruit_into_requested` / `recruit_requested`. Le panneau est chargé à l'exécution
## (pas de typage statique : l'autoload IconLibrary doit être résolu).
## Usage : godot --headless --path game --script res://tests/wh_armyb_ui_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	var panel: Control = (load("res://scripts/map/settlement_panel.gd") as GDScript).new()
	root.add_child(panel)
	await process_frame
	if not check(panel.recruit_target != null and panel.sortie_button != null, "panel widgets missing"):
		panel.queue_free()
		return
	panel.settlement_id = "set_test"
	var armies := [
		{"id": "army_1", "name": "Armée du roi", "units": 7, "room": 12},
		{"id": "army_2", "name": "Armée pleine", "units": 40, "room": 0},
	]
	panel.call("_fill_recruit_target", armies)
	check(panel.recruit_target.visible and panel.recruit_target.item_count == 2, "garrison + one army with room")
	var got: Array = []
	panel.recruit_requested.connect(func(s: String, u: String) -> void: got.append(["garrison", s, u]))
	panel.recruit_into_requested.connect(func(s: String, u: String, a: String) -> void: got.append(["army", s, u, a]))
	panel.call("_on_recruit_pressed", "unit_knights")
	panel.recruit_target.select(1)
	panel.call("_on_recruit_pressed", "unit_knights")
	check(got == [["garrison", "set_test", "unit_knights"], ["army", "set_test", "unit_knights", "army_1"]], "signals: %s" % [got])
	panel.call("_fill_recruit_target", [])
	check(not panel.recruit_target.visible, "no army here: no selector")
	panel.queue_free()
