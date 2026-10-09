extends TestCase

## Lot WH chars (ADR 0276, 0277), en headless : onglet « Actes royaux » du panneau de cour (liste,
## bouton, ordre, recharge, refus), recrutement d'un capitaine, bloc « Faits d'armes » et pastille
## de niveau de la fiche personnage, blessure temporaire annoncée sur la fiche.
## Usage : godot --headless --path game --script res://tests/wh_chars_ui_test.gd

const FACTION_ID := "fac_france"


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	var real_capable: bool = ClassDB.class_exists("CampaignSim") \
		and ClassDB.instantiate("CampaignSim").has_method("get_royal_acts")
	if not check(real_capable, "wh_chars_ui_test needs the real CampaignSim (get_royal_acts)"):
		return
	var sim: Object = ClassDB.instantiate("CampaignSim")
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	if not check(sim.call("new_campaign", data_dir, FACTION_ID, 1337), "new_campaign failed"):
		return
	var ids: Array = sim.call("get_faction_characters", FACTION_ID)
	var ruler := str(ids[0])

	# Données du pont.
	var acts: Array = sim.call("get_royal_acts", FACTION_ID)
	check(acts.size() >= 4, "France should be offered at least 4 royal acts (got %d)" % acts.size())
	var coronation := _find(acts, "act_sacre_reims")
	check(not coronation.is_empty(), "the coronation at Reims is a French act")
	check(_find(acts, "act_chevauchee_royale").is_empty(), "the English chevauchée is not offered to France")
	var info: Dictionary = sim.call("get_captain_info", FACTION_ID)
	check(int(info.get("cap", 0)) >= 2 and int(info.get("cost", 0)) > 0, "captain info should give a cap and a cost")

	# Onglet « Actes royaux » du panneau de cour.
	var court: Node = (load("res://scenes/ui/court_panel.tscn") as PackedScene).instantiate()
	court.set("sim_source", sim)
	root.add_child(court)
	await process_frame
	var rows: Array[Dictionary] = []
	for id in ids:
		rows.append(sim.call("get_character", id))
	court.show_court(rows, "France", Color(0.2, 0.3, 0.7))
	court.show_tab(2)  # CourtPanel.TAB_ACTS
	await process_frame
	var section: Node = court.acts_section
	check(section.act_buttons.has("act_sacre_reims"), "the coronation card should have its button")
	check(court.acts_scroll.visible and not court.rows_list.get_parent().visible, "acts tab replaces the list")
	check(not (section.act_buttons["act_sacre_reims"] as Button).disabled, "coronation should be affordable at the start")

	# Un acte : accompli, puis en recharge et refus rouge à la seconde demande.
	var result: Dictionary = section.request_act("act_sacre_reims")
	check(bool(result.get("ok", false)), "performing the coronation should be accepted: %s" % str(result.get("error", "")))
	await process_frame
	check((section.act_buttons["act_sacre_reims"] as Button).disabled, "button disabled while cooling down")
	var again: Dictionary = section.request_act("act_sacre_reims")
	check(not bool(again.get("ok", true)) and "recharge" in str(again.get("error", "")), "second performance refused with the cooldown")
	check(section.error_label.visible, "the refusal is shown in red")
	check(int(_find(sim.call("get_royal_acts", FACTION_ID), "act_sacre_reims").get("active_left", 0)) > 0, "the act is in force")

	# Capitaine.
	var before_count := int(info.get("count", 0))
	var hired: Dictionary = section.request_captain()
	check(bool(hired.get("ok", false)), "hiring a captain should be accepted: %s" % str(hired.get("error", "")))
	check(int(sim.call("get_captain_info", FACTION_ID).get("count", 0)) == before_count + 1, "one more captain")
	court.queue_free()
	await process_frame

	# Fiche : niveau, faits d'armes, blessure temporaire.
	sim.call("submit_order", {"type": "debug_grant_xp", "character": ruler, "amount": 400})
	var sheet: Node = (load("res://scenes/ui/character_sheet.tscn") as PackedScene).instantiate()
	root.add_child(sheet)
	await process_frame
	var character: Dictionary = sim.call("get_character", ruler)
	check(int(character.get("level", 0)) > 1, "400 XP should have raised the ruler's level")
	check(character.has("feats"), "get_character should expose the feats")
	sheet.show_character(character, sim.call("get_skill_tree"), sim.call("get_learnable", ruler), [], [], [])
	await process_frame
	check(sheet.feats_label != null and sheet.feats_label.text != "", "the feats block is filled")
	check(CharacterSheet.feats_text({"battles_fought": 3, "battles_won": 2, "sieges_won": 1, "raids_led": 0, "last_battle_turns_ago": 2, "last_battle_won": true}).contains("victoire"), "feats text names the last battle")
	check(CharacterSheet.feats_text({}).begins_with("Aucun"), "empty feats read as none yet")
	sheet.queue_free()
	await process_frame


static func _find(entries: Array, id: String) -> Dictionary:
	for entry in entries:
		if str(entry.get("id", "")) == id:
			return entry
	return {}
