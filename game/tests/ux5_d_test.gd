extends TestCase

## Lot UX5-D : diplomatie (tri/recherche/tendance, bloc Opinion, seuil d'acceptation, bandeau de la Cour).
## Usage : godot --headless --path game --script res://tests/ux5_d_test.gd

var map: Node3D = null


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _wait(frames: int) -> void:
	for _i in frames:
		await process_frame


func _run() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	settings.call("use_test_file")
	settings.call("set_value", "tutorial/enabled", false, false)
	settings.call("set_value", "game/autosave_interval", 0, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	map = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _wait(10)
	var controller: Node = map.diplomacy
	controller.open_panel("fac_england")
	await _wait(10)
	var panel: Control = controller.panel
	var list: DiplomacyFactionList = panel.get("_faction_list")
	# D1 : recherche + tri.
	var all_rows := panel.find_children("Faction_*", "Button", true, false)
	check(all_rows.size() > 3, "faction rows missing")
	check(list.search_field != null and list.search_field.placeholder_text == "Chercher une maison…", "search field missing")
	list.search_field.text_changed.emit("angl")
	await _wait(2)
	var found := list.visible_entries()
	check(found.size() >= 1 and found.size() < all_rows.size(), "search should narrow the list (%d of %d)" % [found.size(), all_rows.size()])
	for e in found:
		check(str(e["name"]).to_lower().contains("angl"), "search kept %s" % e["name"])
	list.search_field.text_changed.emit("")
	list.sort_button.item_selected.emit(1)
	var by_attitude := list.visible_entries()
	var ordered := true
	for i in range(1, by_attitude.size()):
		ordered = ordered and int(by_attitude[i - 1]["attitude"]) <= int(by_attitude[i]["attitude"])
	check(ordered, "attitude sort order")
	list.sort_button.item_selected.emit(2)
	var by_power := list.visible_entries()
	ordered = true
	for i in range(1, by_power.size()):
		ordered = ordered and int(by_power[i - 1].get("power", 0)) >= int(by_power[i].get("power", 0))
	check(ordered, "power sort order")
	list.sort_button.item_selected.emit(0)
	# D5 : pas de flèche après chargement ; une fois un tour écoulé, la référence existe.
	check(list.trend_of("fac_england", 10) == "", "no trend without a previous turn")
	list.show_entries(list._entries, "fac_england", 5)
	list._previous = {"fac_england": 5}
	check(list.trend_of("fac_england", 8) == "↑" and list.trend_of("fac_england", 2) == "↓" and list.trend_of("fac_england", 5) == "=", "trend glyphs")
	list._previous = {}
	list.show_entries(list._entries, "fac_england", 6)
	check(list._previous.has("fac_england"), "new turn should snapshot attitudes")
	# D2 : bloc Opinion.
	var head := panel.find_child("Head", true, false) as Control
	var opinion := head.find_child("Opinion", true, false)
	var entry: Dictionary = panel.call("_entry", "fac_england")
	if not (entry.get("attitude_reasons", []) as Array).is_empty():
		if check(opinion != null, "Opinion block missing"):
			var weights: Array = []
			for line in opinion.find_children("OpinionReason", "Label", true, false):
				weights.append(absi(int((line as Label).text.split(" ", false)[0])))
			var sorted_ok := true
			for i in range(1, weights.size()):
				sorted_ok = sorted_ok and weights[i - 1] >= weights[i]
			check(sorted_ok, "opinion reasons must be sorted by absolute weight")
			check(opinion.find_child("OpinionTotal", true, false) != null, "opinion total missing")
	# D3 : jauge de seuil, aucun pourcentage.
	var negotiation: DiplomacyNegotiationTab = panel.get("negotiation")
	negotiation.load_example_draft()
	await _wait(3)
	var words := negotiation.margin_label.text
	check(words.contains("manque") or words.contains("Marge"), "threshold distance in words: " + words)
	check(not words.contains("%") and not negotiation.chance_label.text.contains("%"), "no percentage")
	check(DiplomacyNegotiationTab.threshold_words(-12) == "Il manque 12 au seuil", "manque wording")
	check(DiplomacyNegotiationTab.threshold_words(8).begins_with("Marge de 8"), "marge wording")
	# D4 : bandeau, visible ssi des lignes existent.
	var court: DiplomacyCourtBanner = panel.get("_court")
	check(court.visible == (not court.lines().is_empty()), "court banner visibility matches content")
