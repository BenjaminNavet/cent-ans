extends SceneTree

## Captures H9 : section « La Table » d'une province française (infobulle d'un régime ouverte)
## et infobulle d'une technologie de la médecine (plantes, note historique).
## Usage (avec affichage, pas en headless) :
##   godot --path game --script res://tests/table_screenshot.gd
## Écrit `docs/img/table-section.png` et `docs/img/tech-medicine.png`.

const FACTION_ID := "fac_france"
const PROVINCE_ID := "prov_auvergne"  # au moins un régime indisponible (laitages)


func _init() -> void:
	await process_frame
	var out_dir := ProjectSettings.globalize_path("res://").path_join("../docs/img").simplify_path()
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var store: Node = root.get_node("/root/CodexStore")
	store.call("use_test_file")
	var sim: Object = ClassDB.instantiate("CampaignSim")
	sim.call("new_campaign", data_dir, FACTION_ID, 1337)

	var backdrop := ColorRect.new()
	backdrop.color = Color(0.16, 0.13, 0.10)
	backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(backdrop)

	# Panneau parchemin contenant la section (même thème que le panneau de province).
	var frame := PanelContainer.new()
	frame.theme = load("res://scenes/ui/parchment_theme.tres")
	frame.position = Vector2(40, 40)
	frame.custom_minimum_size = Vector2(400, 0)
	root.add_child(frame)
	var table := TableSection.new()
	frame.add_child(table)
	table.show_for(PROVINCE_ID, true, sim)
	table.options_box.show()
	for _i in 4:
		await process_frame
	var target := ""
	for option in table.options:
		if not bool(option.get("available", false)):
			target = str(option.get("id", ""))
			break
	if target == "" and not table.options.is_empty():
		target = str(table.options[0].get("id", ""))
	var tip := RichTooltip.make_panel(RichTooltip.diet(table._option(target)))
	root.add_child(tip)
	tip.position = Vector2(470, 120)
	for _i in 6:
		await process_frame
	_save(out_dir.path_join("table-section.png"))
	frame.queue_free()
	tip.queue_free()

	# Panneau des technologies sur l'onglet Médecine + infobulle du jardin des simples.
	var panel: Node = (load("res://scenes/ui/tech_panel.tscn") as PackedScene).instantiate()
	root.add_child(panel)
	await process_frame
	var tree: Array = sim.call("get_tech_tree", FACTION_ID)
	panel.call("show_tree", tree, {}, int(sim.call("get_research_points", FACTION_ID)), "France", Color(0.2, 0.3, 0.7))
	panel.call("select_branch", "medicine")
	for node in tree:
		if str(node.get("id", "")) == "tech_herb_garden":
			var tech_tip := RichTooltip.make_panel(RichTooltip.technology(node))
			root.add_child(tech_tip)
			tech_tip.position = Vector2(760, 300)
	for _i in 6:
		await process_frame
	_save(out_dir.path_join("tech-medicine.png"))
	store.call("reset_discoveries")
	quit(0)


func _save(path: String) -> void:
	var image := root.get_texture().get_image()
	var error := image.save_png(path)
	print("table screenshot %s: %s" % [path, "ok" if error == OK else "error %d" % error])
