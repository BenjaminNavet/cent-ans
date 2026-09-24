extends SceneTree

## Captures H11 : panneau de faction (budget, Monnaie après trois saisons de monnaie affaiblie,
## infobulle d'un niveau, Ordre de chevalerie) et fenêtre « Captifs et rançons » (données
## simulées : aucun captif en 1337).
## Usage (avec affichage, pas en headless) :
##   godot --path game --script res://tests/coinage_screenshot.gd
## Écrit `docs/img/coinage.png` et `docs/img/ransoms.png`.

const FACTION_ID := "fac_france"


func _init() -> void:
	await process_frame
	var out_dir := ProjectSettings.globalize_path("res://").path_join("../docs/img").simplify_path()
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var store: Node = root.get_node("/root/CodexStore")
	store.call("use_test_file")
	var facade: Node = root.get_node("/root/SimFacade")
	var sim: Object = ClassDB.instantiate("CampaignSim")
	sim.call("new_campaign", data_dir, FACTION_ID, 1337)
	sim.call("submit_order", {"type": "set_coinage", "level": "debased"})
	for _turn in 3:
		sim.call("end_turn")
	facade.set("sim", sim)

	var backdrop := ColorRect.new()
	backdrop.color = Color(0.16, 0.13, 0.10)
	backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(backdrop)
	var layer := Control.new()
	layer.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(layer)
	var panel: FactionPanel = (load("res://scenes/ui/faction_panel.tscn") as PackedScene).instantiate()
	layer.add_child(panel)
	await process_frame
	panel.show_faction(FACTION_ID, "France", facade.call("faction_color", FACTION_ID), sim.call("get_faction_economy", FACTION_ID))
	for _i in 4:
		await process_frame
	var options: Array = (sim.call("get_coinage", "") as Dictionary).get("options", [])
	var tip := RichTooltip.make_panel(RichTooltip.coinage(options[3], true))
	root.add_child(tip)
	tip.position = Vector2(panel.global_position.x - 360, 300)
	# Refus du second changement de l'année, affiché en rouge.
	panel.coinage_section.request_level("strong")
	for _i in 6:
		await process_frame
	_save(out_dir.path_join("coinage.png"))
	tip.queue_free()

	panel.toggle_ransoms()
	panel.ransom_panel.show_data(_mock_ransoms())
	panel.ransom_panel.pay_ransom("chr_jean_de_normandie", 1)  # refus réel du pont
	for _i in 6:
		await process_frame
	panel.ransom_panel.position = Vector2(maxf(8.0, panel.global_position.x - panel.ransom_panel.size.x - 12.0), 56)
	panel._scroll.scroll_vertical = 10000  # Ordre de chevalerie et bouton des rançons visibles
	for _i in 3:
		await process_frame
	_save(out_dir.path_join("ransoms.png"))
	store.call("reset_discoveries")
	quit(0)


static func _mock_ransoms() -> Dictionary:
	return {
		"ours": [{"character": "chr_jean_de_normandie", "name": "Jean, duc de Normandie", "faction": "fac_france", "captor": "fac_england",
			"rank": "heir", "rank_label": "Héritier", "prestige": 25, "ransom": 6250,
			"terms": {"kind": "money", "province": ""},
			"plans": [2, 3, 4, 5, 6].map(func(n: int) -> Dictionary: return {"installments": n, "total": 6900, "installment": 6900 / n}),
			"cedable_provinces": []}],
		"held": [
			{"character": "chr_jean_de_montfort", "name": "Jean de Montfort", "faction": "fac_brittany", "captor": "fac_france",
				"rank": "great_lord", "rank_label": "Grand seigneur", "prestige": 30, "ransom": 1950,
				"terms": {"kind": "province", "province": "prov_bretagne"}, "plans": [], "cedable_provinces": ["prov_bretagne"]},
			{"character": "chr_mock_knight", "name": "Thomas Holland", "faction": "fac_england", "captor": "fac_france",
				"rank": "knight", "rank_label": "Chevalier", "prestige": 12, "ransom": 450,
				"terms": {"kind": "money", "province": ""}, "plans": [], "cedable_provinces": ["prov_guyenne"]}],
		"debts": [{"character": "chr_charles_de_blois", "name": "Charles de Blois", "creditor": "fac_england",
			"remaining": 9000, "installment": 3000, "next_due_turn": 4, "missed": 1}],
	}


func _save(path: String) -> void:
	var image := root.get_texture().get_image()
	var error := image.save_png(path)
	print("coinage screenshot %s: %s" % [path, "ok" if error == OK else "error %d" % error])
