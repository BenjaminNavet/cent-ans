extends TestCase

## WH uicards top4 : le panneau de colonie affiche population et mécontentement de sa province
## (avec la mention de révolte), comme le panneau de province.
## Usage : godot --headless --path game --script res://tests/wh_settlement_rows_test.gd


func _init() -> void:
	await process_frame
	var panel: Control = load("res://scripts/map/settlement_panel.gd").new()
	root.add_child(panel)
	await process_frame
	var detail := {"id": "set_x", "name": "Test", "kind": "city", "province": "prov_x", "owner": "fac_france",
		"controller": "fac_france", "income": 100,
		"province_state": {"population_total": 12345, "unrest": 70, "revolt_seasons": 1, "revolt_seasons_needed": 3}}
	panel.show_settlement(detail)
	check(panel.unrest_value.text.begins_with("70 %"), "unrest text: %s" % panel.unrest_value.text)
	check(panel.unrest_value.text.contains("révolte"), "revolt mention: %s" % panel.unrest_value.text)
	check(panel.population_value.text.contains("12"), "population: %s" % panel.population_value.text)
	check(panel.unrest_value.tooltip_text != "", "unrest tooltip")
	detail.erase("province_state")
	panel.show_settlement(detail)
	check(panel.unrest_value.text == "—" and panel.population_value.text == "—", "no state shows dashes")
	finish()
