extends SceneTree

## Q8 : « Commencer » doit lancer la faction choisie (recette : Angleterre choisie, partie
## lancée en Transylvanie). Choisit chaque départ recommandé, presse le bouton, lit la faction
## transmise à `SimFacade`.
## Usage : godot --headless --path game --script res://tests/q8_start_faction_test.gd

var _failures := 0


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	root.size = Vector2i(1280, 720)
	var settings: Node = root.get_node_or_null("/root/Settings")
	settings.call("set_value", "battle_prologue/never_ask", true)
	for id in ["fac_england", "fac_burgundy", "fac_france"]:
		change_scene_to_file("res://scenes/start_menu.tscn")
		for _i in 10:
			await process_frame
		var menu: Node = current_scene
		var select: FactionSelect = menu.get("faction_select")
		menu.call("open_faction_select") if menu.has_method("open_faction_select") else null
		select.visible = true
		for _i in 5:
			await process_frame
		select.select(id)
		for _i in 5:
			await process_frame
		(menu.get("start_button") as Button).pressed.emit()
		for _i in 5:
			await process_frame
		var pending := str(root.get_node("SimFacade").get("pending_faction"))
		print("q8 start %s -> pending %s (selected %s)" % [id, pending, select.selected_faction])
		if pending != id:
			_failures += 1
			push_error("q8: chose %s, starting %s" % [id, pending])
	print("q8 start faction: %s" % ("OK" if _failures == 0 else "%d FAILURES" % _failures))
	quit(1 if _failures > 0 else 0)
