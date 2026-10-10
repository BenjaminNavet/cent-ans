extends TestCase

## Test headless du lot WR « sortie » (ADR 0306) : la sortie de la garnison assiégée est une
## bataille de campagne en attente, avec le dialogue d'avant-bataille habituel.
##  1. l'anglais assiège Boulogne, le joueur (France) fait sortir sa garnison : une bataille en
##     attente `sortie` (garnison attaquante), rien n'est encore résolu ;
##  2. le dialogue d'avant-bataille propose « Livrer bataille » / « Résolution automatique » /
##     « Rester dans la place » ;
##  3. « Résolution automatique » : la sortie est résolue par le cœur (journal « Sortie ») ;
##  4. « Livrer bataille » : la bataille 3D (BattleSim) se joue, son résultat est accepté par
##     `resolve_battle` et la sortie disparaît des batailles en attente.
## Usage : godot --headless --path game --script res://tests/wr_sortie_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _new_sim() -> Object:
	var sim: Object = ClassDB.instantiate("CampaignSim")
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	if not check(sim.call("new_campaign", data_dir, "fac_france", 1337), "new_campaign failed"):
		return null
	return sim


func _stage(sim: Object) -> int:
	var armies := BattleScene.main_armies(sim, "fac_france", "fac_england")
	if armies.size() != 2:
		return -1
	return int(sim.call("debug_stage_sortie", armies[1], "prov_boulonnais"))


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.class_exists("BattleSim") and ClassDB.instantiate("CampaignSim").has_method("debug_stage_sortie"),
			"CampaignSim.debug_stage_sortie missing (run core/build.sh)"):
		return
	# 1-2. Bataille en attente et dialogue.
	var sim := _new_sim()
	if sim == null:
		return
	var index := _stage(sim)
	if not check(index >= 0, "debug_stage_sortie failed"):
		return
	var pending: Array = sim.call("get_pending_battles")
	if not check(pending.size() == 1, "one pending battle expected, got %d" % pending.size()):
		return
	var entry: Dictionary = pending[0]
	check(bool(entry.get("sortie", false)) and not bool(entry.get("siege", true)), "the pending battle should be a sortie: %s" % [entry])
	check(str(entry.get("player_side", "")) == "attacker", "the player's garrison attacks")
	check(int(entry.get("attacker_strength", 0)) > 0, "attacker strength = the garrison")
	var setup: Dictionary = sim.call("get_battle_setup", index)
	check(not setup.is_empty() and str(setup["attacker"]["faction"]) == "fac_france" and str(setup["defender"]["faction"]) == "fac_england",
			"setup: the garrison (France) attacks the besiegers (England)")
	check(not setup.has("siege") or setup["siege"] == null or (setup["siege"] is Dictionary and (setup["siege"] as Dictionary).is_empty()), "no walls to take in a sortie")
	var dialog: PreBattleDialog = (load("res://scenes/battle/pre_battle_dialog.tscn") as PackedScene).instantiate()
	root.add_child(dialog)
	await process_frame
	dialog.show_battle(sim, entry)
	await process_frame
	check(dialog.visible and dialog.title_label.text.begins_with("Sortie de"), "dialog title: %s" % dialog.title_label.text)
	check(dialog.fight_button.text == "Livrer bataille" and not dialog.fight_button.disabled, "dialog: « Livrer bataille » expected, got %s" % dialog.fight_button.text)
	check(dialog.auto_button.text == "Résolution automatique", "dialog: auto-resolution button")
	check(dialog.withdraw_button.text == "Rester dans la place" and not dialog.withdraw_button.disabled, "dialog: the player may stay inside")

	# 3. Résolution automatique.
	var auto_events: Array = sim.call("auto_resolve_battle", index)
	check((sim.call("get_pending_battles") as Array).is_empty(), "auto: no pending battle left")
	var told := false
	for event in auto_events:
		told = told or str((event as Dictionary).get("text_fr", "")).contains("Sortie")
	check(told, "auto: the journal should tell the sortie")
	dialog.queue_free()

	# 4. Bataille 3D headless puis résultat appliqué.
	sim = _new_sim()
	if sim == null:
		return
	index = _stage(sim)
	pending = sim.call("get_pending_battles")
	setup = sim.call("get_battle_setup", index)
	var battle: Object = ClassDB.instantiate("BattleSim")
	if not check(index >= 0 and battle.call("setup", setup, int(pending[0]["seed"])), "BattleSim.setup refused the sortie setup"):
		return
	battle.call("set_ai", "attacker", true)
	for _i in 12000:
		battle.call("tick", 0.1)
		if battle.call("is_finished"):
			break
	if not check(battle.call("is_finished"), "the sortie battle should end"):
		return
	var result: Dictionary = sim.call("resolve_battle", index, battle.call("get_outcome"))
	check(bool(result.get("ok", false)), "resolve_battle refused: %s" % result.get("error", "?"))
	check((sim.call("get_pending_battles") as Array).is_empty(), "the fought sortie is gone from the pending battles")
	var line := false
	for event in result.get("events", []):
		line = line or str((event as Dictionary).get("text_fr", "")).contains("Sortie")
	check(line, "the result should tell the sortie")
