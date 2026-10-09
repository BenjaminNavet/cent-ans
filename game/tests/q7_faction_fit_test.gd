extends TestCase

## Q7 : l'ajustement de l'écran des factions ne doit pas boucler (échelle → largeur → libellés
## repliés → taille minimale → nouvel ajustement) : en 1280×720 la sélection d'une faction
## saturait la file de messages et le jeu plantait. Compte les ajustements par sélection.
## Usage : godot --headless --path game --script res://tests/q7_faction_fit_test.gd

const MAX_FITS_PER_SELECT := 8


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	if DisplayServer.get_name() == "headless":
		root.size = Vector2i(1280, 720)
	else:
		var settings: Node = root.get_node_or_null("/root/Settings")
		settings.call("set_value", "video/fullscreen", false)
		settings.call("set_value", "video/resolution", Vector2i(1920, 1080))
		for _i in 10:
			await process_frame
		print("q7 window %s, viewport %s" % [DisplayServer.window_get_size(), root.get_visible_rect().size])
	change_scene_to_file("res://scenes/start_menu.tscn")
	for _i in 10:
		await process_frame
	var select: FactionSelect = current_scene.get("faction_select")
	select.visible = true
	for _i in 5:
		await process_frame
	var ids: Array = (select.get("_cards") as Dictionary).keys()
	ids.push_front("fac_france")
	for id in ids.slice(0, 8):
		select.fit_count = 0
		select.select(str(id))
		for _i in 6:
			await process_frame
		var fits := select.fit_count
		print("q7 fit: %s → %d fit(s), scale %.3f" % [id, fits, select.get("_content").scale.x])
		if fits > MAX_FITS_PER_SELECT:
			failures += 1
			printerr("FAIL q7: %s triggered %d fits (feedback loop)" % [id, fits])
	finish()
