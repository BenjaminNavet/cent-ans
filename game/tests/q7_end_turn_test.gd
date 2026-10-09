extends TestCase

## Q7 : Entrée (`campaign_end_turn`) finit la saison même quand la fiche d'une ville ou le rapport
## de saison est ouvert (la recette voyait une fin de tour sur deux ignorée).
## Usage : godot --headless --path game --script res://tests/q7_end_turn_test.gd


var map: Node = null


func _init() -> void:
	_run.call_deferred()


func _wait(frames: int) -> void:
	for _i in frames:
		await process_frame


func _date() -> String:
	return str(map.sim.call("get_date_label"))


func _press_enter() -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = KEY_ENTER
		event.physical_keycode = KEY_ENTER
		event.pressed = pressed
		Input.parse_input_event(event)
		Input.flush_buffered_events()
		await _wait(2)
	await _wait(20)


func _run() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	settings.call("use_test_file")
	settings.call("set_value", "tutorial/enabled", false, false)
	settings.call("set_value", "game/autosave_interval", 0, false)
	settings.call("set_value", "interface/season_report", true, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	root.size = Vector2i(1280, 720)
	map = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _wait(10)
	var tutorial := map.find_child("Tutorial", true, false) as CanvasItem
	if tutorial != null:
		tutorial.hide()

	var before := _date()
	await _press_enter()
	check(_date() != before, "Enter on the bare map should end the season (still %s)" % before)

	# Fiche de Paris ouverte.
	map.settlements_ctl.open_settlement("set_paris")
	await _wait(5)
	check(map.settlements_ctl.panel.is_visible_in_tree(), "Paris panel should be open")
	before = _date()
	await _press_enter()
	check(_date() != before, "Enter with the settlement panel open should end the season (still %s, panels %s)" % [before, _panel_names()])

	# Rapport de saison ouvert.
	var report: Control = map.flow.season_report
	if not report.visible:
		report.show_report(_date(), [])
		report.show()
	await _wait(3)
	before = _date()
	await _press_enter()
	check(_date() != before, "Enter with the season report open should end the season (still %s, panels %s)" % [before, _panel_names()])

	# Relecture du tour de l'IA en cours : Entrée la passe, la saison ne change pas.
	var replay: Node = map.ai_replay
	if check(replay != null, "no AiTurnReplay"):
		replay.playing = true
		replay.set("_skip", false)
		before = _date()
		await _press_enter()
		check(bool(replay.get("_skip")) or not replay.playing, "Enter during the AI replay should skip it")
		check(_date() == before, "Enter during the AI replay must not start another season")
		replay.playing = false

	finish()


func _panel_names() -> Array:
	var names: Array = []
	for panel in map.ui.panels.visible_panels():
		names.append(str((panel as Node).name))
	return names
