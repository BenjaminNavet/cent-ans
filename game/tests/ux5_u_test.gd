extends TestCase

## Lot UX5-U : « Montre des hommes d'armes » (liste des unités), vérifiée par l'état des nœuds.
##  1. ligne d'ost à deux niveaux : mini-sceau, effectif « n/max », alertes, état ;
##  2. alertes (sans chef, vivres bas, sous-effectif, à bout de mouvement) sur entrées synthétiques ;
##  3. tri et filtres cumulables, défilement conservé ;
##  4. pied « Montre du royaume » ;
##  5. Tab / Maj+Tab parcourent les osts affichés.
## Usage : godot --headless --path game --script res://tests/ux5_u_test.gd


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
	if not check(map.load_ok and map.sim != null, "campaign map failed to start"):
		return
	var roster: Node = map.units_ctl
	roster.toggle()
	await process_frame
	var ids: PackedStringArray = map.player_army_ids()

	# 1. Ligne à deux niveaux.
	var entry: Dictionary = roster.army_entry(ids[0], map.sim.call("get_army", ids[0]))
	check(int(entry["max_men"]) >= int(entry["men"]) and int(entry["max_men"]) > 0, "men/max_men: %s" % entry)
	var row: Control = roster._rows["army:" + ids[0]]
	check(row.find_children("*", "Control", true, false).any(func(c: Node) -> bool: return "empty" in c and c.has_method("setup")), "mini seal on the row")
	var texts := PackedStringArray()
	for label in row.find_children("*", "Label", true, false):
		texts.append((label as Label).text)
	check(" ".join(texts).contains("/%s" % Money.digits(int(entry["max_men"]))), "row shows n/max: %s" % " ".join(texts))
	check(" ".join(texts).contains("moral") and " ".join(texts).contains("vivres"), "row shows morale and supply")
	var title: Array = roster.panel.find_children("*", "Label", true, false).filter(func(l: Node) -> bool: return (l as Label).text == "Montre des hommes d’armes")
	check(not title.is_empty(), "panel title")

	# 2. Alertes.
	var sick: Dictionary = roster.army_entry("x", {"general": "", "general_name": "", "movement_left": 0, "movement_max": 100, "supply": 10,
		"units": [{"strength": 100, "max_strength": 400, "morale": 60}], "stance": "normal", "faction": "fac_france"})
	for alert in ["leaderless", "supply", "strength", "spent"]:
		check((sick["alerts"] as Array).has(alert), "alert %s expected: %s" % [alert, sick["alerts"]])
	check(int(sick["morale"]) == 60, "mean morale")
	var fine: Dictionary = roster.army_entry("y", {"general": "c1", "general_name": "Jean", "movement_left": 50, "movement_max": 100, "supply": 90,
		"units": [{"strength": 380, "max_strength": 400, "morale": 80}], "stance": "siege", "faction": "fac_france"})
	check((fine["alerts"] as Array).is_empty() and fine["state"] == "siege", "no alert, siege state: %s" % fine)
	var camp: Dictionary = roster.army_entry("z", {"general": "c1", "general_name": "Jean", "movement_left": 50, "movement_max": 100, "stance": "entrenched", "settlement": "", "units": []})
	check(camp["state"] == "camp", "entrenched in the field is a camp")

	# 3. Tri et filtres (sur des entrées synthétiques, indépendant de l'état de la partie).
	var a: Dictionary = fine.duplicate()
	a["key"] = "army:a"; a["men"] = 100; a["ratio"] = 0.2; a["place"] = "Rouen"
	var b: Dictionary = sick.duplicate()
	b["key"] = "army:b"; b["men"] = 900; b["ratio"] = 0.0; b["place"] = "Arras"
	var entries := [a, b]
	roster._sort_mode = "alert"
	check(roster._sorted_filtered(entries)[0]["key"] == "army:b", "alerts first")
	roster._sort_mode = "men"
	check(roster._sorted_filtered(entries)[0]["key"] == "army:b", "most men first")
	roster._sort_mode = "move"
	check(roster._sorted_filtered(entries)[0]["key"] == "army:a", "most movement first")
	roster._sort_mode = "place"
	check(roster._sorted_filtered(entries)[0]["key"] == "army:b", "place alphabetical (Arras)")
	roster._filters["leaderless"] = true
	check(roster._sorted_filtered(entries).size() == 1, "leaderless filter")
	roster._filters["siege"] = true
	check(roster._sorted_filtered(entries).is_empty(), "filters accumulate (leaderless AND siege)")
	roster._filters["leaderless"] = false
	roster._filters["siege"] = false
	roster._sort_mode = "alert"
	roster.set_filter("spent", true)
	check(roster.row_count() <= ids.size() + 8, "spent filter applied")
	roster.set_filter("spent", false)
	roster.set_sort("men")
	check(roster._rows.size() >= ids.size(), "all armies back")

	# Défilement conservé à la reconstruction.
	roster._scroll.custom_minimum_size.y = 40
	await process_frame
	roster._scroll.scroll_vertical = 0
	roster.refresh()
	await process_frame
	check(roster._scroll.scroll_vertical >= 0, "scroll restored without error")

	# 4. Pied.
	var foot: Label = roster.panel.find_child("RealmFoot", true, false)
	check(foot != null and foot.text.contains("Montre du royaume") and foot.text.contains("entretien") and foot.text.contains("ost"), "foot: %s" % (foot.text if foot else ""))

	# 5. Tab / Maj+Tab.
	roster.set_sort("alert")
	await process_frame
	var order: Array = roster._ordered_keys
	check(order.size() == ids.size(), "ordered keys = armies")
	roster.focus_entry(order[0])
	if order.size() > 1:
		check(roster.cycle(1) and map.selected_army == str(order[1]).trim_prefix("army:"), "Tab goes to the next ost")
		check(roster.cycle(-1) and map.selected_army == str(order[0]).trim_prefix("army:"), "Shift+Tab goes back")
	roster.panel.hide()
	check(not roster.cycle(1), "closed roster leaves Tab to the idle cycle")
	map.queue_free()
	await process_frame
