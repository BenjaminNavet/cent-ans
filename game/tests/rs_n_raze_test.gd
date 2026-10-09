extends TestCase

## Test headless du lot RS-N (bouton « Raser ») sur la vraie simulation et les vraies données
## `data/` :
##  1. panneau d'une colonie française avec un bâtiment dépendant d'un autre (marché → maison
##     des métiers) : le marché montre un bouton « Raser » désactivé, avec la raison du cœur en
##     tooltip ; la maison des métiers (sans dépendant) montre un bouton actif ;
##  2. clic sur le bouton actif → confirmation (`ConfirmPanel`) ; renoncer laisse le
##     bâtiment en place ; confirmer envoie l'ordre `demolish`, le bâtiment disparaît de la
##     colonie et le remboursement est versé.
## Usage : godot --headless --path game --script res://tests/rs_n_raze_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("settlement_demolition_preview"),
			"CampaignSim.settlement_demolition_preview missing (run core/build.sh)"):
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
	if not check(ctl != null and ctl.available(), "SettlementController missing or inactive"):
		map.queue_free()
		return
	var sim: Object = map.sim

	# Une colonie française avec la chaîne « marché → maison des métiers » : le marché a un
	# dépendant, la maison des métiers n'en a pas.
	var settlement_id := _french_settlement_with_chain(sim)
	if not check(settlement_id != "", "no French settlement with both bld_market and bld_weaving_workshop"):
		map.queue_free()
		return

	map.settlement_layer.select(settlement_id)
	var panel: Control = ctl.panel
	check(panel.visible and panel.settlement_id == settlement_id, "settlement panel should open on %s" % settlement_id)
	panel.show_buildings_tab()

	var market_row := _building_row(panel.buildings_list, "bld_market")
	var weaving_row := _building_row(panel.buildings_list, "bld_weaving_workshop")
	if not check(market_row != null, "no row for bld_market"):
		map.queue_free()
		return
	if not check(weaving_row != null, "no row for bld_weaving_workshop"):
		map.queue_free()
		return
	var market_button := market_row.get_node_or_null("RazeButton") as Button
	var weaving_button := weaving_row.get_node_or_null("RazeButton") as Button
	check(market_button != null, "bld_market row should have a Raser button")
	check(weaving_button != null, "bld_weaving_workshop row should have a Raser button")
	if market_button == null or weaving_button == null:
		map.queue_free()
		return

	# 1. Le marché est refusé (la maison des métiers en dépend) ; raison en français, pas
	# technique, donnée par le cœur.
	check(market_button.disabled, "raising bld_market should be refused (bld_weaving_workshop depends on it)")
	check(market_button.tooltip_text.contains("dépend"), "market tooltip should give the core's French reason: %s" % market_button.tooltip_text)
	check(not weaving_button.disabled, "bld_weaving_workshop has no dependent: Raser should be enabled")
	check(weaving_button.tooltip_text.contains("Rembourse"), "weaving tooltip should mention the refund: %s" % weaving_button.tooltip_text)

	# 2. Clic sur le marché (désactivé) : aucun ordre, aucune confirmation.
	market_button.emit_signal("pressed")
	check(not ctl.raze_dialog.visible, "a disabled Raser button should not open the confirmation")

	# 3. Clic sur la maison des métiers : confirmation ouverte, renoncer laisse le bâtiment.
	weaving_button.emit_signal("pressed")
	check(ctl.raze_dialog.visible, "Raser should open the confirmation dialog")
	ctl.raze_dialog._on_cancel()
	var still_there: Dictionary = sim.call("settlement_detail", settlement_id)
	check((still_there.get("buildings", PackedStringArray()) as PackedStringArray).has("bld_weaving_workshop"),
		"cancelling the confirmation should not raze the building")

	# 4. Confirmer : ordre `demolish`, bâtiment rasé, remboursement versé.
	var treasury_before := int(sim.call("get_faction_summary", "fac_france").get("treasury", 0))
	weaving_row = _building_row(panel.buildings_list, "bld_weaving_workshop")
	weaving_button = weaving_row.get_node_or_null("RazeButton") as Button
	weaving_button.emit_signal("pressed")
	check(ctl.raze_dialog.visible, "Raser should reopen the confirmation dialog")
	ctl.raze_dialog._on_confirm()
	var after: Dictionary = sim.call("settlement_detail", settlement_id)
	check(not (after.get("buildings", PackedStringArray()) as PackedStringArray).has("bld_weaving_workshop"),
		"confirming should raze bld_weaving_workshop")
	var treasury_after := int(sim.call("get_faction_summary", "fac_france").get("treasury", 0))
	check(treasury_after > treasury_before, "razing should refund part of the building's cost (%d -> %d)" % [treasury_before, treasury_after])

	map.queue_free()
	await process_frame


## Colonie française (contrôlée et possédée) avec `bld_market` et `bld_weaving_workshop`
## construits ensemble (le second dépend du premier).
func _french_settlement_with_chain(sim: Object) -> String:
	for entry in sim.call("settlements"):
		if str(entry["owner"]) != "fac_france" or str(entry["controller"]) != "fac_france":
			continue
		var id := str(entry["id"])
		var detail: Dictionary = sim.call("settlement_detail", id)
		var buildings := detail.get("buildings", PackedStringArray()) as PackedStringArray
		if buildings.has("bld_market") and buildings.has("bld_weaving_workshop"):
			return id
	return ""


## Ligne (`HBoxContainer`, nommée `building_id`) de `buildings_list`.
func _building_row(list: Node, building_id: String) -> Node:
	return list.get_node_or_null(NodePath(building_id))
