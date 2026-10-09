extends TestCase

## Test headless du lot UB1 (interface de bataille) :
##  1. sons d'interface : les 8 événements `ui_*` de la banque se chargent, `UiSounds` les joue
##     (noms courts compris) et respecte la recharge ;
##  2. prévision d'avant-bataille (`get_battle_forecast`) et retraite (`withdraw_pending_battle`)
##     par le pont, sur une vraie campagne ;
##  3. écran d'avant-bataille rempli (verdict, cartes, boutons) ;
##  4. HUD : noms distincts des homonymes, journal regroupé et repliable, confirmation de la
##     retraite générale ;
##  5. écran de fin : grand titre (Victoire / Pyrrhus / Défaite), suites (`BattleAftermath`).
## Usage : godot --headless --path game --script res://tests/ub1_ui_test.gd

const UI_EVENTS := ["ui_order", "ui_order_refused", "ui_alert", "ui_letter", "ui_recruit", "ui_build", "ui_army_select", "ui_card"]


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	_check_sounds()
	await _check_forecast()
	_check_hud()
	_check_result()


func _check_sounds() -> void:
	var bank := SoundBank.load_default()
	for event_name in UI_EVENTS:
		check(bank.has_event(event_name), "sound bank: %s missing" % event_name)
		check(str(bank.event(event_name).get("bus", "")) == "Interface", "%s should play on the Interface bus" % event_name)
		check(bank.pick_stream(event_name) != null, "%s: no stream" % event_name)
	var sounds := UiSounds.instance()
	check(sounds != null and sounds.get_parent() != null, "UiSounds node not created")
	check(UiSounds.play("order"), "short name « order » should play ui_order")
	check(not UiSounds.play("order"), "ui_order should respect its cooldown")
	check(UiSounds.play("ui_card") and sounds.played.back() == "ui_card", "ui_card not recorded")
	UiSounds.play_order_result({"ok": false})
	check(sounds.played.back() == "ui_order_refused", "a refused order should sound « ui_order_refused »")
	check(not UiSounds.play("no_such_event"), "unknown event should be refused")


func _check_forecast() -> void:
	if not ClassDB.class_exists("CampaignSim"):
		check(false, "CampaignSim missing (run core/build.sh)")
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not check(sim.has_method("get_battle_forecast") and sim.has_method("withdraw_pending_battle"), "bridge: forecast / withdraw missing (run core/build.sh)"):
		return
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	if not check(sim.call("new_campaign", data_dir, "fac_france", 1337), "new_campaign failed"):
		return
	var armies := BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var forecast: Dictionary = sim.call("get_battle_forecast", index)
	check(forecast.has("attacker_win_chance") and float(forecast["attacker_share"]) > 0.0 and float(forecast["attacker_share"]) < 1.0, "forecast: %s" % [forecast])
	check(bool(forecast.get("can_withdraw", false)), "the attacking player may withdraw")
	# Écran d'avant-bataille.
	var dialog: PreBattleDialog = (load("res://scenes/battle/pre_battle_dialog.tscn") as PackedScene).instantiate()
	root.add_child(dialog)
	await process_frame
	dialog.show_battle(sim, (sim.call("get_pending_battles") as Array)[0])
	await process_frame
	check(dialog.visible and dialog.verdict_label.text != "", "pre-battle: verdict missing")
	check(dialog.body_label.text.contains("Météo prévue") and dialog.body_label.text.contains("Terrain"), "pre-battle: conditions missing")
	check(not dialog.withdraw_button.disabled and dialog.fight_button.text == "Combattre", "pre-battle: buttons")
	var cards := 0
	for column in dialog._columns:
		cards += _count(column, "RosterCard")
	check(cards >= 2, "pre-battle: regiment cards missing (%d)" % cards)
	var withdrawn := [-1]
	dialog.withdraw_requested.connect(func(i: int) -> void: withdrawn[0] = i)
	dialog.withdraw_button.emit_signal("pressed")
	check(withdrawn[0] == index and not dialog.visible, "pre-battle: « Retraite » should emit withdraw_requested")
	var result: Dictionary = sim.call("withdraw_pending_battle", index)
	check(bool(result.get("ok", false)) and (sim.call("get_pending_battles") as Array).is_empty(), "withdraw: %s" % [result])
	dialog.queue_free()


func _count(node: Node, class_label: String) -> int:
	var total := 1 if node is RosterCard and class_label == "RosterCard" else 0
	for child in node.get_children():
		total += _count(child, class_label)
	return total


func _check_hud() -> void:
	var units := [
		{"id": 1, "side": "attacker", "name": "Chevaliers"},
		{"id": 2, "side": "attacker", "name": "Archers"},
		{"id": 3, "side": "attacker", "name": "Chevaliers"},
		{"id": 4, "side": "defender", "name": "Chevaliers"},
	]
	var names := BattleHud.distinct_names(units, "attacker")
	check(names[1] == "Chevaliers I" and names[3] == "Chevaliers II" and names[2] == "Archers", "distinct names: %s" % [names])
	var hud := BattleHud.new()
	root.add_child(hud)
	hud.toggle_log()  # VN4 : journal replié par défaut, on le déplie pour lire les regroupements
	hud.add_events([{"time": 10.0, "text_fr": "Les Archers plantent leurs pieux."}, {"time": 11.0, "text_fr": "Les Archers plantent leurs pieux."}, {"time": 12.0, "text_fr": "Charge !"}])
	check(hud.log_line_count() == 2, "log: repeated lines should be grouped (%d)" % hud.log_line_count())
	var grouped := false
	for child in hud.log_box.get_children():
		if child is Label and (child as Label).text.contains("(×2)"):
			grouped = true
	check(grouped, "log: « (×2) » missing")
	hud.toggle_log()
	check(hud.log_box.get_child_count() == 2, "log: folded log should keep the header and one line")
	var fired := [""]
	hud.command_pressed.connect(func(command: String) -> void: fired[0] = command)
	hud.withdraw_all_button.emit_signal("pressed")
	check(hud.confirm_panel.visible and fired[0] == "", "general retreat should ask for confirmation first")
	(hud.confirm_panel.find_child("Confirm", true, false) as Button).emit_signal("pressed")
	check(fired[0] == "withdraw_all" and not hud.confirm_panel.visible, "confirmation should order the general retreat")
	check(hud.BAND_HEIGHT <= 140.0, "bottom band should stay compact")
	hud.queue_free()


func _check_result() -> void:
	check(BattleResultScreen.banner_title(true, 0.1, 0.6) == "Victoire", "banner: victory")
	check(BattleResultScreen.banner_title(true, 0.45, 0.3) == "Victoire à la Pyrrhus", "banner: pyrrhic")
	check(BattleResultScreen.banner_title(false, 0.5, 0.1) == "Défaite", "banner: defeat")
	var before := [{"unit_type": "a", "experience": 1}, {"unit_type": "b", "experience": 0}, {"unit_type": "a", "experience": 2}]
	var after := [{"unit_type": "a", "experience": 2}, {"unit_type": "a", "experience": 2}]  # le « b » est tombé
	check(BattleAftermath.veterans(before, after) == 1, "aftermath: veterans %d" % BattleAftermath.veterans(before, after))
	var diff := BattleAftermath.diff(
		{"general": {"name": "Philippe", "experience": 10, "skill_points": 0}, "held": {}, "ours": {}, "units": []},
		{"general": {"name": "Philippe", "experience": 30, "skill_points": 1, "alive": true}, "held": {"chr_x": {"name": "Édouard", "ransom": 1000}}, "ours": {}, "units": []})
	check(int(diff["general_xp"]) == 20 and int(diff["skill_points"]) == 1 and int(diff["ransom_total"]) == 1000, "aftermath diff: %s" % [diff])
