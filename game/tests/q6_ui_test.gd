extends SceneTree

## Test headless des corrections Q6 (recette du 2026-09-28), vérifié par l'état des nœuds :
##  1. choix de faction dans une vue de 1137×640 (fenêtre 1280×720 à l'échelle du jeu) :
##     « Retour » et « Commencer » restent entièrement dans la vue, onglet des cartes comme onglet
##     de la carte des factions.
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
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	await _test_faction_select_fits()


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
