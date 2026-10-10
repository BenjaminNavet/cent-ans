extends TestCase

## Lot UX5-C : « Censier du royaume » (liste des colonies) : colonnes alignées, sceaux d'alerte,
## tri par en-tête, filtres cumulés + recherche, ligne de totaux.
## Usage : godot --headless --path game --script res://tests/ux5_c_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
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
	if not check(map.load_ok and map.sim != null and facade.is_real, "campaign map failed to start"):
		return
	var holdings: Node = map.holdings_ctl
	holdings.toggle()
	await process_frame
	var overview: Dictionary = map.sim.call("get_holdings_overview", map.player_faction)
	var provinces: Array = overview["provinces"]

	# 1. Colonnes : en-têtes, titre, une cellule par colonne.
	var panel: Control = holdings.panel
	var head := panel.find_child("ColumnHead", true, false) as Control
	check(head != null and head.find_children("*", "Button", false, false).size() == 5, "5 column headers expected")
	var has_title := panel.find_children("*", "Label", true, false).any(func(l) -> bool: return (l as Label).text == "Censier du royaume")
	check(has_title, "title should be 'Censier du royaume'")

	# Province avec le plus de colonies, dépliée.
	var target: Dictionary = provinces[0]
	for province in provinces:
		if (province["settlements"] as Array).size() > (target["settlements"] as Array).size():
			target = province
	var pid := str(target["province"])
	var settlements: Array = target["settlements"]
	holdings.set_expanded(pid, true)
	await process_frame
	check(holdings.settlement_order(pid).size() == settlements.size(), "all settlements listed with no filter")
	var rows := panel.find_children("Row_*", "Button", true, false)
	check(rows.size() >= settlements.size() and rows.size() > 0, "rows rendered for expanded province")
	if rows.size() > 0:
		var cells := (rows[0] as Control).get_node("Cells")
		check(cells.get_child_count() == 9, "5 cells + 4 rules per row, got %d" % cells.get_child_count())

	# 2. Totaux.
	var expected_income := 0
	var expected_garrison := 0
	var expected_idle := 0
	for province in provinces:
		for s in province["settlements"]:
			expected_income += int(s["income"])
			expected_garrison += int(s["garrison_strength"])
			expected_idle += 1 if bool(s["idle"]) else 0
	var totals: Dictionary = holdings.totals()
	check(int(totals["income"]) == expected_income and int(totals["income"]) == int(overview["settlement_income_total"]),
		"income total %s vs %d" % [totals["income"], expected_income])
	check(int(totals["garrison"]) == expected_garrison and int(totals["idle"]) == expected_idle, "garrison / idle totals")
	check(panel.find_child("Totals", true, false).get_child_count() >= 9, "totals row cells")

	# 3. Tri par colonne, puis inversion.
	if settlements.size() >= 2:
		holdings.sort_by_column("income")
		var incomes := _incomes(settlements, holdings.settlement_order(pid))
		check(_is_sorted(incomes, false), "income desc: %s" % str(incomes))
		holdings.sort_by_column("income")
		check(holdings.column_sort() == "income asc", "second click flips direction")
		check(_is_sorted(_incomes(settlements, holdings.settlement_order(pid)), true), "income asc after second click")
		holdings.sort_by_column("name")
		var names: Array = []
		for id in holdings.settlement_order(pid):
			for s in settlements:
				if s["id"] == id:
					names.append(str(s["name"]).to_lower())
		var sorted_names := names.duplicate()
		sorted_names.sort()
		check(names == sorted_names, "name asc")

	# 4. Filtres cumulés (ET) + recherche.
	holdings.set_filter("all")
	holdings.toggle_filter("idle")
	holdings.toggle_filter("upgrade")
	var both := 0
	for province in provinces:
		for s in province["settlements"]:
			if bool(s["idle"]) and not (s["options_available"] as Array).is_empty():
				both += 1
	check(holdings.active_filters().size() == 2, "two filters active")
	check(holdings.row_count() == both, "idle AND upgrade: %d vs %d" % [holdings.row_count(), both])
	holdings.toggle_filter("idle")
	holdings.toggle_filter("upgrade")
	check(holdings.active_filters().is_empty(), "filters cleared")
	var probe := str((settlements[0] as Dictionary)["name"]).substr(0, 3)
	holdings.set_search(probe.to_upper())
	var matches := 0
	for province in provinces:
		for s in province["settlements"]:
			if str(s["name"]).to_lower().find(probe.to_lower()) >= 0:
				matches += 1
	check(holdings.row_count() == matches and matches >= 1, "search by name: %d vs %d" % [holdings.row_count(), matches])
	holdings.set_search("zzzz-aucune")
	check(holdings.row_count() == 0, "search without match")
	holdings.set_search("")

	# 5. Sceaux d'alerte : un sceau « chantier libre » par colonie libre affichée, avec infobulle.
	holdings.set_filter("idle")
	await process_frame
	var seals := panel.find_children("Seal_idle", "Control", true, false)
	check(seals.size() == holdings.row_count(), "idle seals %d vs idle rows %d" % [seals.size(), holdings.row_count()])
	for seal in seals:
		check((seal as Control).tooltip_text != "", "seal tooltip")
	map.queue_free()


func _incomes(settlements: Array, order: Array) -> Array:
	var out: Array = []
	for id in order:
		for s in settlements:
			if s["id"] == id:
				out.append(int(s["income"]))
	return out


func _is_sorted(values: Array, ascending: bool) -> bool:
	for i in range(1, values.size()):
		if ascending and values[i - 1] > values[i]:
			return false
		if not ascending and values[i - 1] < values[i]:
			return false
	return true
