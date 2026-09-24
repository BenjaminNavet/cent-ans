extends SceneTree

## Captures C3 : arbre familial des Valois (onglet du panneau Cour, ≥ 3 générations après
## quelques années de campagne) et fiche d'Édouard III (Angleterre).
## Usage (avec affichage, pas en headless) :
##   godot --path game --script res://tests/c3_screenshot.gd
## Écrit `docs/img/c3/arbre-valois.png` et `docs/img/c3/fiche-edouard-iii.png`.

const TURNS := 13  # hiver 1339 : Charles V et Louis d'Anjou sont nés


func _init() -> void:
	await process_frame
	var out_dir := ProjectSettings.globalize_path("res://").path_join("../docs/img/c3").simplify_path()
	DirAccess.make_dir_recursive_absolute(out_dir)
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var store: Node = root.get_node_or_null("/root/CodexStore")
	if store != null:
		store.call("use_test_file")
	root.size = Vector2i(1440, 900)
	var facade: Node = root.get_node("/root/SimFacade")
	var sim: Object = ClassDB.instantiate("CampaignSim")
	sim.call("new_campaign", data_dir, "fac_france", 1337)
	for _turn in TURNS:
		sim.call("end_turn")
	facade.set("sim", sim)

	var backdrop := ColorRect.new()
	backdrop.color = Color(0.16, 0.13, 0.10)
	backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(backdrop)
	var layer := Control.new()
	layer.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(layer)

	var court: Node = (load("res://scenes/ui/court_panel.tscn") as PackedScene).instantiate()
	layer.add_child(court)
	await process_frame
	var rows: Array[Dictionary] = []
	for id in sim.call("get_faction_characters", "fac_france"):
		rows.append(sim.call("get_character", id))
	court.show_court(rows, "France", facade.call("faction_color", "fac_france"))
	court.show_tab(1)  # CourtPanel.TAB_TREE
	for _i in 8:
		await process_frame
	print("c3 tree: %d nodes, %d generations" % [court.family_tree.node_count(), court.family_tree.generation_count()])
	_save(out_dir.path_join("arbre-valois.png"))
	court.hide()

	var sheet: Node = (load("res://scenes/ui/character_sheet.tscn") as PackedScene).instantiate()
	layer.add_child(sheet)
	await process_frame
	# Fiche d'Édouard III : campagne jouée par l'Angleterre (l'ordre `learn_skill` n'est permis
	# que sur un personnage de sa faction) ; deux compétences apprises.
	var england: Object = ClassDB.instantiate("CampaignSim")
	england.call("new_campaign", data_dir, "fac_england", 1337)
	facade.set("sim", england)
	var edward := "chr_edward_iii"
	england.call("submit_order", {"type": "debug_grant_xp", "character": edward, "amount": 600})
	for skill in ["skill_archerie", "skill_tir_a_volonte"]:
		var result: Dictionary = england.call("submit_order", {"type": "learn_skill", "character": edward, "skill": skill})
		print("c3 learn %s: %s" % [skill, result])
	var character: Dictionary = england.call("get_character", edward)
	var skill_tree: Array = england.call("get_skill_tree")
	var learnable: Array = england.call("get_learnable", edward)
	sheet.show_character(character, skill_tree, learnable, [], [], [])
	for _i in 8:
		await process_frame
	_save(out_dir.path_join("fiche-edouard-iii.png"))
	if store != null:
		store.call("reset_discoveries")
	quit(0)


func _save(path: String) -> void:
	var image := root.get_texture().get_image()
	var error := image.save_png(path)
	print("c3 screenshot %s: %s" % [path, "ok" if error == OK else "error %d" % error])
