extends TestCase

## A6-L7 : panneau de colonie sans défilement imbriqué, recrutement groupé (U13), raison de
## construction sur sa ligne (U12), file de construction (M8). Maquette de données, sans simulation.
## `godot --headless --path game --script res://tests/a6_l7_panel_test.gd`


func _init() -> void:
	await process_frame
	var panel = load("res://scripts/map/settlement_panel.gd").new()  # chargé à l'exécution : les autoloads existent alors
	root.add_child(panel)
	var detail := {
		"id": "set_x", "province": "prov_x", "name": "Paris", "kind": "city", "owner": "fac_france",
		"controller": "fac_france", "buildings_info": [], "buildings": PackedStringArray(),
		"construction": {"building": "bld_a", "name": "Marché", "turns_left": 2},
		"build_queue": [{"building": "bld_b", "name": "Forge", "turns_left": 3}],
	}
	var recruits := [
		{"unit_type": "u1", "name": "Milice", "cost": 10, "upkeep": 1, "available": true, "group": "ready"},
		{"unit_type": "u2", "name": "Mamelouks", "cost": 10, "upkeep": 1, "available": false, "group": "elsewhere", "reason": "réservé à d'autres factions"},
		{"unit_type": "u3", "name": "Arbalétriers", "cost": 10, "upkeep": 1, "available": false, "group": "soon", "reason": "technologie requise : X"},
		{"unit_type": "u4", "name": "Lanciers", "cost": 10, "upkeep": 1, "available": false, "group": "blocked", "reason": "trésor insuffisant (10 livres nécessaires)"},
	]
	var buildable := [{"building": "bld_c", "name": "Halle", "cost": 5, "turns": 2, "available": false, "reason": "file de construction pleine (3 chantiers)"}]
	panel.show_settlement(detail, recruits, buildable, true)
	check(panel.find_children("*", "ScrollContainer", true, false).is_empty(), "no scroll inside the panel")
	var texts: Array = []
	for button in panel.recruit_list.find_children("*", "Button", true, false):
		texts.append(button.text)
	var joined := " | ".join(texts)
	check(joined.contains("Milice"), "ready unit listed")
	check(not joined.contains("Mamelouks"), "elsewhere unit hidden")
	check(joined.contains("Bientôt (1)"), "soon section header: %s" % joined)
	check(joined.find("Milice") < joined.find("Lanciers"), "ready before blocked")
	var soon_body := panel.recruit_list.find_child("SoonList", true, false) as Control
	check(soon_body != null and not soon_body.visible, "soon section folded by default")
	check(panel.queue_box.visible and panel.queue_box.get_child_count() == 1, "one queued building shown")
	check(panel.queue_box.find_child("CancelQueuedButton", true, false) != null, "queued entry has a cancel button")
	var fired: Array = []
	panel.cancel_queued_build_requested.connect(func(id: String, index: int) -> void: fired.append([id, index]))
	(panel.queue_box.find_child("CancelQueuedButton", true, false) as Button).pressed.emit()
	check(fired == [["set_x", 0]], "cancel queued emits settlement and index: %s" % str(fired))
	# U12 : la raison est un enfant direct de la liste (ligne pleine largeur), pas dans la ligne du bouton.
	var reason_direct := false
	for child in panel.buildable_list.get_children():
		if child is Label and (child as Label).text.begins_with("file de construction"):
			reason_direct = true
	check(reason_direct, "build reason on its own full-width line")
	print("a6_l7_panel_test: %s" % ("OK" if failures == 0 else "%d failure(s)" % failures))
	finish()
