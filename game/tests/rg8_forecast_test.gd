extends TestCase

## RG8 : prévisions du prochain tour dans la barre du haut, sur la vraie simulation.
##  1. `get_faction_forecast` : prestige / hommes / troubles avec courant, prochain, termes triés ;
##  2. pastilles : texte avec flèche ou « = », infobulle avec « Actuel → Prochain tour » ;
##  3. solde et trésor : infobulle avec la prévision ; trésor prochain = trésor + solde.
## Usage : godot --headless --path game --script res://tests/rg8_forecast_test.gd


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _run() -> void:
	if not check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_faction_forecast"),
			"CampaignSim.get_faction_forecast missing (run core/build.sh)"):
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
	var sim: Object = map.sim
	var player: String = map.player_faction

	# 1. Données du cœur.
	var forecast: Dictionary = sim.call("get_faction_forecast", player)
	for gauge in ["prestige", "men", "unrest"]:
		var g: Dictionary = forecast.get(gauge, {})
		check(g.has("current") and g.has("next") and g.has("delta") and g.has("contributions"), "forecast %s incomplete" % gauge)
		check(int(g.get("next", 0)) - int(g.get("current", 0)) == int(g.get("delta", 1)), "forecast %s delta" % gauge)
	var unrest_terms: Array = forecast["unrest"]["contributions"]
	check(unrest_terms.size() >= 2, "unrest forecast has terms")
	for i in range(1, unrest_terms.size()):
		check(absf(float(unrest_terms[i - 1]["amount"])) >= absf(float(unrest_terms[i]["amount"])), "unrest terms sorted")

	# 2. Pastilles : texte et infobulle.
	map._refresh_top_bar()
	var ui: Node = map.ui
	for spec in [[ui.prestige_label, "prestige"], [ui.men_label, "men"], [ui.unrest_label, "unrest"]]:
		var chip: Control = spec[0]
		var text: String = chip.text
		check("▲" in text or "▼" in text or "=" in text, "chip %s carries a delta: %s" % [spec[1], text])
		check("Actuel → Prochain tour" in chip.tooltip_text, "chip %s tooltip has the forecast line" % spec[1])

	# 3. Solde et trésor.
	var economy: Dictionary = sim.call("get_faction_economy", player)
	var treasury := int((sim.call("get_faction_summary", player) as Dictionary).get("treasury", 0))
	var net := int(economy.get("net_income", 0))
	check("Actuel → Prochain tour" in ui.treasury_label.tooltip_text, "treasury tooltip has the forecast line")
	check(Money.amount(treasury + net) in ui.treasury_label.tooltip_text, "next treasury = treasury + net")
	check("Actuel → Prochain tour" in ui.income_label.tooltip_text, "net balance tooltip has the forecast line")
	check("Principales recettes" in ui.income_label.tooltip_text, "net balance tooltip lists the main sources")

	# Texte pur : « Autres » reçoit le reste au-delà de cinq termes.
	var terms: Array = []
	for i in 7:
		terms.append({"key": "k%d" % i, "label": "Terme %d" % i, "amount": 10 - i})
	var text := ForecastText.tooltip({"current": 5, "next": 40, "delta": 35, "other": 0.0, "contributions": terms})
	check("Autres" in text and "Terme 4" in text and not ("Terme 5" in text), "top 5 then Autres")
	map.queue_free()
