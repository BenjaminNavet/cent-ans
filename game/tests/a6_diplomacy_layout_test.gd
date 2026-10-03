extends SceneTree

## Test headless A6-L5 (U15, U16, M6) : le panneau de diplomatie tient dans 1280×720 (taille de base du
## viewport) sans qu'aucun bouton ne dépasse son cadre, avec des clauses ajoutées ; le bouton
## « Commerce » existe ; l'étiquette de verdict affiche un score signé, pas un pourcentage.
## Usage : godot --headless --path game --script res://tests/a6_diplomacy_layout_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("a6_diplomacy_layout_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("a6_diplomacy_layout_test: " + message)
	return condition


func _buttons(node: Node, out: Array) -> void:
	if node is BaseButton and (node as Control).is_visible_in_tree():
		out.append(node)
	for child in node.get_children():
		_buttons(child, out)


func _run() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null, "campaign map failed to start"):
		return
	var sim: Object = map.sim
	# Un SubViewport de 1280×720 : la taille de base du viewport, quel que soit l'écran de la machine.
	var sub := SubViewport.new()
	sub.size = Vector2i(1280, 720)
	sub.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(sub)
	var panel: Control = (load("res://scripts/ui/diplomacy_panel.gd") as GDScript).new()
	panel.set("sim", sim)
	panel.set("player_faction", "fac_france")
	panel.set("province_name_of", map.province_name_of)
	panel.set("map_data", map.map_data)
	panel.hide()
	var host := Control.new()
	host.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	sub.add_child(host)
	host.add_child(panel)
	panel.call("refresh")
	panel.call("select_faction", "fac_england")
	panel.show()
	for index in 6:
		await process_frame
	panel.set("_articles", [{"kind": "trade_agreement"}, {"kind": "gold", "giver": "proposer", "amount": 2500}])
	panel.call("_render_draft")
	for index in 6:
		await process_frame
	var rect := Rect2(panel.global_position, panel.size)
	var view := Rect2(Vector2.ZERO, panel.get_viewport_rect().size)
	_check(view.encloses(rect.grow(-0.5)), "panel %s exceeds the 1280x720 viewport" % rect)
	var buttons: Array = []
	_buttons(panel, buttons)
	var names := PackedStringArray()
	var found_trade := false
	for button in buttons:
		var control := button as Control
		var button_rect := control.get_global_rect()
		if control.name == "TradeButton":
			found_trade = true
		# Les rangées d'une liste défilante sortent légitimement du cadre (à la verticale).
		if _in_scroll(control) and button_rect.position.x >= rect.position.x - 1.0 and button_rect.end.x <= rect.end.x + 1.0:
			continue
		if not rect.grow(1.0).encloses(button_rect):
			names.append("%s %s" % [control.name, button_rect])
	_check(names.is_empty(), "buttons outside the panel %s : %s" % [rect, ", ".join(names.slice(0, 8))])
	_check(found_trade, "no « Commerce » button in the panel")
	var chance_label: Label = panel.get("_chance_label")
	_check(not chance_label.text.contains("%"), "verdict label still shows a percentage: " + chance_label.text)
	_check(chance_label.text.contains("(") and (chance_label.text.begins_with("Accepterait") or chance_label.text.begins_with("Refuserait")), "verdict label lacks signed score: " + chance_label.text)
	sub.queue_free()


func _in_scroll(node: Node) -> bool:
	var parent := node.get_parent()
	while parent != null:
		if parent is ScrollContainer:
			return true
		parent = parent.get_parent()
	return false

