extends SceneTree

## Test headless Q6 (recette du 2026-09-28) autour du panneau de diplomatie, vérifié par l'état
## des nœuds (pas de capture), dans trois configurations d'écran :
##  - fenêtre 1920×1080, taille d'interface 1,25 (vue logique 1280×720, réglage réel du joueur) ;
##  - fenêtre 1280×720, taille d'interface 1,0 ; fenêtre 1280×640, taille d'interface 1,0.
## Vérifie :
##  1. après choix d'une faction, ajout d'une clause « Accord commercial » et d'une contre-
##     proposition, « Proposer le traité » (`SendTreaty`) est entièrement dans la vue et reçoit
##     la souris (le contrôle sous son centre est lui) ;
##  2. fin de tour avec proposition reçue : la diplomatie s'ouvre et le rapport de saison dans la
##     même image ; le « Continuer » du rapport est au-dessus du panneau et reçoit la souris.
## Usage : godot --headless --path game --script res://tests/q6_diplomacy_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const CONFIGS := [
	[Vector2i(1920, 1080), 1.25],
	[Vector2i(1280, 720), 1.0],
	[Vector2i(1280, 640), 1.0],
]

var _failures := 0
var map: Node3D = null


func _init() -> void:
	await process_frame
	await _run()
	print("q6_diplomacy_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("q6_diplomacy_test: " + message)
	return condition


func _wait(frames: int) -> void:
	for _i in frames:
		await process_frame


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
	root.size = CONFIGS[0][0]
	settings.call("set_value", "interface/ui_size", CONFIGS[0][1], false)
	map = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _wait(10)
	var tutorial := map.find_child("Tutorial", true, false) as CanvasItem
	if tutorial != null:
		tutorial.hide()  # Q7 : le panneau du didacticiel n'est pas l'objet de ce test
	for config in CONFIGS:
		root.size = config[0]
		settings.call("set_value", "interface/ui_size", config[1], false)
		await _wait(4)
		var label := "%dx%d @%.2f" % [config[0].x, config[0].y, config[1]]
		await _test_send_treaty_reachable(label)
		await _test_report_above_diplomacy(label)
	settings.call("set_value", "interface/ui_size", 1.0, false)
	map.queue_free()
	await process_frame


## Contrôle sous le point `point` de la vue logique (souris déplacée comme par un joueur).
func _control_at(point: Vector2) -> Control:
	var window_point := root.get_final_transform() * point
	var motion := InputEventMouseMotion.new()
	motion.position = window_point
	motion.global_position = window_point
	Input.parse_input_event(motion)
	Input.flush_buffered_events()
	await _wait(2)
	return root.gui_get_hovered_control()


func _check_reachable(button: Control, what: String) -> void:
	var view := root.get_visible_rect()
	var rect := Rect2(button.get_global_transform_with_canvas().origin, button.size * button.get_global_transform_with_canvas().get_scale())
	_check(view.encloses(rect.grow(-0.5)), "%s %s outside the %s view" % [what, rect, view.size])
	var hit: Control = await _control_at(rect.get_center())
	_check(hit == button or (hit != null and button.is_ancestor_of(hit)), "%s: click at %s hits %s" % [what, rect.get_center(), hit.get_path() if hit != null else "nothing"])


func _test_send_treaty_reachable(label: String) -> void:
	await _test_send_treaty_for(label, "fac_albret", "Accord commercial")
	# Q7 : ennemi en guerre, trêve refusée — longue liste de raisons sous la chance d'acceptation.
	await _test_send_treaty_for(label, "fac_england", "Trêve de deux ans")


func _test_send_treaty_for(label: String, target: String, clause: String) -> void:
	label = "%s %s" % [label, target]
	var controller: Node = map.diplomacy  # Q7 : non typé (compilé avant les autoloads)
	controller.open_panel(target)
	await _wait(10)
	var panel: Control = controller.panel
	if not _check(str(panel.get("_selected")) == target, "%s: %s not selected" % [label, target]):
		return
	var clause_menu: MenuButton = panel.get("negotiation").get("clause_menu")
	panel.get("negotiation").call("fill_menu", clause_menu)
	var popup := clause_menu.get_popup()
	var picked := false
	for index in popup.item_count:
		if popup.get_item_text(index) == clause:
			popup.id_pressed.emit(popup.get_item_id(index))
			picked = true
	_check(picked, "%s: « %s » clause missing" % [label, clause])
	await _wait(5)
	panel.get("negotiation").call("ask_counter")
	await _wait(10)
	var send := panel.find_child("SendTreaty", true, false) as Button
	if _check(send != null and send.is_visible_in_tree(), "%s: SendTreaty missing or hidden" % label):
		await _check_reachable(send, "%s: « Proposer le traité »" % label)
	for text in ["Que faudrait-il ?", "Effacer"]:
		for button in panel.find_children("*", "Button", true, false):
			if (button as Button).text == text and (button as Button).is_visible_in_tree():
				await _check_reachable(button, "%s: « %s »" % [label, text])
	panel.hide()
	await _wait(4)


func _test_report_above_diplomacy(label: String) -> void:
	var controller: Node = map.diplomacy  # Q7 : non typé (compilé avant les autoloads)
	var report: SeasonReport = map.flow.season_report
	# Même image, même ordre que `campaign_map` après `end_turn` : diplomatie puis rapport.
	controller.open_panel()
	report.show_report("Test", SeasonReport.build_groups([
		{"kind": "plague", "text_fr": "La peste frappe la province.", "province": "", "faction": map.player_faction}],
		func(_e: Dictionary) -> bool: return true, Callable(), map.player_faction))
	await _wait(10)
	_check(controller.panel.visible and report.visible, "%s: diplomacy %s, report %s (both expected open)" % [label, controller.panel.visible, report.visible])
	var ok: Button = null
	for button in report.find_children("*", "Button", true, false):
		if (button as Button).text == "Continuer":
			ok = button
	if _check(ok != null, "%s: report « Continuer » missing" % label):
		await _check_reachable(ok, "%s: report « Continuer »" % label)
	report.close()
	controller.panel.hide()
	await _wait(4)
