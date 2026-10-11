extends TestCase

## Test headless du lot RG10 (ADR 0344) : vocations des colonies. Vraie simulation (France, graine 1337) :
## les getters du cœur via `SimFacade`, le sélecteur de vocation (textes, menu, ordre, verrou) et son
## intégration dans l'en-tête du panneau Bâtiments. Textes statiques testés sans simulation.
## Usage : godot --headless --path game --script res://tests/rg10_vocation_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	_check_texts()
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_vocation_options"),
			"CampaignSim.get_vocation_options missing (run core/build.sh)"):
		return
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	if not check(facade.new_campaign("fac_france", 1337), "new campaign"):
		return
	var sim: Object = facade.sim
	var id := ""
	for row in sim.call("settlements"):
		if str(row.get("controller", "")) == "fac_france" and str(row.get("kind", "")) == "city":
			id = str(row.get("id", ""))
			break
	if not check(id != "", "a French city"):
		return
	# Getters du cœur.
	var view: Dictionary = facade.get_settlement_vocation(id)
	check(str(view.get("vocation", "x")) == "", "no vocation at the start: %s" % str(view))
	check(bool(view.get("can_change", false)), "a settlement without vocation can choose")
	var options: Array = facade.get_vocation_options(id)
	check(options.size() == 6, "six vocations, got %d" % options.size())
	var ids := []
	for option in options:
		ids.append(str(option.get("id", "")))
		check(int(option.get("cost", 0)) > 0, "option %s has a cost" % str(option.get("id", "")))
		check(option.has("fit") and option.has("preview_effects") and option.has("tiers"), "option fields")
	check(ids.has("agricultural") and ids.has("fortified"), "ids: %s" % str(ids))

	# Sélecteur de vocation.
	var selector := VocationSelector.new()
	root.add_child(selector)
	var changed: Array = []
	selector.vocation_changed.connect(func(settlement: String, vocation: String) -> void: changed.append([settlement, vocation]))
	selector.set_settlement(id, true)
	check(selector.visible, "the selector shows for a real settlement")
	check(selector.button.text == "Vocation : aucune", "button text: %s" % selector.button.text)
	check(selector.button.get_popup().item_count == 6, "menu has 6 entries")
	check(selector.button.tooltip_text.contains("Aucune vocation"), "tooltip without vocation")
	var choice := ""
	for option in options:
		if bool(option.get("available", false)):
			choice = str(option.get("id", ""))
			break
	if not check(choice != "", "an affordable vocation: %s" % str(options)):
		return
	var result: Dictionary = selector.choose(choice)
	check(bool(result.get("ok", false)), "adopt %s: %s" % [choice, str(result)])
	check(changed == [[id, choice]], "vocation_changed emitted: %s" % str(changed))
	check(selector.button.text.begins_with("Vocation : ") and not selector.button.text.contains("aucune"), "button after: %s" % selector.button.text)
	var after: Dictionary = facade.get_settlement_vocation(id)
	check(str(after.get("vocation", "")) == choice, "core vocation after: %s" % str(after))
	check(not bool(after.get("can_change", true)) and int(after.get("locked_until", 0)) > 0, "locked after the change")
	# Verrou : un second choix est refusé par le cœur (message français), menu grisé.
	var other := ""
	for option in facade.get_vocation_options(id):
		if str(option.get("id", "")) != choice:
			other = str(option.get("id", ""))
			check(not bool(option.get("available", true)), "options locked after the change")
			check(str(option.get("reason", "")).contains("ne peut plus changer"), "lock reason: %s" % str(option.get("reason", "")))
			break
	var refused: Dictionary = selector.choose(other)
	check(not bool(refused.get("ok", true)) and selector.last_error() != "", "locked change refused: %s" % str(refused))
	check(changed.size() == 1, "no signal on a refused change")
	check(selector.button.tooltip_text.contains("verrouillé jusqu'au tour"), "tooltip shows the lock: %s" % selector.button.tooltip_text)
	selector.queue_free()

	# Intégration dans l'en-tête du panneau Bâtiments.
	var view_script: Variant = load("res://scripts/ui/settlement_buildings_view.gd")
	var buildings_view = view_script.new()
	root.add_child(buildings_view)
	buildings_view.set_data(id, [], {"used": 0, "max": 6}, 2, {"kind": "city"}, {}, true)
	check(buildings_view.vocation_selector != null and buildings_view.vocation_selector.visible, "header shows the selector")
	check(buildings_view.vocation_selector.button.text.begins_with("Vocation : "), "header text: %s" % buildings_view.vocation_selector.button.text)
	buildings_view.set_data(id, [], {"used": 0, "max": 6}, 2, {"kind": "city"}, {}, false)
	check(buildings_view.vocation_selector.button.disabled, "read-only for a foreign settlement")
	buildings_view.queue_free()
	await process_frame


func _check_texts() -> void:
	check(VocationSelector.button_text({}) == "Vocation : aucune", "empty view text")
	var view := {"vocation": "mercantile", "name": "Marchande", "description": "Le commerce.", "count": 2, "reached_tiers": 1,
		"effect_bonus_percent": 15, "cost_reduction_percent": 20, "off_vocation_surcharge_percent": 10,
		"tiers": [{"min_buildings": 2, "label": "Place marchande", "reached": true, "effects": [{"effect": "trade_income", "value": 5, "mode": "percent"}]},
			{"min_buildings": 4, "label": "Carrefour", "reached": false, "effects": []}],
		"next_tier": {"min_buildings": 4, "label": "Carrefour", "reached": false, "effects": []},
		"active_effects": [{"effect": "trade_income", "value": 8.75, "mode": "percent"}], "locked_until": 12}
	check(VocationSelector.button_text(view) == "Vocation : Marchande (1/2)", "button text: %s" % VocationSelector.button_text(view))
	var tip := VocationSelector.tooltip_text_for(view, [])
	check(tip.contains("+15 %") and tip.contains("−20 %") and tip.contains("+10 %"), "tooltip percents: %s" % tip)
	check(tip.contains("Prochain palier : encore 2 bâtiment(s)"), "next tier: %s" % tip)
	check(tip.contains("verrouillé jusqu'au tour 12"), "lock line")
	var label := VocationSelector.option_label({"name": "Militaire", "cost": 1250, "fit": 2})
	check(label.begins_with("Militaire — ") and label.contains("2 bât. concordants"), "option label: %s" % label)
	check(VocationSelector.option_label({"name": "Agricole", "current": true}) == "Agricole (actuelle)", "current option")
	check(VocationSelector.option_tooltip({"description": "d", "reason": "trésor insuffisant"}).contains("Impossible : trésor insuffisant"), "option reason")
