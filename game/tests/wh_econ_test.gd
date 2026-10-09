extends TestCase

## Test headless WH econ : économie lisible. Vraie simulation (France, graine 1337) : termes signés du
## mécontentement dans l'infobulle de jauge, revenu d'une colonie et budget décomposés par source
## (sommes cohérentes), impôt par province (section et ordre), « Emplacements n/m » de la bande.
## Usage : godot --headless --path game --script res://tests/wh_econ_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _wait(frames: int) -> void:
	for i in frames:
		await process_frame


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_income_breakdown"),
			"CampaignSim.get_income_breakdown missing (run core/build.sh)"):
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
	root.size = Vector2i(1280, 720)
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _wait(10)
	if not check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var sim: Object = map.sim

	# 1. Mécontentement : termes signés, repris par l'infobulle de la jauge.
	var city: Dictionary = sim.call("get_province_city", "prov_ile_de_france")
	var peasants: Dictionary = city.get("classes", {}).get("peasants", {})
	var terms: Array = peasants.get("unrest_terms", [])
	check(not terms.is_empty(), "peasants must expose unrest_terms")
	var tooltip: String = RichTooltip.gauge("unrest", float(peasants.get("unrest", 0)), terms)
	check(tooltip.contains("Cible d'équilibre") and tooltip.contains("Impôt"), "gauge tooltip lists the terms: %s" % tooltip)

	# 2. Revenus : colonie et budget, somme des lignes = total annoncé.
	var detail: Dictionary = sim.call("settlement_detail", "set_paris")
	var lines: Array = detail.get("income_lines", [])
	var sum := 0
	for line in lines:
		sum += int(line["value"])
	check(sum == int(detail.get("income", -1)), "settlement lines %d vs income %d" % [sum, int(detail.get("income", -1))])
	var economy: Dictionary = sim.call("get_faction_economy", "fac_france")
	var total := 0
	for line in economy.get("income_lines", []):
		total += int(line["value"])
	var receipts := 0
	for line in economy.get("budget_lines", []):
		if str(line["key"]) == "receipts":
			receipts = int(line["projected"])
	check(total == receipts, "receipt lines %d vs budget line %d" % [total, receipts])
	var table := BudgetTable.new()
	root.add_child(table)
	table.show_budget(economy)
	check(table.cells.has("receipts/poll_peasants"), "budget table shows the peasants' poll tax")
	table.queue_free()

	# 3. Impôt par province : ordre accepté, effet visible, retour au taux commun.
	var section := ProvinceTaxSection.new()
	root.add_child(section)
	section.show_for("prov_ile_de_france", true, sim)
	var before: int = int(sim.call("settlement_detail", "set_paris").get("income", 0))
	var result: Dictionary = section.request_rate("low")
	check(bool(result.get("ok", false)), "set_province_tax refused: %s" % str(result))
	var tax: Dictionary = sim.call("get_province_tax", "prov_ile_de_france")
	check(str(tax.get("rate", "")) == "low" and bool(tax.get("own", false)), "province bracket should be low, got %s" % str(tax))
	var after: int = int(sim.call("settlement_detail", "set_paris").get("income", 0))
	check(after < before, "a low bracket lowers the income (%d -> %d)" % [before, after])
	section.queue_free()

	# 4. Emplacements n/m.
	var usage: Dictionary = sim.call("settlement_slot_usage", "set_paris")
	check(int(usage.get("max", -1)) > 0, "Paris has a slot cap, got %s" % str(usage))
	map.settlement_layer.select("set_paris")
	await _wait(10)
	var title: Label = map.settlements_ctl.slot_bar.title_label
	check(title.text.contains("Emplacements"), "slot bar title: %s" % title.text)
	print("wh_econ: terms %d, income %d (lines %d), budget lines %d, slots %s, title '%s'" % [terms.size(), int(detail.get("income", 0)), lines.size(), total, str(usage), title.text.replace("\n", " | ")])
	map.queue_free()
