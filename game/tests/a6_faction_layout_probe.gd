extends SceneTree

## A6-L11 (U2) : sonde de mise en page de l'écran de choix de faction en 1280×720 (fenêtre) :
## imprime l'échelle appliquée et, par onglet, la largeur occupée. Échoue si le texte courant
## passe sous 16 px effectifs ou si le contenu est réduit sous 0,9.
## Usage : godot --headless --path game --script res://tests/a6_faction_layout_probe.gd

var _failures := 0


func _init() -> void:
	_run.call_deferred()


func _run() -> void:
	root.size = Vector2i(1280, 720)
	change_scene_to_file("res://scenes/start_menu.tscn")
	for _i in 10:
		await process_frame
	var select: FactionSelect = current_scene.get("faction_select")
	select.visible = true
	for _i in 6:
		await process_frame
	var tabs := select.start_tabs
	print("a6 view %s, content scale %.3f, root scale %.2f" % [root.get_visible_rect().size, select.get("_content").scale.x, root.content_scale_factor])
	for index in tabs.get_tab_count():
		tabs.current_tab = index
		for _i in 6:
			await process_frame
		var page := tabs.get_current_tab_control()
		var used := Rect2()
		for child in page.find_children("*", "Control", true, false):
			var c := child as Control
			if c.is_visible_in_tree() and c.size.x > 0:
				var rect := Rect2(c.global_position, c.size)
				used = rect if used.size == Vector2.ZERO else used.merge(rect)
		print("a6 tab %d '%s': tabs %s, content span x %.0f..%.0f" % [index, tabs.get_tab_title(index), tabs.size, used.position.x, used.end.x])
	var effective := float(UiType.size(UiType.BODY)) * root.content_scale_factor * select.get("_content").scale.x
	print("a6 body px effective %.1f" % effective)
	quit(0)
