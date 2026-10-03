class_name HudController
extends Node

## HUD de campagne branché sur la carte (lot F10b) : bandeau d'ost + sceau du chef pour
## l'armée sélectionnée, cloche de fin de saison et ses alertes, lettres scellées. Les
## composants vivent dans `MapUI` (nœuds `ArmyStrip`, `GeneralSeal`, `EndTurnCluster`,
## `NewsLetters`) ; ce contrôleur les alimente depuis la simulation et relie leurs signaux.
## `campaign_map.gd` n'appelle que `setup`, `show_army`, `refresh` et `after_end_turn`.
## Aucune règle de jeu : lecture de l'état et envoi des ordres existants.

var map: Node = null  # CampaignMap
var ui: MapUI = null
## Événements du dernier tour (alertes « construction achevée »).
var last_events: Array = []
## TW2-T3 : panneau « Mercenaires » (créé à la première ouverture).
var mercenary_panel: MercenaryPanel = null


func setup(campaign_map: Node) -> void:
	map = campaign_map
	ui = map.get("ui")
	ui.alert_activated.connect(_on_alert_activated)
	ui.news_activated.connect(_on_news_activated)
	ui.army_general_clicked.connect(_on_general_clicked)
	ui.army_split_requested.connect(_on_split_requested)
	ui.army_garrison_requested.connect(_on_garrison_requested)
	ui.army_mercenaries_requested.connect(show_mercenaries)
	ui.load_requested.connect(func(_path: String) -> void:
		last_events = []
		update_interest()
		refresh())
	var settings := map.get_node_or_null("/root/Settings")
	if settings != null:
		settings.changed.connect(func(key: String) -> void:
			if key == "interface/news_filter":
				update_interest())
	update_interest()
	refresh()


## Lot U5 : instantané des voisins, alliés, ennemis et grandes puissances du joueur (filtre des
## lettres et du bandeau), à refaire après chaque fin de tour (`CampaignMap._on_end_turn`).
func update_interest() -> void:
	var sim := _sim()
	if sim == null or ui == null:
		return
	var settings := map.get_node_or_null("/root/Settings")
	var mode := str(settings.call("get_value", "interface/news_filter")) if settings != null else NewsInterest.MODE_INTEREST
	ui.news_interest = NewsInterest.build(sim, map.get("map_data"), str(map.get("player_faction")), mode)


func _sim() -> Object:
	return map.get("sim")


## Vrai si la simulation accepte l'ordre `split_army` (aucune ne l'expose encore : le bouton
## « Séparer » reste masqué).
func split_supported() -> bool:
	var sim := _sim()
	return sim != null and sim.has_method("supports_order") and bool(sim.call("supports_order", "split_army"))


## Lot C7d : disponibilité du bouton « Garnison » pour `army` (résultat de `get_army`).
## `{can_garrison, reason}` : `can_garrison` vrai seulement si l'armée est au joueur, sur une
## colonie qu'il contrôle ; `reason` (français, vide = actif) désactive le bouton si la
## colonie est assiégée ou si sa garnison est pleine. Aucune règle ici : tout vient de
## `settlement_detail` (`garrison_cap` / `garrison_free`, lot C7d).
func garrison_availability(army: Dictionary, is_player: bool) -> Dictionary:
	var sim := _sim()
	var location := str(army.get("location", ""))
	if not is_player or location == "" or sim == null or not sim.has_method("settlement_detail"):
		return {"can_garrison": false, "reason": ""}
	var detail: Dictionary = sim.call("settlement_detail", location)
	if detail.is_empty() or str(detail.get("controller", "")) != str(map.get("player_faction")):
		return {"can_garrison": false, "reason": ""}
	if not (detail.get("siege", {}) as Dictionary).is_empty():
		return {"can_garrison": true, "reason": "Colonie assiégée : impossible d'y laisser une garnison."}
	var free := int(detail.get("garrison_free", -1))
	if free == 0:
		return {"can_garrison": true, "reason": "Garnison complète (%s au maximum)." % FrText.count(int(detail.get("garrison_cap", 0)), "unité")}
	return {"can_garrison": true, "reason": ""}


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
	var traditions: Node = map.get("traditions")
	if traditions != null:  # TW2-T5 : nom gardé par une armée aguerrie quand le chef change
		var kept := str(traditions.call("kept_title", army_id))
		if kept != "":
			title = kept
	if not is_player and general_name != "":
		title += " (%s)" % faction_name
	var vassal_text := str(map.call("army_hover_text", army_id))  # M9 : « (vassal de Y) »
	if vassal_text.contains("(vassal de "):
		title += " — " + vassal_text.substr(vassal_text.find("(vassal de "))
	var garrison := garrison_availability(army, is_player)
	if is_player and sim.has_method("get_stance_options"):  # CV3-4 : refus des postures (cœur)
		army = army.duplicate()
		army["stance_options"] = sim.call("get_stance_options", army_id)
	if is_player and sim.has_method("get_army_replenishment"):  # TW2-T2 : reconstitution (cœur)
		army = army.duplicate()
		army["replenishment"] = sim.call("get_army_replenishment", army_id)
	ui.show_army(army_id, army, character, faction, is_player, title, army_status(army, is_player),
		is_player and split_supported(), bool(garrison["can_garrison"]), str(garrison["reason"]))
	if traditions != null:  # TW2-T5 : bouton « Traditions » du bandeau
		traditions.call("on_army_shown", army_id, is_player)


## Position et ordre en cours de l'armée (étiquette au-dessus du bandeau).
func army_status(army: Dictionary, is_player: bool) -> String:
	var where := str(map.call("province_name_of", str(army.get("location_province", army.get("location", "")))))
	var text := "Ost à %s" % where
	var path: Array = army.get("path_provinces", army.get("path", []))
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
		"diplomacy_offer":  # Q5
			var diplomacy: Node = map.get("diplomacy")
			if diplomacy != null and diplomacy.call("available"):
				diplomacy.call("open_panel")
			return
		"ransom":  # H11 : panneau de faction puis fenêtre des captifs et rançons
			ui.faction_panel_requested.emit()
			if not (ui.faction_panel.ransom_panel != null and ui.faction_panel.ransom_panel.visible):
				ui.faction_panel.toggle_ransoms()
			return
		"herbarium":  # H9 : fiche de la plante dans le Codex
			var bubbles: Node = map.get_node_or_null("/root/CodexBubbles")
			if bubbles != null:
				bubbles.call("open_entry", str(alert.get("codex", "")))
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
		open_general_picker(ui.current_army_id)  # U10 : « Sans chef » → choix du général


## Lot U10 : choix du général de l'armée `army_id` (armée du joueur).
func open_general_picker(army_id: String) -> void:
	var army: Dictionary = _sim().call("get_army", army_id)
	if str(army.get("faction", "")) != str(map.get("player_faction")):
		return
	var place := str(map.call("province_name_of", str(army.get("location_province", army.get("location", "")))))
	ui.show_general_picker(army_id, "Choisir le chef de l'ost (%s)" % place, general_candidates(army))


## Lot U10 : personnages du joueur pour commander `army` : `[{id, name, detail, reason}]`, les
## disponibles sur place d'abord (`reason` vide), puis les autres avec leur empêchement.
func general_candidates(army: Dictionary) -> Array:
	var sim := _sim()
	var location := str(army.get("location_province", army.get("location", "")))
	var free: Array = []
	var busy: Array = []
	for id in sim.call("get_faction_characters", str(map.get("player_faction"))):
		var character: Dictionary = sim.call("get_character", id)
		if character.is_empty() or not bool(character.get("alive", true)):
			continue
		var command := int((character.get("skills", {}) as Dictionary).get("command", 0))
		var entry := {"id": str(id), "name": str(character.get("name", "?")), "detail": "commandement %d" % command, "command": command}
		var female := str(character.get("sex", "")) == "female"
		if bool(character.get("captive", false)):
			entry["reason"] = "retenue captive" if female else "retenu captif"
		elif str(character.get("army", "")) != "":
			entry["reason"] = "commande déjà une armée"
		elif str(character.get("governor_of", "")) != "":
			entry["reason"] = "gouverne une province"
		elif str(character.get("location", "")) != location:
			entry["reason"] = "ailleurs : %s" % str(map.call("province_name_of", str(character.get("location", ""))))
		if str(entry.get("reason", "")) == "":
			free.append(entry)
		else:
			busy.append(entry)
	free.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return int(a["command"]) > int(b["command"]))
	return free + busy.slice(0, 6)


func _on_split_requested(army_id: String, unit_indices: Array) -> void:
	if not split_supported():
		return
	map.call("_submit", {"type": "split_army", "army": army_id, "unit_indices": unit_indices}, "Armée séparée.")


## Lot C7d : bouton « Garnison ». Le refus éventuel du cœur (siège survenu entre-temps,
## garnison remplie par un autre ordre) revient dans le toast d'erreur de `_submit`.
func _on_garrison_requested(army_id: String, unit_indices: Array) -> void:
	map.call("_submit", {"type": "garrison_units", "army": army_id, "unit_indices": unit_indices}, "Régiment laissé en garnison." if unit_indices.size() <= 1 else "Régiments laissés en garnison.")


## TW2-T3 : ouvre (ou rafraîchit) le panneau « Mercenaires » de l'armée `army_id` : compagnies
## de sa région, prix, solde, réserves et refus, tels que le cœur les donne (`get_mercenaries`).
func show_mercenaries(army_id: String) -> void:
	var sim := _sim()
	if sim == null or not sim.has_method("get_mercenaries"):
		ui.show_toast("Mercenaires indisponibles avec cette simulation.", true)
		return
	var info: Dictionary = sim.call("get_mercenaries", army_id)
	if info.is_empty():
		return
	if mercenary_panel == null:
		mercenary_panel = MercenaryPanel.new()
		mercenary_panel.theme = ui.event_log.theme
		UiZones.put(UiZones.Zone.SIDE_PANEL, mercenary_panel)
		ui.register_panel(mercenary_panel, PanelStack.Kind.CENTRAL)
		mercenary_panel.hire_requested.connect(_on_hire_requested)
	mercenary_panel.show_market(army_id, info)
	mercenary_panel.show()


## Un clic sur une compagnie : ordre `hire_mercenary` (refus du cœur dans le toast d'erreur de
## `_submit`), puis le panneau se met à jour (réserve, engagements restants, trésor).
func _on_hire_requested(army_id: String, unit_type: String) -> void:
	map.call("_submit", {"type": "hire_mercenary", "army": army_id, "unit": unit_type}, "Compagnie engagée : elle rejoint l'ost.")
	if mercenary_panel != null and mercenary_panel.visible:
		show_mercenaries(army_id)
