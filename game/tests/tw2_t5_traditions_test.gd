extends TestCase

## Test headless du lot TW2-T5 (traditions d'armée) sur la vraie simulation et les vraies données,
## vérifié par l'état des nœuds (pas de capture) :
##  1. pont : `get_army_traditions` (rang, xp, choix), refus sans rang, `debug_grant_army_xp` ;
##  2. le bandeau d'une armée du joueur porte le bouton « Traditions (1) » après un rang, et le toast
##     l'annonce ; le panneau liste une tradition par branche, choisir en adopte une ;
##  3. le nom gardé reste au bandeau quand le chef quitte l'armée (pas de chef : titre inchangé).
## Usage : godot --headless --path game --script res://tests/tw2_t5_traditions_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_army_traditions"),
			"CampaignSim.get_army_traditions missing (run core/build.sh)"):
		return
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var sim: Object = map.sim
	var army_id := ""
	for id in map.player_army_ids():
		var army: Dictionary = sim.call("get_army", id)
		if str(army.get("general", "")) != "":
			army_id = id
			break
	if check(army_id != "", "France has no army led by a general"):
		_test_bridge(sim, army_id)
		await _test_panel(map, sim, army_id)
	map.queue_free()
	await process_frame


# --- 1. Pont --------------------------------------------------------------------------------


func _test_bridge(sim: Object, army_id: String) -> void:
	var info: Dictionary = sim.call("get_army_traditions", army_id)
	for key in ["xp", "rank", "max_rank", "next_threshold", "pending", "name", "named", "banner_house", "effects_text", "chosen", "options"]:
		check(info.has(key), "get_army_traditions lacks %s: %s" % [key, info.keys()])
	check(int(info.get("rank", -1)) == 0 and int(info.get("pending", -1)) == 0, "a fresh army has no rank: %s" % info)
	check(int(info.get("max_rank", 0)) in [3, 4], "3 or 4 ranks expected: %s" % info.get("max_rank"))
	var options: Array = info.get("options", [])
	check(options.size() >= 15, "five branches of three tiers: %d options" % options.size())
	var refused: Dictionary = sim.call("choose_army_tradition", army_id, "trad_march_1")
	check(not bool(refused.get("ok", true)) and str(refused.get("error", "")).contains("rang"),
		"a choice without a rank is refused in French: %s" % refused)
	check((sim.call("get_army_traditions", "army_999999") as Dictionary).is_empty(), "an unknown army gives an empty dictionary")
	check(TraditionsController.next_by_branch(options).size() == 5, "one next tradition per branch")
	check(TraditionsController.rank_line(info).begins_with("Rang 0 sur"), "rank line: %s" % TraditionsController.rank_line(info))


# --- 2. Bandeau, toast et panneau -----------------------------------------------------------


func _test_panel(map: Node, sim: Object, army_id: String) -> void:
	var controller: TraditionsController = map.get("traditions")
	if not check(controller != null, "the campaign map has no traditions controller"):
		return
	map.select_army(army_id)
	await process_frame
	check(controller.button.visible and controller.button.text == "Traditions", "the army strip shows « Traditions »: %s" % controller.button.text)
	var before_title := str(sim.call("get_army_traditions", army_id).get("name", ""))
	var first := int(sim.call("get_army_traditions", army_id).get("next_threshold", 0))
	check(bool(sim.call("debug_grant_army_xp", army_id, first)), "debug_grant_army_xp refused")
	map.refresh_all()
	await process_frame
	var info: Dictionary = sim.call("get_army_traditions", army_id)
	check(int(info.get("rank", 0)) == 1 and int(info.get("pending", 0)) == 1, "rank 1 with one choice: %s" % info)
	check(bool(info.get("named", false)), "the army keeps its name from its first experience")
	check(controller._notified.get(army_id, 0) == 1, "the new rank is announced once")
	check(controller.notify_new_ranks().is_empty(), "no second toast for the same rank")
	map.select_army(army_id)
	await process_frame
	check(controller.button.text == "Traditions (1)", "the button counts the pending choice: %s" % controller.button.text)

	controller.button.pressed.emit()
	await process_frame
	check(controller.panel.visible and controller.army_id == army_id, "the button opens the traditions panel")
	var rows := controller.panel.find_child("Options", true, false)
	if check(rows != null and rows.get_child_count() == 5, "one row per branch"):
		var enabled := 0
		for child in rows.get_children():
			if not (child as Button).disabled:
				enabled += 1
		check(enabled == 5, "every first tier is open at rank 1 (%d)" % enabled)
	var rank_label: Label = controller.panel.find_child("RankLine", true, false)
	check(rank_label != null and rank_label.text.begins_with("Rang 1 sur"), "rank line in the panel")
	var march: Button = controller.panel.find_child("trad_march_1", true, false)
	if check(march != null, "the march tradition has a row"):
		march.pressed.emit()
		await process_frame
	info = sim.call("get_army_traditions", army_id)
	check(int(info.get("pending", 1)) == 0 and (info.get("chosen", PackedStringArray()) as PackedStringArray).size() == 1,
		"the click adopted the tradition: %s" % info)
	check(str(info.get("effects_text", "")).contains("mouvement"), "effects: %s" % info.get("effects_text"))
	check(controller.button.text == "Traditions", "no choice left on the button")
	rows = controller.panel.find_child("Options", true, false)
	var next_march: Button = controller.panel.find_child("trad_march_2", true, false)
	check(next_march != null and next_march.disabled, "the second march tier waits for the next rank")
	print("tw2_t5: %s — %s ; %s" % [info.get("name"), TraditionsController.rank_line(info), info.get("effects_text")])

	# 3. Nom gardé : le titre du bandeau ne suit plus le chef.
	var kept := controller.kept_title(army_id)
	check(kept.begins_with("Ost"), "kept title: %s (was %s)" % [kept, before_title])
	var strip: Node = map.ui.army_strip
	var title_label: Label = strip.get("_title_label")
	if title_label != null:
		check(title_label.text.to_lower().contains(kept.substr(4).to_lower()), "the strip title uses the kept name: %s / %s" % [title_label.text, kept])
