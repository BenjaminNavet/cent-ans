extends SceneTree

## TW trans : résumé de résolution automatique (top1), risque du général sur le bouton Auto
## (top3), chargement passable (top4), carte « Dans l'Histoire » (top5), briefing (top9).

var _failures := 0


func _check(ok: bool, what: String) -> void:
	if not ok:
		_failures += 1
		printerr("FAIL: ", what)


func _initialize() -> void:
	# top1 : modèle du résumé, côté défenseur, général pris.
	var result := {
		"attacker_won": true, "province": "Guyenne",
		"attacker": {"faction": "fac_england", "faction_name": "Angleterre", "player": false, "soldiers_before": 2000, "losses": 300, "general_captured": false, "general_killed": false, "routed": false},
		"defender": {"faction": "fac_france", "faction_name": "France", "player": true, "soldiers_before": 1500, "losses": 900, "general_captured": true, "general_killed": false, "routed": true},
	}
	var model := BattleResultScreen.auto_summary_model(result, {"general_name": "Philippe"})
	_check(model["player_side"] == "defender", "player side is defender")
	_check((model["outcome"] as Dictionary)["winner"] == "attacker", "winner attacker")
	_check(float(model["totals"]["defender"]["ratio"]) == 0.6, "loss ratio 60 %")
	var captives := ", ".join(PackedStringArray(model["captive_lines"]))
	_check(captives.contains("Philippe est prisonnier"), "own general captured line: " + captives)
	_check(captives.contains("mise en déroute"), "own rout line: " + captives)
	var screen := BattleResultScreen.new()
	root.add_child(screen)
	await process_frame  # _ready de l'écran
	screen.show_auto_summary(result, {"general_name": "Philippe"})
	_check(screen.title_label != null and screen.title_label.text == "Défaite", "defeat banner")
	_check(screen.subtitle_label.text.contains("Guyenne") and not screen.subtitle_label.text.contains("min"), "auto subtitle without duration")
	_check(screen.aftermath_box.get_child_count() == 3, "three aftermath cards")
	# Victoire avec captifs ramenés.
	var won_model := BattleResultScreen.auto_summary_model({"attacker_won": true, "province": "X",
		"attacker": {"faction": "a", "faction_name": "A", "player": true, "soldiers_before": 100, "losses": 10},
		"defender": {"faction": "d", "faction_name": "D", "player": false, "soldiers_before": 100, "losses": 60, "general_captured": true}},
		{"captives": [{"name": "Sire", "rank": "baron", "ransom": 1200}], "ransom_total": 1200})
	_check(won_model["player_side"] == "attacker", "player attacker")
	_check(PackedStringArray(won_model["loot_lines"])[0].contains("rançons"), "ransom loot line")
	_check(not ", ".join(PackedStringArray(won_model["captive_lines"])).contains("rançon à fixer"), "no duplicate ransom line when captive listed")

	# top3 : texte du risque.
	var forecast := {"attacker_general_loss_pct": 12.4, "defender_general_loss_pct": 30.0}
	_check(PreBattleDialog.general_risk_text(forecast, "attacker") == "Risque : général perdu 12 % · chef ennemi pris 30 %", "risk text attacker: " + PreBattleDialog.general_risk_text(forecast, "attacker"))
	_check(PreBattleDialog.general_risk_text({"attacker_general_loss_pct": 0.0, "defender_general_loss_pct": 0.0}, "attacker") == "", "no risk, no pill")
	_check(PreBattleDialog.general_risk_text({}, "attacker") == "", "old forecast, no pill")

	# top4 : chargement passable sauf le premier.
	_check(not BattleLoadingCard.skippable_by_setting(true, true), "first loading keeps its time")
	_check(BattleLoadingCard.skippable_by_setting(false, true), "later loading skippable by setting")
	_check(not BattleLoadingCard.skippable_by_setting(false, false), "setting off keeps the time")

	# top5 : Crécy, Français vainqueurs (attaquants) alors que l'Histoire donne la victoire aux Anglais.
	var crecy := {"historical_winner": "defender", "name": "Bataille de Crécy", "date_fr": "26 août 1346",
		"attacker": {"faction_name": "France"}, "defender": {"faction_name": "Angleterre"}}
	var lines := BattleResultScreen.history_card_lines(crecy, "attacker", "attacker")
	_check(lines.size() == 2 and str(lines[0]) == "Vous avez renversé l’Histoire", "Crécy reversed: " + str(lines))
	_check(str(lines[1]).contains("Angleterre") and str(lines[1]).contains("1346"), "history line: " + str(lines))
	_check(str(BattleResultScreen.history_card_lines(crecy, "defender", "defender")[0]).begins_with("L’Histoire se répète"), "repeated win")
	_check(str(BattleResultScreen.history_card_lines(crecy, "defender", "attacker")[0]).begins_with("Comme à l’Histoire"), "repeated loss")
	_check(BattleResultScreen.history_card_lines({}, "attacker", "attacker").is_empty(), "no card outside history")
	_check(BattleResultScreen.history_card_lines({"site_only": true, "historical_winner": "defender"}, "attacker", "attacker").is_empty(), "no card for site only")

	# top9 : briefing.
	_check(CampaignBriefing.should_show(0, true, false), "briefing at turn 1")
	_check(not CampaignBriefing.should_show(1, true, false) and not CampaignBriefing.should_show(0, false, false) and not CampaignBriefing.should_show(0, true, true), "briefing gates")
	var briefing_model := CampaignBriefing.build_model("France", "Printemps 1337", [{"title": "Tenir Paris"}], ["Angleterre"])
	_check(briefing_model["objectives"] == ["• Tenir Paris"] and briefing_model["neighbours"] == ["• Angleterre"], "briefing lists")
	_check((briefing_model["tips"] as Array).size() == 3 and str(briefing_model["intro"]) == "France — Printemps 1337", "briefing tips and intro")
	var peace := CampaignBriefing.build_model("France", "x", [], [])
	_check(str(peace["neighbours"][0]).contains("paix"), "peace line")
	var briefing := CampaignBriefing.new()
	root.add_child(briefing)
	await process_frame
	briefing.show_briefing(briefing_model)
	var closed := [false]
	briefing.closed.connect(func() -> void: closed[0] = true)
	briefing.start_button.emit_signal("pressed")
	_check(closed[0], "briefing closes")

	if _failures == 0:
		print("trans_test: OK")
	quit(1 if _failures > 0 else 0)
