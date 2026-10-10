extends TestCase

## Lot UX5-T : savoirs. Panneau instancié seul, données synthétiques (sans simulation).
## Usage : godot --headless --path game --script res://tests/ux5_t_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _node(id: String, branch: String, tier: int, state: String, prereqs: Array, units: Array = [], buildings: Array = [], extra: Dictionary = {}, unit_ids: Array = []) -> Dictionary:
	var node := {"id": id, "name": "Savoir " + id, "branch": branch, "tier": tier, "state": state, "cost": 100,
		"effective_cost": 100, "progress": 0, "prerequisites": prereqs, "queue_position": 0,
		"unlock_names": {"units": units, "buildings": buildings},
		"unlocks": {"units": unit_ids, "buildings": []},
		"effects": [{"kind": "army_armor", "value": 2.0, "mode": "add"}]}
	node.merge(extra, true)
	return node


func _run() -> void:
	var panel: Control = (load("res://scenes/ui/tech_panel.tscn") as PackedScene).instantiate()
	root.add_child(panel)
	await process_frame
	var tree: Array = [
		_node("a", "military", 1, "known", []),
		_node("b", "military", 2, "available", ["a"], ["Arbalétrier"], ["Forge"], {}, ["unit_knights"]),
		_node("c", "military", 3, "locked", ["b", "x"], [], [], {}),
		_node("d", "military", 4, "locked", ["c"]),
		_node("e", "civil", 1, "researching", [], [], ["Moulin"], {"progress": 40}),
		_node("f", "civil", 2, "available", ["e"], [], [], {"queue_position": 1}),
		_node("g", "medicine", 1, "available", []),
	]
	tree.append(_node("x", "military", 1, "available", []))
	var research := {"technology": "e", "name": "Savoir e", "progress": 40, "cost": 100, "points_per_turn": 20, "turns_left": 3}
	var queue: Array = [{"id": "f", "name": "Savoir f", "cost": 100}, {"id": "g", "name": "Savoir g", "cost": 60}]
	panel.show_tree(tree, research, 20, "France", Color.WHITE, queue, {}, 7)
	await process_frame
	await process_frame
	var tree_script: Script = load("res://scripts/ui/tech_tree_view.gd")
	var tip_script: Script = load("res://scripts/ui/rich_tooltip.gd")
	var mil: Control = panel.military_view
	# T1 : pastilles et effet clé
	var foot := mil.buttons["b"].get_node("Foot") as Control
	var pill_line := foot.get_node("Pills") as Control
	var texts := PackedStringArray()
	for label in pill_line.get_children():
		if label is Label:
			texts.append((label as Label).text)
	var pills := " | ".join(texts)
	# Arbalétrier a une icône (id connu) : plus de glyphe ; Forge sans id garde ⛫.
	check(pill_line.get_node_or_null("PillIcon") != null, "unlock icon missing")
	check(pills.contains("Arbalétrier") and not pills.contains("⚔") and pills.contains("⛫ Forge"), "pills: " + pills)
	check((foot.get_node("Effect") as Label).text.contains("Armure"), "effect line missing")
	check(tree_script.NODE_SIZE.y >= 80.0 and tree_script.NODE_SIZE.y <= 96.0, "node height")
	# T7 : glyphes d'état
	check(str(mil.buttons["a"].get_meta("glyph")) != "", "known node needs a glyph")
	check(str(mil.buttons["c"].get_meta("glyph")) != "", "locked node needs a glyph")
	check(str(panel.civil_view.buttons["f"].get_meta("glyph")) == "n°1", "queued glyph")
	check((mil.buttons["a"] as Button).text.begins_with(str(mil.buttons["a"].get_meta("glyph"))), "glyph in text")
	check(not tree_script.state_fill("known").is_equal_approx(tree_script.state_fill("locked")), "state fills differ")
	# T2 : chemin surligné
	check(mil.missing_requirements("c") == PackedStringArray(["Savoir b", "Savoir x"]), "missing requirements: " + str(mil.missing_requirements("c")))
	mil.set_focus("c")
	check(mil._path.has("b") and mil._path.has("x") and mil._path.has("d") and not mil._path.has("a"), "path ancestors/descendants: " + str(mil._path.keys()))
	check(not mil.is_dimmed("b") and mil.is_dimmed("a"), "non-path nodes are dimmed")
	mil.set_focus("")
	check(not mil.is_dimmed("a"), "dim cleared")
	check(tree_script.tooltip_for({"name": "N", "lock_note": "Verrouillée : requiert A"}).begins_with("Verrouillée : requiert A"), "tooltip locked first")
	var tip_node := {"id": "c", "state": "locked", "prerequisites": [], "lock_note": "Verrouillée : requiert A"}
	check((tip_script.technology_spec(tip_node)["warnings"] as Array)[0] == "Verrouillée : requiert A", "rich tooltip locked")
	# T3 : filtres
	panel.search_edit.text = "forge"
	panel.search_edit.text_changed.emit("forge")
	check(not mil.is_dimmed("b") and mil.is_dimmed("a") and mil.is_dimmed("c"), "search keeps matching node")
	panel.search_edit.text = ""
	panel.search_edit.text_changed.emit("")
	panel.reach_toggle.button_pressed = true
	check(not mil.is_dimmed("b") and mil.is_dimmed("a") and mil.is_dimmed("c"), "reach filter")
	panel.reach_toggle.button_pressed = false
	panel.unlock_toggle.button_pressed = true
	check(not mil.is_dimmed("b") and mil.is_dimmed("x"), "unlock filter")
	panel.unlock_toggle.button_pressed = false
	panel.fade_known_toggle.button_pressed = true
	check(mil.is_dimmed("a") and not mil.is_dimmed("b"), "fade known")
	panel.fade_known_toggle.button_pressed = false
	check(not mil.is_dimmed("a"), "filters cleared")
	check(mil.buttons["a"].position == Vector2(12, 34 + 0), "layout not altered by filters" if mil.buttons["a"].position.x == 12 else "layout")
	# T4 : ruban
	check(panel.ribbon.visible, "ribbon visible")
	var slot0: Dictionary = panel._slots[0]
	check((slot0["name"] as Label).text.contains("Savoir e") and (slot0["turns"] as Label).text == "3 tours", "slot 0: " + (slot0["turns"] as Label).text)
	check(((panel._slots[1] as Dictionary)["turns"] as Label).text.contains("8 tours"), "slot 1 cumulative: " + ((panel._slots[1] as Dictionary)["turns"] as Label).text)
	check((panel.ribbon_end_label.text) == "Fin de la file : tour 18", panel.ribbon_end_label.text)
	check(not panel.idle_label.visible, "idle hidden while researching")
	# T5 : ouverture
	check(panel.focus_start(research) == "e" and panel.tabs.current_tab == 1, "opens on research branch")
	check(panel.focus_start({}) == "b" and panel.tabs.current_tab == 0, "first available otherwise")
	panel.show_tree(tree, {}, 20, "France", Color.WHITE, [], {"points": 10, "cap": 100}, 7)
	check(panel.idle_label.visible and panel.idle_label.text == "Aucun savoir à l'étude", "idle mention")
	check(not panel.ribbon.visible, "ribbon hidden with no queue")
	panel.queue_free()
