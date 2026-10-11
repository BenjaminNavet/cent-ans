extends TestCase

## RG12 : fenêtre Économie — bouton et touche, ouverture, trois onglets, données du cœur
## (recettes ventilées, prévisions, historique, partenaires triés, monnaie), ordre d'accord
## commercial, liens depuis la diplomatie, la touche I et le panneau de faction.
## Usage : godot --headless --path game --script res://tests/rg12_economy_panel_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not check(map.load_ok and map.sim != null, "campaign map failed to start"):
		return
	var economy: EconomyController = map.economy
	if not check(economy != null and economy.panel != null, "economy controller missing"):
		return
	var window: EconomyWindow = economy.panel
	var ui: MapUI = map.ui
	var sim: Object = map.sim

	# Bouton, touche libre, enregistrement dans la pile des panneaux.
	check(economy.button != null and economy.button.name == "EconomyButton", "top bar button Économie")
	check(InputMap.has_action("map_toggle_economy") and not InputMap.action_get_events("map_toggle_economy").is_empty(), "key bound to map_toggle_economy")
	var taken := 0
	for event in InputMap.action_get_events("map_toggle_economy"):
		for other in InputMap.get_actions():
			if str(other) == "map_toggle_economy" or str(other).begins_with("ui_"):
				continue
			if InputMap.action_has_event(other, event):
				taken += 1
	check(taken == 0, "economy key is free (shared with %d actions)" % taken)
	check(ui.panels.is_registered(window), "window registered in the panel stack")
	check(not window.visible, "window starts hidden")
	check(window.get_node_or_null("ResizeGrip") != null, "resize grip attached (RG1)")

	# Ouverture par l'action de la touche.
	ui.press_action("map_toggle_economy")
	await process_frame
	await process_frame
	check(window.visible, "key opens the window")
	ui.press_action("map_toggle_economy")
	await process_frame
	check(not window.visible, "key closes the window")

	# Onglets.
	economy.open_panel(EconomyWindow.TAB_OVERVIEW)
	check(window.visible and window.tabs.get_tab_count() == 3, "three tabs")
	check(window.current_tab() == EconomyWindow.TAB_OVERVIEW, "opens on overview")

	# Vue d'ensemble : recettes ventilées = get_income_breakdown, solde, prévisions, historique.
	var breakdown: Dictionary = sim.call("get_income_breakdown", "fac_france")
	var overview := window.overview_tab
	check(not (breakdown.get("lines", []) as Array).is_empty(), "income breakdown has lines")
	check(int(overview.shown.get("income_total", -1)) == int(breakdown.get("total", -2)), "overview income total matches get_income_breakdown")
	var economy_data: Dictionary = sim.call("get_faction_economy", "fac_france")
	check(int(overview.shown.get("net", 0)) == int(economy_data.get("net_income", 1)), "overview balance matches the core")
	check(int(overview.shown.get("treasury", 0)) == int(economy_data.get("treasury", 1)), "overview treasury matches the core")
	check(overview.budget_table.cells.has("receipts") and overview.budget_table.cells.has("net"), "budget table shows receipts and balance")
	check((overview.shown.get("gauges", []) as Array).size() == 3, "three forecast gauges (prestige, men, unrest)")
	check(overview.drivers_label.text != "", "main causes shown")
	check(overview.shown.has("history"), "history section built")
	# L'historique du solde est celui du cœur (vide avant la première fin de tour).
	check(int(overview.shown.get("history", -1)) == mini((economy_data.get("budget_history", []) as Array).size(), EconomyOverviewTab.HISTORY_SHOWN), "history matches the core")

	# Commerce : routes et partenaires triés, projection avant signature.
	economy.open_panel(EconomyWindow.TAB_TRADE)
	await process_frame
	var trade := window.trade_tab
	var partners: Array = sim.call("get_trade_partner_options", "fac_france")
	check(not partners.is_empty(), "core lists trade partners for France")
	var sorted_ok := true
	for index in range(1, partners.size()):
		if int(partners[index - 1].get("gain_self", 0)) < int(partners[index].get("gain_self", 0)):
			sorted_ok = false
	check(sorted_ok, "partners sorted by gain")
	check(not trade.partner_rows.is_empty() or not trade.agreement_rows.is_empty(), "trade tab shows partners or agreements")
	var previous := 1 << 40
	var rows_sorted := true
	for row in trade.partner_rows:
		if int(row["gain_self"]) > previous:
			rows_sorted = false
		previous = int(row["gain_self"])
		check(str(row["text"]).contains("par saison pour nous"), "projection text on partner row: %s" % row["text"])
	check(rows_sorted, "partner rows sorted by projected gain")
	check(trade.map_button != null, "« Voir sur la carte » button")

	# Ordre d'accord : le bouton « Proposer » envoie `propose_treaty` au cœur.
	var signed_id := ""
	var tried := 0
	for row in trade.partner_rows.duplicate():
		tried += 1
		var button: Button = trade.buttons["propose"].get(row["id"])
		check(button != null, "propose button for " + str(row["id"]))
		if button == null:
			continue
		button.pressed.emit()
		await process_frame
		for agreement in window.trade_tab.agreement_rows:
			if agreement["id"] == row["id"]:
				signed_id = str(row["id"])
		if signed_id != "" or tried >= 8:
			break
	check(tried > 0, "at least one partner proposed")
	if signed_id != "":
		var projection: Dictionary = sim.call("get_trade_agreement_projection", "fac_france", signed_id)
		check(bool(projection.get("signed", false)), "agreement signed in the core")
		check(window.trade_tab.buttons["break"].has(signed_id), "break button for the new agreement")
		check(not window.trade_tab.buttons["propose"].has(signed_id), "no more proposal for the signed partner")
		# Rompre : deux pressions (confirmation).
		var drop: Button = window.trade_tab.buttons["break"][signed_id]
		drop.pressed.emit()
		check(drop.text == "Confirmer ?", "break asks for confirmation")
		drop.pressed.emit()
		await process_frame
		check(not bool(sim.call("get_trade_agreement_projection", "fac_france", signed_id).get("signed", true)), "agreement broken in the core")
	else:
		print("rg12: no partner accepted in 8 tries, signing path covered by order emission only")

	# « Voir sur la carte » active le calque des routes.
	var layer_was: bool = map.trade_mode
	window.trade_layer_requested.emit()
	await process_frame
	check(map.trade_mode != layer_was or layer_was, "show on map enables the trade layer")
	check(not window.visible, "window closes to show the map")

	# Monnaie : quatre niveaux de frappe affichés.
	economy.open_panel(EconomyWindow.TAB_CURRENCY)
	await process_frame
	check(window.currency_tab.rows.size() == 4, "four coinage levels shown (%d)" % window.currency_tab.rows.size())
	check(window.currency_tab.name_label.get_child_count() > 0, "currency name chip")
	economy.close_panel()

	# Liens : diplomatie, touche I, panneau de faction.
	map.diplomacy.panel.trade_view_requested.emit()
	await process_frame
	check(window.visible and window.current_tab() == EconomyWindow.TAB_TRADE, "diplomacy « Commerce » link opens the Commerce tab")
	economy.close_panel()
	ui.hotkey_pressed.emit("campaign_commerce")
	await process_frame
	check(window.visible and window.current_tab() == EconomyWindow.TAB_TRADE, "key I opens the Commerce tab")
	ui.hotkey_pressed.emit("campaign_commerce")
	await process_frame
	check(not window.visible, "key I again closes it")
	ui.faction_panel.toggle_trade()
	await process_frame
	check(window.visible and window.current_tab() == EconomyWindow.TAB_TRADE, "faction panel button opens the Commerce tab")
	var button_text := ""
	for button in map.diplomacy.panel.find_children("TradeButton", "Button", true, false):
		button_text = (button as Button).text
	check(button_text.contains("Économie"), "diplomacy button reads Commerce → Économie (%s)" % button_text)
	map.queue_free()
