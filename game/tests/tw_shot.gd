extends SceneTree

## Captures de contrôle du chantier TW (fenêtré, via tools/godot_bg.sh) : briefing de campagne,
## bouton « Désigner héritier », Réglages (touches de la bataille), avant-bataille (carte du site,
## risque en résolution automatique), résumé de résolution automatique, HUD de bataille (Poursuivre,
## « Renforts dans », « Nuit dans »).
## Usage : tools/godot_bg.sh --path game --script res://tests/tw_shot.gd -- --out=/chemin/absolu


const MAP_PATHS := preload("res://scripts/map/map_paths.gd")


func _frames(count: int) -> void:
	for _i in count:
		await process_frame


func _shot(path: String) -> void:
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(path)
	print("tw_shot: -> %s" % path)


func _init() -> void:
	var out := CmdArgs.value("--out", "user://tw_shot")
	DirAccess.make_dir_recursive_absolute(out)
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "feudal_tutorial/done", true, false)
		settings.call("set_value", "interface/campaign_briefing", true, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	root.size = Vector2i(1280, 720)
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _frames(60)
	# 1. Briefing de début de campagne.
	await _shot(out.path_join("tw_briefing.png"))
	if map.briefing != null:
		map.briefing.start_button.emit_signal("pressed")
	await _frames(10)
	# 2. Cour, onglet arbre : bouton « Désigner héritier » armé sur un parent du souverain.
	map.call("_on_court_panel_requested")
	await _frames(10)
	var court: Node = null
	for node in root.find_children("*", "PanelContainer", true, false):
		var script: Script = node.get_script()
		if script != null and script.resource_path.ends_with("court_panel.gd"):
			court = node
	if court != null:
		court.call("show_tab", 1)  # onglet « Arbre familial »
		await _frames(20)
		var tree: Node = court.get("family_tree")
		for id in tree.get("_nodes"):
			if bool(tree.call("can_designate", str(id))):
				tree.set("selected_id", str(id))
				break
		tree.call("_refresh_designate_button")
		await _frames(10)
	await _shot(out.path_join("tw_heir.png"))
	map.queue_free()
	await _frames(5)
	# 3. Réglages, onglet Commandes (« Touches de la bataille »).
	var menu := SettingsMenu.new()
	menu.initial_tab = "Commandes"
	menu.position = Vector2(200, 60)
	root.add_child(menu)
	await _frames(20)
	await _shot(out.path_join("tw_settings_keys.png"))
	menu.queue_free()
	# 4. Avant-bataille : carte du site et risque pour le général en résolution automatique.
	var fresh: Object = ClassDB.instantiate("CampaignSim")
	fresh.call("new_campaign", MAP_PATHS.default_data_dir(), "fac_france", 1337)
	var armies := BattleScene.main_armies(fresh, "fac_france", "fac_england")
	var index := int(fresh.call("debug_stage_battle", armies[0], armies[1]))
	var dialog: PreBattleDialog = (load("res://scenes/battle/pre_battle_dialog.tscn") as PackedScene).instantiate()
	root.add_child(dialog)
	await process_frame
	dialog.show_battle(fresh, fresh.call("get_pending_battles")[index])
	await _frames(30)
	await _shot(out.path_join("tw_prebattle.png"))
	dialog.queue_free()
	# 5. Résumé de résolution automatique (défaite, général pris).
	var screen := BattleResultScreen.new()
	root.add_child(screen)
	await process_frame
	screen.show_auto_summary({
		"attacker_won": true, "province": "Guyenne",
		"attacker": {"faction": "fac_england", "faction_name": "Angleterre", "player": false, "soldiers_before": 2000, "losses": 300},
		"defender": {"faction": "fac_france", "faction_name": "France", "player": true, "soldiers_before": 1500, "losses": 900, "general_captured": true, "routed": true},
	}, {"general_name": "Philippe"})
	await _frames(30)
	await _shot(out.path_join("tw_auto_summary.png"))
	screen.queue_free()
	# 6. HUD de bataille : Poursuivre, renforts, nuit.
	var hud := BattleHud.new()
	root.add_child(hud)
	await _frames(5)
	hud.set_clock(754.0, 1.0, false)
	hud.set_time_left(245.0, 260.0, [], false)
	hud.set_reinforcements(95.0)
	await _frames(20)
	await _shot(out.path_join("tw_hud.png"))
	quit(0)
