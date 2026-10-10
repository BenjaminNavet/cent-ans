extends SceneTree

## Captures de contrôle du chantier WR (fenêtré, via tools/godot_bg.sh) : offre de mission, barre de genres
## du journal, bouton « Exécuter » armé du panneau des captifs, dialogue de sortie.
## Usage : tools/godot_bg.sh --path game --script res://tests/wr_shot.gd -- --out=/chemin/absolu


const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _frames(count: int) -> void:
	for _i in count:
		await process_frame


func _shot(path: String) -> void:
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(path)
	print("wr_shot: -> %s" % path)


func _init() -> void:
	var out := CmdArgs.value("--out", "user://wr_shot")
	DirAccess.make_dir_recursive_absolute(out)
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "feudal_tutorial/done", true, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	root.size = Vector2i(1280, 720)
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _frames(10)
	var sim: Object = map.sim
	sim.call("end_turn")
	await _frames(30)
	# 1. Offre de mission.
	map.mission_offer.open_window()
	await _frames(30)
	await _shot(out.path_join("wr_missions.png"))
	map.mission_offer.window.hide()
	# 2. Journal ouvert, filtré sur la guerre.
	var journal: JournalView = map.ui.journal
	journal.set_expanded(true)
	journal.set_genre_filter("war")
	await _frames(30)
	await _shot(out.path_join("wr_journal.png"))
	journal.set_genre_filter("all")
	journal.set_expanded(false)
	# 3. Captifs : bouton « Exécuter » armé.
	var panel := RansomPanel.new()
	map.ui.add_child(panel)
	panel.show_data({"ours": [], "debts": [], "held": [{
		"character": "chr_jean_de_normandie", "name": "Jean de Normandie", "faction": "fac_france",
		"captor": "fac_england", "rank": "heir", "rank_label": "héritier", "prestige": 20, "ransom": 5000,
		"terms": {"kind": "money", "province": ""}, "plans": [], "cedable_provinces": [],
		"execution": {"prestige": 10, "victim_opinion": -60, "house_opinion": -25, "others_opinion": -8,
			"ransom_lost": 5000, "ruler_trait": "trait_cruel"}}]})
	panel.visible = true
	await _frames(5)
	(panel.rows["chr_jean_de_normandie"]["execute"] as Button).pressed.emit()
	await _frames(30)
	await _shot(out.path_join("wr_captives.png"))
	panel.queue_free()
	map.queue_free()
	await _frames(5)
	# 4. Sortie : dialogue d'avant-bataille.
	var fresh: Object = ClassDB.instantiate("CampaignSim")
	fresh.call("new_campaign", MAP_PATHS.default_data_dir(), "fac_france", 1337)
	var armies := BattleScene.main_armies(fresh, "fac_france", "fac_england")
	var index := int(fresh.call("debug_stage_sortie", armies[1], "prov_boulonnais"))
	var dialog: PreBattleDialog = (load("res://scenes/battle/pre_battle_dialog.tscn") as PackedScene).instantiate()
	root.add_child(dialog)
	await process_frame
	dialog.show_battle(fresh, fresh.call("get_pending_battles")[index])
	await _frames(30)
	await _shot(out.path_join("wr_sortie.png"))
	quit(0)
