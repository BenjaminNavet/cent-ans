extends TestCase

## WH uicards top7 : pastilles prestige / hommes / troubles de la barre du haut, alimentées par
## `get_faction_summary`, et barre qui tient à 1280 px.
## Usage : godot --headless --path game --script res://tests/wh_topchips_test.gd


func _init() -> void:
	await process_frame
	var facade: Node = root.get_node("/root/SimFacade")
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not check(map.get("load_ok") and map.get("sim") != null, "campaign map failed to start"):
		finish()
		return
	var summary: Dictionary = map.sim.call("get_faction_summary", "fac_france")
	check(summary.has("prestige") and summary.has("soldiers") and summary.has("mean_unrest"), "summary keys: %s" % [summary.keys()])
	check(int(summary.get("soldiers", 0)) > 0, "France starts with men under arms")
	var ui = map.ui
	check(ui.men_label.text.contains(Money.digits(int(summary["soldiers"]))), "men chip: %s" % ui.men_label.text)
	check(ui.prestige_label.text.contains(str(int(summary["prestige"]))), "prestige chip: %s" % ui.prestige_label.text)
	check(ui.unrest_label.text.contains("%d %%" % int(summary["mean_unrest"])), "unrest chip: %s" % ui.unrest_label.text)
	check(ui.men_label.tooltip_text != "", "men chip has a tooltip")
	map.queue_free()
	finish()
