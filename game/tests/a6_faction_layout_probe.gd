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
	var content := select.get("_content") as Control
	print("a6 min sizes: content %s" % content.get_combined_minimum_size())
	var first_card := (select.get("_cards") as Dictionary).values()[0] as Control
	print("a6 card count %d, first card min %s" % [(select.get("_cards") as Dictionary).size(), first_card.get_combined_minimum_size()])
	for node in first_card.find_children("*", "Control", true, false):
		var cc := node as Control
		if cc.get_combined_minimum_size().x > 150:
			print("a6   card child %s %s min %s" % [cc.get_class(), cc.name, cc.get_combined_minimum_size()])
	for node in content.find_children("*", "Control", true, false):
		var c := node as Control
		if c.name in ["ScreenBody", "SideColumn", "StartTabs", "DetailScroll", "Départs recommandés", "SpecialCards", "MapBody"] or c.get_parent() == content.get_child(0):
			print("a6   %s: min %s" % [c.name, c.get_combined_minimum_size()])
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
		print("a6 tab %d %s: tabs %s, span x %.0f..%.0f, scale %.3f" % [index, tabs.get_tab_title(index), tabs.size, used.position.x, used.end.x, (select.get("_content") as Control).scale.x])
	var content_scale: float = (select.get("_content") as Control).scale.x
	var effective: float = float(FactionSelect.BODY_PX) * root.content_scale_factor * content_scale
	print("a6 body px effective %.1f" % effective)
	if effective < 16.0:
		_failures += 1
		printerr("FAIL a6: body text %.1f px < 16" % effective)
	quit(1 if _failures > 0 else 0)
