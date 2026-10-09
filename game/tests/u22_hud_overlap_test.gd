extends TestCase

## U22 : à 1280x720, aucun panneau visible du HUD de bataille ne se chevauche, et la harangue
## (`BattleSpeech`) est en haut au centre, sous la barre de rapport de forces, sans recouvrir
## les panneaux de formations ni les cartes d'unités.
##
## Usage : godot --headless --path game --script res://tests/u22_hud_overlap_test.gd


func _init() -> void:
	await process_frame
	root.size = Vector2i(1280, 720)
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.autoplay = true
	root.add_child(scene)
	for _i in 8:
		await process_frame
	if scene.battle == null:
		push_error("u22: battle demo failed to stage (run core/build.sh?)")
		failures += 1
		finish()
		return
	var hud: CanvasLayer = scene.hud
	# Sélection de deux régiments : fait apparaître le panneau de formations / les ordres.
	var ids: Array[int] = []
	for unit in scene.battle.call("get_units"):
		if str(unit["side"]) == scene.player_side and ids.size() < 2:
			ids.append(int(unit["id"]))
	scene.selected = ids
	for _i in 6:
		await process_frame
	var panels: Array = _top_level_panels(hud.root)
	if scene._leader_bar != null and is_instance_valid(scene._leader_bar):
		for child in scene._leader_bar.get_children():
			panels.append(child)
	var rects: Array = []
	for panel in panels:
		var control := panel as Control
		if control == null or not control.is_visible_in_tree() or control.size.x < 4.0 or control.size.y < 4.0:
			continue
		if str(control.name).begins_with("UiZone_"):
			# Zone fixe : on compare le contenu réellement visible (union des panneaux posés).
			var content := _content_rect(control)
			if content.size == Vector2.ZERO:
				continue
			rects.append([control, content])
			continue
		rects.append([control, control.get_global_rect()])
	print("u22: %d visible panels" % rects.size())
	for i in rects.size():
		var a: Rect2 = rects[i][1]
		print("  %s %s" % [(rects[i][0] as Node).name, a])
		for j in range(i + 1, rects.size()):
			var b: Rect2 = rects[j][1]
			if a.intersection(b).get_area() > 4.0:
				failures += 1
				push_error("u22: panels overlap: %s %s / %s %s" % [(rects[i][0] as Node).name, a, (rects[j][0] as Node).name, b])
	# Harangue : bandeau en haut au centre, sous la barre de rapport de forces.
	var speech := BattleSpeech.new()
	speech.speaker = "Test"
	root.add_child(speech)
	speech._build_subtitle()
	speech._line_label.text = "Soldats, voici le jour ! Tenez bon, et que Dieu nous garde."
	await process_frame
	var band: Control = speech._band
	if band == null:
		failures += 1
		push_error("u22: speech band missing")
	else:
		await process_frame
		var band_rect := band.get_global_rect()
		print("  speech band %s" % band_rect)
		var bar_bottom := 0.0
		for entry in rects:
			if (entry[0] as Node).get_index() >= 0 and (entry[1] as Rect2).position.y < 20.0 and (entry[1] as Rect2).size.x > 600.0:
				bar_bottom = maxf(bar_bottom, (entry[1] as Rect2).end.y)
		if band_rect.position.y < bar_bottom - 0.5:
			failures += 1
			push_error("u22: speech overlaps the top bar (%s < %s)" % [band_rect.position.y, bar_bottom])
		if band_rect.end.y > root.get_visible_rect().size.y * 0.4:
			failures += 1
			push_error("u22: speech band too low: %s" % band_rect)
		if absf(band_rect.get_center().x - root.get_visible_rect().size.x * 0.5) > 2.0:
			failures += 1
			push_error("u22: speech band not centred: %s" % band_rect)
	finish()


## Enfants directs du HUD qui sont des panneaux (hors plein écran et conteneurs vides).
func _top_level_panels(hud_root: Control) -> Array:
	var out: Array = []
	for child in hud_root.get_children():
		var control := child as Control
		if control == null:
			continue
		var view := hud_root.get_viewport().get_visible_rect().size
		if control.size.x >= view.x * 0.9 and control.size.y >= view.y * 0.8:
			continue  # couche plein écran (repères d'unités, etc.)
		if control.name in ["UiZone_TOP_BAR", "UiZone_MODAL"]:
			continue  # la barre de rapport de forces est le panneau suivant ; MODAL = fenêtres
		out.append(control)
	return out


## Union des rectangles des `PanelContainer` visibles sous `zone` (vide si aucun).
func _content_rect(zone: Control) -> Rect2:
	var union := Rect2()
	var found := false
	var stack: Array = zone.get_children()
	while not stack.is_empty():
		var node: Node = stack.pop_back()
		if node is PanelContainer and (node as Control).is_visible_in_tree() and (node as Control).size.y > 4.0:
			var rect := (node as Control).get_global_rect()
			union = rect if not found else union.merge(rect)
			found = true
		else:
			stack.append_array(node.get_children())
	return union
