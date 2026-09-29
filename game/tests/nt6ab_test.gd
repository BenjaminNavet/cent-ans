extends SceneTree

## NT6a/NT6b « petites suites » :
##  1. discours du général adverse (composé pour le camp adverse, enchaîné après celui du joueur,
##     annulé si le joueur passe le premier) ;
##  2. indicateur « visé » des béliers et beffrois de siège (champ `target` des unités) ;
##  3. surprime des mercenaires dans le tableau du budget (et exposée par le pont) ;
##  4. en-tête du panneau de province : 5 lignes au plus en 1280×720, avis longs contenus.
## Usage : godot --headless --path game --script res://tests/nt6ab_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
## Hauteur maximale de l'en-tête (px du canevas) : 5 lignes de ~26 px, titre à lettrine compris.
const HEADER_MAX_HEIGHT := 150.0

var _failures := 0


func _init() -> void:
	await process_frame
	_check_speech()
	_check_targeted_marks()
	_check_premium_row()
	await _check_map()
	print("nt6ab_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("nt6ab_test: " + message)
	return condition


func _wait(frames: int) -> void:
	for _i in frames:
		await process_frame


# --- 1. Discours adverse -------------------------------------------------------------


func _check_speech() -> void:
	var setup := {
		"attacker": {"faction": "fac_france", "general": {"name": "Philippe VI de Valois"}},
		"defender": {"faction": "fac_england", "general": {"name": "Édouard III"}},
	}
	var enemy := BattleSpeech.compose(setup, "defender", 0.6, "hills", "clear", 7)
	_check(not enemy.is_empty() and not (enemy["lines"] as Array).is_empty(), "the enemy general has a speech")
	_check(str(enemy["speaker"]) == "Édouard III", "the enemy general speaks: %s" % enemy.get("speaker", ""))
	_check(str(enemy["cry"]) == "Saint George !", "enemy war cry: %s" % enemy.get("cry", ""))
	var ours := BattleSpeech.compose(setup, "attacker", 1.6, "hills", "clear", 7)
	_check(enemy["lines"] != ours["lines"], "the two speeches differ")
	var scene_script: GDScript = load("res://scripts/battle/battle_scene.gd")
	var methods := scene_script.get_script_method_list().map(func(m: Dictionary) -> String: return str(m["name"]))
	_check(methods.has("_start_enemy_speech") and methods.has("_advise_after_speeches"), "battle scene chains the enemy speech")
	var speech := BattleSpeech.new()
	_check(not speech.skipped, "a fresh speech is not skipped")
	speech.free()


# --- 2. Indicateur « visé » ----------------------------------------------------------


func _check_targeted_marks() -> void:
	var siege := BattleSiege.new()
	root.add_child(siege)
	var ram := Node3D.new()
	var tower := Node3D.new()
	siege.add_child(ram)
	siege.add_child(tower)
	siege._machines = {7: ram, 8: tower}
	var units := [
		{"id": 7, "side": "attacker", "present": true, "render": "ram", "x": 0.0, "z": 0.0, "target": -1},
		{"id": 8, "side": "attacker", "present": true, "render": "tower", "x": 5.0, "z": 0.0, "target": -1},
		{"id": 20, "side": "defender", "present": true, "render": "", "x": 0.0, "z": 9.0, "target": 7},
		{"id": 21, "side": "attacker", "present": true, "render": "", "x": 0.0, "z": 9.0, "target": 8},
	]
	siege._update_target_marks(units)
	_check(siege.targeted_ids == [7], "only the ram, aimed at by an enemy, is targeted: %s" % [siege.targeted_ids])
	_check(ram.get_node_or_null("TargetMark") != null and (ram.get_node("TargetMark") as Node3D).visible, "the ram carries a visible mark")
	_check(tower.get_node_or_null("TargetMark") == null, "a friendly target does not mark the tower")
	units[2]["target"] = -1
	units[3]["side"] = "defender"
	siege._update_target_marks(units)
	_check(siege.targeted_ids == [8], "the belfry is now targeted: %s" % [siege.targeted_ids])
	_check(not (ram.get_node("TargetMark") as Node3D).visible, "the ram mark disappears with the target")
	siege.queue_free()


# --- 3. Surprime des mercenaires -----------------------------------------------------


func _check_premium_row() -> void:
	var table := BudgetTable.new()
	root.add_child(table)
	var lines := [{"key": "receipts", "projected": 500, "charge": false}, {"key": "armies", "projected": -300, "charge": true}]
	table.show_budget({"budget_lines": lines, "net_income": 200, "net_income_last_turn": 180, "mercenary_premium": 120, "mercenary_premium_last_turn": 90})
	_check(table.cells.has("mercenary_premium"), "the premium line is shown")
	if table.cells.has("mercenary_premium"):
		var cell: Dictionary = table.cells["mercenary_premium"]
		_check(str(cell["projected"]).contains("120") and str(cell["last"]).contains("90"), "premium values: %s" % [cell])
	table.show_budget({"budget_lines": lines, "net_income": 200})
	_check(not table.cells.has("mercenary_premium"), "no premium, no line")
	table.queue_free()


# --- 4. En-tête du panneau de province, avis longs, pont ------------------------------


func _check_map() -> void:
	var settings: Node = root.get_node("/root/Settings")
	settings.call("use_test_file")
	settings.call("set_value", "tutorial/enabled", false, false)
	settings.call("set_value", "game/autosave_interval", 0, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	root.size = Vector2i(1280, 720)
	settings.call("set_value", "interface/ui_size", 1.0, false)
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _wait(10)
	var economy: Dictionary = map.sim.call("get_faction_economy", "fac_france")
	_check(economy.has("mercenary_premium") and economy.has("mercenary_premium_last_turn"), "the bridge exposes the mercenary premium")
	var capital := str(facade.faction_info(map.player_faction).get("capital", ""))
	map.flow.focus_province(capital)
	await _wait(20)
	var panel: Control = map.ui.province_panel
	if _check(panel.is_visible_in_tree(), "province panel open"):
		var header: float = panel.tabs.get_global_rect().position.y - panel.get_node("VBox").get_global_rect().position.y
		_check(header <= HEADER_MAX_HEIGHT, "province header is %.0f px tall (max %.0f)" % [header, HEADER_MAX_HEIGHT])
		_check(panel.tabs.get_global_rect().size.y >= 250.0, "tabs keep room: %.0f px" % panel.tabs.get_global_rect().size.y)
	# Avis très longs : chaque avis reste dans la zone (retour à la ligne, hauteur bornée).
	var long_text := "Un avis très long qui se répète. ".repeat(30)
	for _i in 3:
		map.ui.show_toast(long_text)
	await _wait(8)
	var zone: Control = map.ui.get_node("UiZone_TOASTS")
	var zone_rect := zone.get_global_rect()
	var shown := 0
	for entry: Control in UiZones.layout().toasts():
		if not entry.visible:
			continue
		shown += 1
		var rect := entry.get_global_rect()
		_check(rect.end.x <= zone_rect.end.x + 0.5 and rect.end.y <= zone_rect.end.y + 0.5, "toast %s exceeds the zone %s" % [rect, zone_rect])
		var label := entry.find_child("Text", true, false) as Label
		_check(label.get_visible_line_count() <= UiLayout.TOAST_MAX_LINES, "toast shows %d lines" % label.get_visible_line_count())
	_check(shown > 0, "toasts shown")
	map.queue_free()
	await process_frame
