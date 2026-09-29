extends SceneTree

## Test headless des corrections Q6 (recette du 2026-09-28), vérifié par l'état des nœuds :
##  1. choix de faction dans une vue de 1137×640 (fenêtre 1280×720 à l'échelle du jeu) :
##     « Retour » et « Commencer » restent entièrement dans la vue, onglet des cartes comme onglet
##     de la carte des factions ;
##  2. fenêtre de décision (`ChronicleWindow`) : le corps défilant a la hauteur de son contenu
##     (il s'ouvrait vide, choix inaccessibles) et les choix sont cliquables à l'écran ;
##  3. barre du haut de la carte, fenêtre 1280×720 en taille d'interface 1,25 (vue 1137×640) :
##     elle tient dans l'écran (palier compact), et reprend ses libellés longs à 1920×1080 ;
##  4. fin de tour : la diplomatie ne s'ouvre seule que pour une offre nouvelle de poids (pas pour
##     un accord commercial, pas quand une décision de chronique attend).
## Usage : godot --headless --path game --script res://tests/q6_ui_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const SMALL_VIEW := Vector2(1137.0, 640.0)

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("q6_ui_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("q6_ui_test: " + message)
	return condition


func _run() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	settings.call("use_test_file")
	settings.call("set_value", "tutorial/enabled", false, false)
	settings.call("set_value", "game/autosave_interval", 0, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	await _test_faction_select_fits()
	await _test_chronicle_body_visible()
	await _test_top_bar_fits()
	_test_diplomacy_auto_open()


func _test_faction_select_fits() -> void:
	var host := Control.new()
	host.size = SMALL_VIEW
	root.add_child(host)
	var select := FactionSelect.new()
	host.add_child(select)
	for tab in [0, 1]:
		select.start_tabs.current_tab = tab
		for _i in 6:
			await process_frame
		var view := Rect2(host.global_position, SMALL_VIEW)
		for button: Button in [select.start_button, select.back_button]:
			var rect := button.get_global_rect()
			_check(view.encloses(rect), "tab %d: « %s » %s outside the %s view" % [tab, button.text, rect, SMALL_VIEW])
	host.queue_free()
	await process_frame


func _test_chronicle_body_visible() -> void:
	var host := Control.new()
	host.size = Vector2(1280.0, 720.0)
	root.add_child(host)
	var window := ChronicleWindow.new()
	host.add_child(window)
	await process_frame
	var options := []
	for i in 3:
		options.append({"index": i, "text": "Choix %d" % i, "effects_text": "Trésor -100"})
	var decision := {"id": 7, "title": "Un festin", "text": "La cour attend.", "event": "", "kind": "", "options": options}
	for pass_index in 2:  # deux décisions de suite : la borne ne doit pas rester à zéro
		window.show_decision(decision, 2)
		for _i in 4:
			await process_frame
		var scroll: ScrollContainer = window.get("_scroll")
		_check(scroll.size.y > 100.0, "pass %d: decision body height %.0f (empty window)" % [pass_index, scroll.size.y])
		var view := Rect2(host.global_position, host.size)
		var buttons := window.find_children("*", "Button", true, false)
		var choices := buttons.filter(func(b: Button) -> bool: return b.text.begins_with("Choix"))
		_check(choices.size() == 3, "pass %d: 3 choices expected, got %d" % [pass_index, choices.size()])
		for button: Button in choices:
			var rect := button.get_global_rect()
			_check(view.encloses(rect) and scroll.get_global_rect().intersects(rect), "pass %d: « %s » %s not visible" % [pass_index, button.text, rect])
	host.queue_free()
	await process_frame


func _test_top_bar_fits() -> void:
	var settings: Node = root.get_node("/root/Settings")
	root.size = Vector2i(1280, 720)
	settings.call("set_value", "interface/ui_size", 1.25, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for _i in 10:
		await process_frame
	var top_bar := map.ui.get_node("TopBar") as Control
	map.ui.fit_top_bar()
	await process_frame
	var view := root.get_visible_rect().size
	var rect := top_bar.get_global_rect()
	_check(rect.position.x >= -0.5 and rect.end.x <= view.x + 0.5, "top bar %s overflows the %s view" % [rect, view])
	_check(not map.ui.treasury_label.text.begins_with("Trésor"), "narrow view: compact treasury label expected, got « %s »" % map.ui.treasury_label.text)
	root.size = Vector2i(1920, 1080)
	settings.call("set_value", "interface/ui_size", 1.0, false)
	for _i in 4:
		await process_frame
	map.ui.fit_top_bar()
	_check(map.ui.treasury_label.text.begins_with("Trésor"), "wide view: full treasury label expected, got « %s »" % map.ui.treasury_label.text)
	map.queue_free()
	await process_frame


class DiplomacyProbe extends "res://scripts/map/diplomacy_controller.gd":
	var opened := 0

	func open_panel(_faction_id: String = "") -> void:
		opened += 1


class SimStub extends RefCounted:
	var offers: Array = []
	var decisions: Array = []

	func get_diplomacy() -> Dictionary:
		return {}

	func get_offers() -> Array:
		return offers

	func get_pending_decisions() -> Array:
		return decisions


class UiStub extends Node:
	func show_toast(_text: String, _warning: bool = false) -> void:
		pass


class MapStub extends Node:
	var sim: Object = null
	var ui: Node = null
	var chronicle: Node = null


func _test_diplomacy_auto_open() -> void:
	var map := MapStub.new()
	var sim := SimStub.new()
	map.sim = sim
	map.ui = UiStub.new()
	var ctl := DiplomacyProbe.new()
	ctl.map = map
	sim.offers = [{"id": 1, "kind": "treaty"}, {"id": 2, "kind": "treaty"}]
	ctl.after_end_turn()
	_check(ctl.opened == 0, "diplomacy opened for routine trade offers")
	sim.offers.append({"id": 3, "kind": "peace"})
	sim.decisions = [{"id": 9}]
	ctl.after_end_turn()
	_check(ctl.opened == 0, "diplomacy opened over a pending chronicle decision")
	sim.offers.append({"id": 4, "kind": "alliance"})
	sim.decisions = []
	ctl.after_end_turn()
	_check(ctl.opened == 1, "diplomacy should open for a fresh alliance offer (opened %d)" % ctl.opened)
	ctl.after_end_turn()
	_check(ctl.opened == 1, "diplomacy reopened without a fresh offer")
	map.ui.free()
	map.free()
	ctl.free()
