extends SceneTree

## Captures DA2 (portraits vivants) : panneau Cour (liste) et arbre familial de la France après
## une campagne avancée (personnages nés en jeu, défunts, figures vieillies).
## Usage (avec affichage, pas en headless) :
##   godot --resolution 1600x900 --path game --script res://tests/da2_screenshot.gd -- --out=<dossier> [--turns=N] [--tag=avant]
## Écrit `<dossier>/<tag>-cour.png` et `<dossier>/<tag>-arbre.png` (défaut : docs/img/da2, tag « apres »).

const DEFAULT_TURNS := 120  # quatre tours par an : 1367


func _init() -> void:
	await process_frame
	var out_dir := ProjectSettings.globalize_path("res://").path_join("../docs/img/da2").simplify_path()
	var turns := DEFAULT_TURNS
	var tag := "apres"
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--turns="):
			turns = int(arg.trim_prefix("--turns="))
		elif arg.begins_with("--tag="):
			tag = arg.trim_prefix("--tag=")
	DirAccess.make_dir_recursive_absolute(out_dir)
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var store: Node = root.get_node_or_null("/root/CodexStore")
	if store != null:
		store.call("use_test_file")
	root.size = Vector2i(1600, 900)
	var facade: Node = root.get_node("/root/SimFacade")
	var sim: Object = ClassDB.instantiate("CampaignSim")
	sim.call("new_campaign", data_dir, "fac_france", 1337)
	var started := Time.get_ticks_msec()
	for _turn in turns:
		sim.call("end_turn")
	print("da2: %d turns in %d ms, %s" % [turns, Time.get_ticks_msec() - started, sim.call("get_date_label")])
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
	print("da2: %d living French characters" % rows.size())
	court.show_court(rows, "France", facade.call("faction_color", "fac_france"))
	court.show_tab(0)  # CourtPanel.TAB_LIST
	for _i in 10:
		await process_frame
	_save(out_dir.path_join("%s-cour.png" % tag))
	court.show_tab(1)  # CourtPanel.TAB_TREE
	for _i in 10:
		await process_frame
	print("da2 tree: %d nodes, %d generations" % [court.family_tree.node_count(), court.family_tree.generation_count()])
	if OS.get_cmdline_user_args().has("--debug-nodes"):
		for node in court.family_tree.find_children("Node_*", "", true, false):
			var entry: Dictionary = node.get("entry")
			if not bool(node.get("_is_portrait")):
				print("da2 no portrait: ", entry, " -> ", LivingPortrait.resolve(entry, LivingPortrait.context_for(entry)))
	_save(out_dir.path_join("%s-arbre.png" % tag))
	if store != null:
		store.call("reset_discoveries")
	quit(0)


func _save(path: String) -> void:
	var image := root.get_texture().get_image()
	var error := image.save_png(path)
	print("da2 screenshot %s: %s" % [path, "ok" if error == OK else "error %d" % error])
