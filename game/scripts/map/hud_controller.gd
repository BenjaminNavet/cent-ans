class_name HudController
extends Node

## HUD « à la Total War » branché sur la carte (lot F10b) : bandeau d'ost + sceau du chef pour
## l'armée sélectionnée, cloche de fin de saison et ses alertes, lettres scellées. Les
## composants vivent dans `MapUI` (nœuds `ArmyStrip`, `GeneralSeal`, `EndTurnCluster`,
## `NewsLetters`) ; ce contrôleur les alimente depuis la simulation et relie leurs signaux.
## `campaign_map.gd` n'appelle que `setup`, `show_army`, `refresh` et `after_end_turn`.
## Aucune règle de jeu : lecture de l'état et envoi des ordres existants.

var map: Node = null  # CampaignMap
var ui: MapUI = null
## Événements du dernier tour (alertes « construction achevée »).
var last_events: Array = []


func setup(campaign_map: Node) -> void:
	map = campaign_map
	ui = map.get("ui")
	ui.alert_activated.connect(_on_alert_activated)
	ui.news_activated.connect(_on_news_activated)
	ui.army_general_clicked.connect(_on_general_clicked)
	ui.army_split_requested.connect(_on_split_requested)
	ui.load_requested.connect(func(_path: String) -> void:
		last_events = []
		refresh())
	refresh()


func _sim() -> Object:
	return map.get("sim")


## Vrai si la simulation accepte l'ordre `split_army` (aucune ne l'expose encore : le bouton
## « Séparer » reste masqué).
func split_supported() -> bool:
	var sim := _sim()
	return sim != null and sim.has_method("supports_order") and bool(sim.call("supports_order", "split_army"))


## Remplit le bandeau et le sceau pour l'armée `army_id` (déjà sélectionnée par la carte).
func show_army(army_id: String, army: Dictionary, is_player: bool) -> void:
	var sim := _sim()
	if sim == null or army.is_empty():
		ui.hide_army()
		return
	var faction := str(army.get("faction", ""))
	var general_id := str(army.get("general", ""))
	var character: Dictionary = {}
	if general_id != "" and sim.has_method("get_character"):
		character = sim.call("get_character", general_id)
	var faction_name := SimFacade.faction_short_name(faction)
	var general_name := str(army.get("general_name", character.get("name", "")))
	var title := "Ost de %s" % (general_name if general_name != "" else faction_name)
	if not is_player and general_name != "":
		title += " (%s)" % faction_name
	ui.show_army(army_id, army, character, faction, is_player, title, army_status(army, is_player), is_player and split_supported())


## Position et ordre en cours de l'armée (étiquette au-dessus du bandeau).
func army_status(army: Dictionary, is_player: bool) -> String:
	var where := str(map.call("province_name_of", str(army.get("location", ""))))
	var text := "Ost à %s" % where
	var path: Array = army.get("path", [])
	if not path.is_empty():
		text += " — en marche vers %s" % map.call("province_name_of", str(path[-1]))
		if path.size() > 1:
			text += " (%d étapes)" % path.size()
	if is_player:
		text += " — clic droit sur une province pour marcher"
	return text


## Après tout changement d'état : date de la cloche et alertes.
func refresh() -> void:
	var sim := _sim()
	if sim == null or ui == null:
		return
	ui.end_turn_cluster.set_date(str(sim.call("get_date_label")), int(sim.call("get_turn")))
	ui.end_turn_cluster.set_alerts(CampaignAlerts.collect(map, last_events))


## Après `end_turn` : événements du tour, puis rafraîchissement des alertes.
func after_end_turn(events: Array) -> void:
	last_events = events
	refresh()


# --- Signaux du HUD ------------------------------------------------------------------


func _flow() -> Node:
	return map.get("flow")


func _on_alert_activated(alert: Dictionary) -> void:
	var flow := _flow()
	match str(alert.get("kind", "")):
		"chronicle_decision":
			var chronicle: Node = map.get("chronicle")
			if chronicle != null:
				chronicle.call("open_window")
			return
		"debt":
			ui.faction_panel_requested.emit()
			return
		"research_idle":
			ui.tech_panel_requested.emit()
			return
	var character_id := str(alert.get("character_id", ""))
	var army_id := str(alert.get("army_id", ""))
	var province_id := str(alert.get("province_id", ""))
	if character_id != "":
		map.call("_on_character_selected", character_id)
	elif army_id != "" and flow != null:
		flow.call("focus_army", army_id)
	elif province_id != "" and flow != null:
		flow.call("focus_province", province_id)


func _on_news_activated(item: Dictionary) -> void:
	var province_id := str(item.get("province_id", ""))
	var faction_id := str(item.get("faction_id", ""))
	var flow := _flow()
	if province_id != "" and flow != null:
		flow.call("focus_province", province_id)
	elif faction_id != "" and faction_id != str(map.get("player_faction")):
		var diplomacy: Node = map.get("diplomacy")
		if diplomacy != null and diplomacy.call("available"):
			diplomacy.call("open_panel", faction_id)


## Sceau : fiche du chef, ou cour filtrée sur les chefs pour une armée sans chef.
func _on_general_clicked(character_id: String) -> void:
	if character_id != "":
		map.call("_on_character_selected", character_id)
	elif str(ui.current_army_id) != "" and bool(map.call("_characters_available")):
		map.set("_court_open", true)
		map.call("_show_court_panel", CourtPanel.FILTER_GENERAL)


func _on_split_requested(army_id: String, unit_indices: Array) -> void:
	if not split_supported():
		return
	map.call("_submit", {"type": "split_army", "army": army_id, "unit_indices": unit_indices}, "Armée séparée.")
