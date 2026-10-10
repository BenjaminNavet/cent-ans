extends TestCase

## TW polish : bouton « Poursuivre », infobulle « Attaque au pas », bark `flanked`, recentrage du
## général, chargement passable au clavier, briefing absent d'une sauvegarde.
##
## Usage : godot --headless --path game --script res://tests/tw_polish_test.gd


func _key(code: Key, pressed: bool = true) -> InputEventKey:
	var event := InputEventKey.new()
	event.keycode = code
	event.pressed = pressed
	return event


func _init() -> void:
	await process_frame
	# 1. Bouton Poursuivre : déclaré parmi les ordres, touche P, infobulle en données.
	var names: Array = []
	for entry in BattleHud.COMMANDS:
		names.append(str(entry[0]))
	check(names.has("pursue"), "pursue is a HUD command")
	check(BattleHotkeys.key_label("pursue") == "P", "pursue shows key P (%s)" % BattleHotkeys.key_label("pursue"))
	var tips: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://../data/ui/tooltips.json"))
	var plain: Dictionary = tips.get("plain", {})
	check(plain.has("battle_pursue") and str(plain["battle_pursue"].get("body", "")) != "", "pursue tooltip in tooltips.json")
	var hud := BattleHud.new()
	root.add_child(hud)
	await process_frame
	var button: Button = hud._command_buttons.get("pursue")
	check(button != null, "pursue button built")
	if button != null:
		var emitted: Array = []
		hud.command_pressed.connect(func(command: String) -> void: emitted.append(command))
		button.pressed.emit()
		check(emitted == ["pursue"], "pursue button emits the order (%s)" % [emitted])
	hud.queue_free()

	# 2. Infobulle « Attaque au pas ».
	check(BattleInput.walk_tip_wanted("melee", true, true), "walk tip on melee target with Alt")
	check(not BattleInput.walk_tip_wanted("melee", false, true), "no walk tip without Alt")
	check(not BattleInput.walk_tip_wanted("move", true, true), "no walk tip on plain ground")
	check(not BattleInput.walk_tip_wanted("melee", true, false), "no walk tip without selection")
	check(BattleInput.walk_tip_text().begins_with("Attaque au pas"), "walk tip text")

	# 3. Bark flanked : situation et lignes (repli de langue) déclarées.
	check(not VoiceLines.situation("flanked").is_empty(), "flanked situation declared")
	for language in ["fr", "en", "oc", "cy"]:
		check(not VoiceLines.bark_candidates(language, "flanked", "infantry").is_empty(), "flanked has lines in %s" % language)

	# 4. Recentrage sur le général : `recenter` en données (désactivable).
	var staging: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://../data/fx/battle_staging.json"))
	check(bool(staging["general_fall_slowmo"].get("recenter", false)), "recenter enabled in data")

	# 5. Chargement : Espace, Échap, Entrée passent ; une autre touche non.
	check(BattleLoadingCard.is_skip_key(_key(KEY_SPACE)) and BattleLoadingCard.is_skip_key(_key(KEY_ESCAPE)) and BattleLoadingCard.is_skip_key(_key(KEY_ENTER)), "Space/Escape/Enter skip loading")
	check(not BattleLoadingCard.is_skip_key(_key(KEY_A)) and not BattleLoadingCard.is_skip_key(_key(KEY_SPACE, false)), "other keys and releases do not")

	# 6. Briefing : pas pour une sauvegarde du tour 0.
	check(CampaignBriefing.should_show(0, true, false, false), "briefing on new game")
	check(not CampaignBriefing.should_show(0, true, false, true), "no briefing on a loaded turn-0 save")
	finish()
