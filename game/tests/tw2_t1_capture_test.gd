extends SceneTree

## Test headless du lot TW2-T1 (sort de la place prise) sur la vraie simulation et les vraies
## données, vérifié par l'état des nœuds (aucune capture d'écran) :
##  1. la France prend Bordeaux (cité de Guyenne) par le chemin commun de prise ; la décision en
##     attente ouvre la fenêtre enluminée (`ChronicleWindow`) avec quatre choix chiffrés ;
##  2. « Raser » est grisé sur une cité, avec la raison ;
##  3. un clic sur « Mettre à rançon » passe l'ordre : le trésor croît de l'or annoncé, la
##     décision disparaît, la fenêtre se ferme.
## Usage : godot --headless --path game --script res://tests/tw2_t1_capture_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("tw2_t1_capture_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("tw2_t1_capture_test: " + message)
	return condition


func _run() -> void:
	if not _check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_pending_captures"),
			"CampaignSim.get_pending_captures missing (run core/build.sh)"):
		return
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var sim: Object = map.sim
	if sim.has_method("set_chronicle_enabled"):
		sim.call("set_chronicle_enabled", false)
	await _test_capture_window(map, sim)
	map.queue_free()
	await process_frame


func _test_capture_window(map: Node, sim: Object) -> void:
	var controller: CaptureController = map.get("capture_fate")
	if not _check(controller != null, "campaign map has no CaptureController"):
		return
	_check((sim.call("get_pending_captures") as Array).is_empty(), "no capture should be pending at start")
	if not _check(sim.call("debug_capture_place", "prov_guyenne"), "debug capture of Guyenne refused"):
		return
	var captures: Array = sim.call("get_pending_captures")
	if not _check(captures.size() == 1, "one pending capture expected, got %d" % captures.size()):
		return
	var capture: Dictionary = captures[0]
	var options: Array = capture.get("options", [])
	_check(options.size() == 4, "four choices expected, got %d" % options.size())
	var outcomes := options.map(func(o: Dictionary) -> String: return str(o.get("outcome", "")))
	_check(outcomes == ["occupy", "ransom", "sack", "raze"], "choices in order occupy/ransom/sack/raze: %s" % [outcomes])
	# 1. La fenêtre s'ouvre d'office au rafraîchissement.
	map.refresh_all()
	await process_frame
	await process_frame
	var window: ChronicleWindow = controller.window
	if not _check(window.visible, "capture window should open after the capture"):
		return
	_check(window.current_decision() == int(capture["id"]), "window should show the pending capture")
	var title: Label = window.get("_title_label")
	_check(title.text == str(capture.get("title", "")) and title.text != "", "window title: %s" % title.text)
	var kind: Label = window.get("_kind_label")
	_check(kind.text.contains("place prise"), "kind label should name the capture: %s" % kind.text)
	var box: VBoxContainer = window.get("_options_box")
	var buttons: Array[Button] = []
	for row in box.get_children():
		buttons.append(row.get_child(0) as Button)
	if not _check(buttons.size() == 4, "four option rows expected, got %d" % buttons.size()):
		return
	# Effets chiffrés sous les boutons (or de la rançon et du pillage).
	var ransom_summary: Label = box.get_child(1).get_child(1) as Label
	_check(ransom_summary != null and ransom_summary.text.contains("Trésor"), "ransom effects should show the gold")
	# 2. Raser une cité : grisé, raison en clair.
	_check(buttons[3].disabled, "raze should be disabled on a city")
	_check(buttons[3].tooltip_text.begins_with("Impossible"), "raze tooltip should give the reason: %s" % buttons[3].tooltip_text)
	_check(not buttons[1].disabled, "ransom should be allowed")
	# 3. Rançon.
	var gold := int(options[1].get("gold", 0))
	_check(gold > 0, "ransom should bring gold")
	var before := int((sim.call("get_faction_summary", "fac_france") as Dictionary).get("treasury", 0)) if sim.has_method("get_faction_summary") else 0
	buttons[1].pressed.emit()
	await process_frame
	await process_frame
	_check((sim.call("get_pending_captures") as Array).is_empty(), "the capture should be decided")
	_check(not window.visible, "window should close once decided")
	if sim.has_method("get_faction_summary"):
		var after := int((sim.call("get_faction_summary", "fac_france") as Dictionary).get("treasury", 0))
		_check(after == before + gold, "treasury %d should grow by %d from %d" % [after, gold, before])
	if _failures == 0:
		print("tw2_t1_capture_test: « %s », rançon %d livres" % [str(capture.get("title", "")), gold])
