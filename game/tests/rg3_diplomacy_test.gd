extends TestCase

## Test headless RG3 : saisie libre d'or/tribut bornée, projection de l'accord commercial,
## brouillon vidé après un traité accepté, clause déjà en vigueur retirée.
## Usage : godot --headless --path game --script res://tests/rg3_diplomacy_test.gd


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
	var sim: Object = map.sim
	# Un SubViewport de 1280×720 : la taille de base du viewport, quel que soit l'écran de la machine.
	var sub := SubViewport.new()
	sub.size = Vector2i(1280, 720)
	sub.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	root.add_child(sub)
	var panel: Control = (load("res://scripts/ui/diplomacy_panel.gd") as GDScript).new()
	panel.set("sim", sim)
	panel.set("player_faction", "fac_france")
	panel.set("map_data", map.map_data)
	panel.hide()
	var host := Control.new()
	host.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	sub.add_child(host)
	host.add_child(panel)
	panel.call("refresh")
	panel.call("select_faction", "fac_england")
	panel.show()
	for index in 6:
		await process_frame
	var negotiation: DiplomacyNegotiationTab = panel.get("negotiation")
	check(DiplomacyNegotiationTab.clamp_amount(99999.0, 500) == 500, "amount clamped to the maximum")
	check(DiplomacyNegotiationTab.clamp_amount(-5.0, 500) == 0, "negative amount clamped to 0")
	var treasury := int(((negotiation.get("_options") as Dictionary).get("ours", {}) as Dictionary).get("treasury", 0))
	check(treasury > 0, "player treasury known (%d)" % treasury)
	negotiation.articles = []
	check(negotiation.add_amount_article("gold", "proposer", 99999999.0), "huge gold amount accepted then clamped")
	check(int(negotiation.articles[0]["amount"]) == treasury, "gold clamped to the treasury (%s)" % str(negotiation.articles))
	check(not negotiation.add_amount_article("gold", "proposer", 0.0), "zero gold refused")
	check(negotiation.add_amount_article("gold", "proposer", 123.0) and negotiation.articles.size() == 1 and int(negotiation.articles[0]["amount"]) == 123, "free amount replaces the previous gold clause")
	check((negotiation.gold_inputs["proposer"] as SpinBox).max_value == treasury, "spin box bounded by the treasury")
	negotiation.articles = [{"kind": "trade_agreement"}]
	negotiation.render_draft()
	var found := false
	for label in negotiation.find_children("TradeProjection", "Label", true, false):
		found = true
		check(label.text.contains("par saison pour vous") or label.text.contains("aucun gain"), "projection text: %s" % label.text)
	check(found, "trade projection shown next to the article")
	# Traité accepté : brouillon vidé.
	negotiation.articles = [{"kind": "trade_agreement"}, {"kind": "gold", "giver": "proposer", "amount": 100}]
	panel.order_resolved({"type": "propose_treaty"}, false)
	check(negotiation.articles.size() == 2, "refused treaty keeps the draft")
	panel.order_resolved({"type": "propose_treaty"}, true)
	check(negotiation.articles.is_empty(), "accepted treaty empties the draft")
	# Clause déjà en vigueur retirée.
	negotiation.articles = [{"kind": "trade_agreement"}, {"kind": "military_access", "giver": "proposer"}]
	var options: Dictionary = negotiation.get("_options")
	options["trade"] = true
	negotiation.call("_prune_in_force")
	check(negotiation.articles.size() == 1 and negotiation.articles[0]["kind"] == "military_access", "signed trade agreement removed from the draft")
	sub.queue_free()
	map.queue_free()
