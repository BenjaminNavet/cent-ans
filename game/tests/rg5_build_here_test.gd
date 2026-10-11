extends TestCase

## Test headless des lots RG4 et RG5 sur la vraie simulation :
##  1. la liste des colonies du panneau de province porte un marteau par colonie du joueur, allumé
##     comme le verdict `can_build` du cœur, et le compte « constructibles » ;
##  2. choisir une autre colonie (marteau ou sélecteur) recharge la vue des bâtiments du panneau de
##     province sans le quitter ;
##  3. la liste « Colonies » (touche B) porte les marteaux et ouvre la vue à côté sans se fermer ;
##  4. annuler un chantier demande confirmation chiffrée, puis rembourse (même tour : tout).
## Usage : godot --headless --path game --script res://tests/rg5_build_here_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("settlement_can_build"),
			"CampaignSim.settlement_can_build missing (run core/build.sh)"):
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
	var ctl: Node = map.settlements_ctl
	var sim: Object = map.sim
	var overview: Dictionary = sim.call("get_holdings_overview", map.player_faction)
	var province_row := {}
	for province in overview.get("provinces", []):
		var mine := (province["settlements"] as Array).filter(func(row: Dictionary) -> bool: return not bool(row["occupied"]))
		if mine.size() >= 2:
			province_row = province
			break
	if not check(not province_row.is_empty(), "a French province with two settlements is needed"):
		map.queue_free()
		return
	var province_id := str(province_row["province"])
	var places: Array = (province_row["settlements"] as Array).filter(func(row: Dictionary) -> bool: return not bool(row["occupied"]))
	print("rg5: province %s, %d settlements, %d buildable" % [province_id, places.size(), int(province_row["buildable_count"])])
	check(int(province_row["buildable_count"]) == places.filter(func(row: Dictionary) -> bool: return bool(row["can_build"])).size(), "buildable_count matches the rows")

	# 1. Marteaux de la liste des colonies du panneau de province.
	map._on_province_selected(map.map_data.index_of_id(province_id))
	var province_panel: Control = map.ui.province_panel
	check(province_panel.visible, "province panel open")
	var hammers: Array = province_panel.settlements_list.find_children("BuildHammer", "Button", true, false)
	check(hammers.size() == places.size(), "one hammer per settlement of the player: %d vs %d" % [hammers.size(), places.size()])
	for hammer in hammers:
		var verdict: Dictionary = sim.call("settlement_can_build", hammer.settlement_id)
		check(hammer.can_build == bool(verdict["ok"]), "hammer %s follows can_build" % hammer.settlement_id)
		check(hammer.can_build or hammer.tooltip_text != "", "a greyed hammer carries its reason")
	var count_label: Label = province_panel.settlements_list.get_node_or_null("BuildableCount")
	check(count_label != null and count_label.text == "constructibles : %d" % int(province_row["buildable_count"]), "constructibles count shown")

	# 2. Changer de colonie recharge la vue des bâtiments dans le panneau de province.
	var first := str(places[0]["id"])
	var second := str(places[1]["id"])
	var companion: BuildCompanion = province_panel.build_companion
	province_panel.select_build_settlement(first)
	check(province_panel.visible and companion.visible and companion.view.settlement_id == first, "build view shows %s" % first)
	var first_usage := companion.view.usage_label.text
	province_panel.select_build_settlement(second)
	check(province_panel.visible and companion.view.settlement_id == second, "build view switched to %s" % second)
	check(companion.view.card_count() > 0, "build view lists slots")
	check(companion.selector.item_count == places.size(), "selector lists the player's settlements")
	companion.selector.select(0)
	companion.selector.item_selected.emit(0)
	check(companion.view.settlement_id == str(companion._ids[0]), "selector switches the build view")
	print("rg5: usage %s / %s" % [first_usage, companion.view.usage_label.text])
	var offered := _buildable_id(sim, first)
	if check(offered != "", "something buildable in %s" % first):
		var player: String = map.player_faction
		var before := int((sim.call("get_faction_economy", player) as Dictionary).get("treasury", 0))
		province_panel.settlement_build_requested.emit(first, offered)
		var after := int((sim.call("get_faction_economy", player) as Dictionary).get("treasury", 0))
		check(after < before, "building from the province view pays the cost")
		check(not (sim.call("settlement_detail", first) as Dictionary).get("construction", {}).is_empty(), "construction started in the chosen settlement")
		# 4. Annulation avec confirmation chiffrée, même tour : remboursement complet.
		var quote: Dictionary = sim.call("settlement_cancel_quote", first, -1)
		check(not quote.is_empty() and int(quote["refund"]) == int(quote["paid"]), "same-turn quote is the full price")
		ctl.request_cancel_build(first, -1)
		check(ctl.cancel_dialog.visible, "cancel asks for confirmation")
		var expected := "Vous récupérerez %s sur %s investis (0 tour écoulé sur %d)." % [Money.amount(int(quote["refund"])), Money.amount(int(quote["paid"])), int(quote["total"])]
		check(ctl.cancel_dialog.text_label.text == expected, "cancel text: %s" % ctl.cancel_dialog.text_label.text)
		check((sim.call("settlement_detail", first) as Dictionary).get("construction", {}).size() > 0, "nothing cancelled before confirmation")
		ctl.cancel_dialog.yes_button.pressed.emit()
		var final_treasury := int((sim.call("get_faction_economy", player) as Dictionary).get("treasury", 0))
		check(final_treasury == before, "full refund the same turn: %d vs %d" % [final_treasury, before])

	# 3. Liste « Colonies » : marteaux, vue à côté sans fermer la liste.
	var holdings: Node = map.holdings_ctl
	holdings.toggle()
	holdings.set_filter("all")
	var table_hammers: Array = holdings.panel.find_children("BuildHammer", "Button", true, false)
	check(table_hammers.size() > 0, "Colonies list has hammers")
	holdings.open_build_view(second)
	check(holdings.panel.visible and holdings.companion_panel.visible and holdings.companion.view.settlement_id == second, "Colonies window opens the build view alongside")
	map.queue_free()


func _buildable_id(sim: Object, settlement: String) -> String:
	for option in sim.call("settlement_buildable", settlement):
		if bool(option.get("available", false)) and int(option.get("cost", 0)) > 0:
			return str(option["building"])
	return ""
