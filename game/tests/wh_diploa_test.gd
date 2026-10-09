extends TestCase

## Lot WH `diploa` : diplomatie lisible, vérifiée par l'état des nœuds (sans capture).
##  1. fiche d'une faction : étiquette qualitative d'attitude, noms cliquables d'alliés / ennemis /
##     vassaux (un clic ouvre la fiche) ;
##  2. aperçu de déclaration de guerre : prévision des alliés de la cible et coût en prestige ;
##  3. bouton « Retirer l'accès militaire » seulement si l'accès est accordé ;
##  4. historique : un pacte rompu est libellé et rouge.
## Usage : godot --headless --path game --script res://tests/wh_diploa_test.gd

var map: Node3D = null


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _wait(frames: int) -> void:
	for _i in frames:
		await process_frame


func _run() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	settings.call("use_test_file")
	settings.call("set_value", "tutorial/enabled", false, false)
	settings.call("set_value", "game/autosave_interval", 0, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	map = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await _wait(10)
	var controller: Node = map.diplomacy
	controller.open_panel("fac_england")
	await _wait(10)
	var panel: Control = controller.panel
	var sim: Object = panel.get("sim")
	var entry: Dictionary = panel.call("_entry", "fac_england")
	check(not entry.is_empty() and str(entry.get("attitude_band", "")) != "", "attitude_band missing from get_diplomacy")
	var head := panel.find_child("Head", true, false) as Control
	var texts := PackedStringArray()
	for label in head.find_children("*", "Label", true, false):
		texts.append((label as Label).text)
	check(" ".join(texts).find(str(entry["attitude_band"])) >= 0, "head should show the attitude label")
	# 1. Noms cliquables : un clic sur un lien ouvre la fiche de cette faction.
	var links := head.find_children("Link_*", "Button", true, false)
	if check(not links.is_empty(), "england's sheet should link allies/enemies/vassals"):
		var target := (links[0] as Button).name.trim_prefix("Link_")
		(links[0] as Button).pressed.emit()
		await _wait(4)
		check(str(panel.get("_selected")) == target, "clicking a name should select %s, got %s" % [target, panel.get("_selected")])
	# 2. Aperçu de guerre : trouver une cible avec des alliés non en guerre contre nous.
	var found := false
	for e in sim.call("get_diplomacy", map.player_faction):
		if str(e["status"]) == "war":
			continue
		var verdict: Dictionary = sim.call("evaluate_proposal", {"type": "declare_war", "target": str(e["id"])})
		var allies: Array = verdict.get("allies", [])
		if allies.is_empty():
			continue
		found = true
		var first: Dictionary = allies[0]
		check(["joins", "hesitates", "refuses"].has(str(first["answer"])), "unknown forecast answer %s" % first["answer"])
		var lines := PackedStringArray()
		for reason in verdict["reasons"]:
			lines.append(str(reason["text"]))
		check(" | ".join(lines).find("Prestige du souverain") >= 0, "war preview should state the prestige cost")
		check(" | ".join(lines).find(str(first["text"])) >= 0, "forecast line missing from the reasons")
		break
	check(found, "no target with allies found for the war preview")
	# 3. Retrait d'accès : bouton présent seulement si l'accès est accordé.
	var actions := panel.find_child("Actions", true, false) as DiplomacyActionsSection
	var fake: Dictionary = (panel.call("_entry", "fac_england") as Dictionary).duplicate()
	fake["access_given"] = false
	actions.show_for(sim, map.player_faction, "fac_england", fake)
	check(_button_texts(actions).find("Retirer l'accès militaire") < 0, "no revoke button without access")
	fake["access_given"] = true
	actions.show_for(sim, map.player_faction, "fac_england", fake)
	check(_button_texts(actions).find("Retirer l'accès militaire") >= 0, "revoke button expected when access is given")
	# 4. Historique : un pacte rompu porte son libellé de rupture.
	check(DiplomacyHistoryTab.RUPTURE_LABELS.has("perjury") and DiplomacyHistoryTab.RUPTURE_LABELS.has("broken"), "rupture labels")
	map.queue_free()
	await process_frame


func _button_texts(node: Node) -> PackedStringArray:
	var texts := PackedStringArray()
	for button in node.find_children("*", "Button", true, false):
		texts.append((button as Button).text)
	return texts
