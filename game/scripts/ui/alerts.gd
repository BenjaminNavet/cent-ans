class_name CampaignAlerts
extends RefCounted

## F3 / F10b — alertes persistantes de la carte, affichées par la cloche de fin de saison
## (`EndTurnCluster`, bas droite) : armée ennemie aux frontières, province assiégée, dette,
## recherche inactive, bâtiment terminé, décision de chronique en attente (bloquante). Chaque
## alerte reste tant que sa condition tient (recalculée à chaque rafraîchissement de la carte).
## Les conditions ne font que lire l'état exposé par la simulation. (L'ancienne colonne
## d'alertes à droite de F3 a été retirée : une seule présentation, la cloche.)


## Alertes du joueur d'après l'état de la carte (`CampaignMap`) et les événements du dernier
## tour. Chaque alerte, au format de `EndTurnCluster.set_alerts` : `{id, kind, text, tooltip,
## severity, province_id?, army_id?, blocking?}` ; types `siege`, `enemy_army`, `debt`,
## `research_idle`, `chronicle_decision` (bloquante), `construction_done`.
static func collect(map: Node, last_events: Array) -> Array:
	var result: Array = []
	var sim: Object = map.get("sim")
	var player := str(map.get("player_faction"))
	var map_data: MapData = map.get("map_data")
	if sim == null or player == "" or map_data == null:
		return result
	var summary: Dictionary = sim.call("get_faction_summary", player)
	var enemies: PackedStringArray = summary.get("at_war_with", PackedStringArray())
	var owned: Dictionary = {}  # province id → true (possédée et tenue par le joueur)
	for index in range(1, map_data.province_count + 1):
		var province_id := str(map_data.get_province(index).get("id", ""))
		var state: Dictionary = sim.call("get_province_state", province_id)
		if str(state.get("owner", "")) != player:
			continue
		owned[province_id] = true
		var siege: Dictionary = state.get("siege", {})
		if not siege.is_empty() and str(siege.get("attacker", "")) != player:
			result.append({
				"id": "siege:" + province_id, "kind": "siege", "severity": "danger",
				"province_id": province_id,
				"text": "%s assiégée (vivres : %d)" % [map.call("province_name_of", province_id), int(siege.get("supplies", 0))],
				"tooltip": "Siège mené par %s depuis %d tour(s)." % [_faction_name(str(siege.get("attacker", ""))), int(siege.get("turns_elapsed", 0))],
			})
	# Armées ennemies dans une province du joueur ou adjacente.
	for army_id in sim.call("get_army_ids"):
		var army: Dictionary = sim.call("get_army", army_id)
		var faction := str(army.get("faction", ""))
		if not (faction in enemies):
			continue
		var location := str(army.get("location_province", army.get("location", "")))
		var threatened := location if owned.has(location) else ""
		if threatened == "":
			for neighbor in map_data.get_province(map_data.index_of_id(location)).get("neighbors", []):
				if owned.has(str(neighbor)):
					threatened = str(neighbor)
					break
		if threatened == "":
			continue
		var strength := 0
		for unit in army.get("units", []):
			if unit is Dictionary:
				strength += int(unit.get("strength", 0))
		var where := "en %s" % map.call("province_name_of", location) if threatened == location else "aux portes de %s" % map.call("province_name_of", threatened)
		result.append({
			"id": "army:" + str(army_id), "kind": "enemy_army", "severity": "danger",
			"army_id": str(army_id), "province_id": location,
			"text": "Armée ennemie %s (%s)" % [where, _thousands(strength)],
			"tooltip": "Armée de %s, %s hommes." % [_faction_name(faction), _thousands(strength)],
		})
	if int(summary.get("treasury", 0)) < 0:
		result.append({
			"id": "debt", "kind": "debt", "severity": "danger",
			"text": "Trésor endetté : %s ℔" % _thousands(int(summary.get("treasury", 0))),
			"tooltip": "En dette, les troupes perdent du moral : licenciez ou relevez l'impôt.",
		})
	if sim.has_method("get_research") and sim.has_method("get_tech_tree"):
		var research: Dictionary = sim.call("get_research", player)
		if research.is_empty() or str(research.get("technology", "")) == "":
			var available := false
			for node in sim.call("get_tech_tree", player):
				if str(node.get("state", "")) == "available":
					available = true
					break
			if available:
				result.append({
					"id": "research", "kind": "research_idle", "severity": "warning",
					"text": "Aucune recherche en cours",
					"tooltip": "Ouvrir les technologies (T).",
				})
	if sim.has_method("get_pending_decisions"):
		for decision in sim.call("get_pending_decisions"):
			result.append({
				"id": "chronicle:%s" % str(decision.get("id", "")), "kind": "chronicle_decision", "severity": "warning",
				"blocking": true, "decision_id": decision.get("id", -1),
				"province_id": str(decision.get("province", "")),
				"text": "Chronique : %s" % str(decision.get("title", "décision en attente")),
			})
	for event in last_events:
		if not (event is Dictionary) or str(event.get("kind", "")) != "building_completed":
			continue
		var province_id := str(event.get("province", ""))
		if province_id == "" or not owned.has(province_id):
			continue
		result.append({
			"id": "building:%s:%d" % [province_id, result.size()], "kind": "construction_done", "severity": "info",
			"province_id": province_id, "text": str(event.get("text_fr", "Bâtiment terminé")),
		})
	result.append_array(table_medicine_alerts(sim, player, last_events))
	result.append_array(ransom_alerts(sim))
	return result


## H11 : un captif du joueur (rançon à payer) et une échéance de rançon due cette saison ou la
## suivante (`get_ransoms`). Payer : panneau de faction → Captifs et rançons.
static func ransom_alerts(sim: Object) -> Array:
	var result: Array = []
	if sim == null or not sim.has_method("get_ransoms"):
		return result
	var ransoms: Dictionary = sim.call("get_ransoms")
	var style: Dictionary = SeasonReport.KIND_STYLES["ransom"]
	for captive in ransoms.get("ours", []):
		result.append({
			"id": "captive:%s" % str(captive.get("character", "")), "kind": "ransom", "glyph": style["glyph"],
			"severity": "warning", "character_id": str(captive.get("character", "")),
			"text": "%s est captif de %s" % [str(captive.get("name", "")), _faction_name(str(captive.get("captor", "")))],
			"tooltip": "Rançon : %s ℔. Payer depuis le panneau de faction → Captifs et rançons." % _thousands(int(captive.get("ransom", 0))),
		})
	var turn := int(sim.call("get_turn")) if sim.has_method("get_turn") else 0
	for debt in ransoms.get("debts", []):
		if int(debt.get("next_due_turn", 0)) - turn > 1:
			continue
		result.append({
			"id": "ransom_due:%s" % str(debt.get("character", "")), "kind": "ransom", "glyph": style["glyph"],
			"severity": "danger" if int(debt.get("missed", 0)) > 0 else "warning",
			"text": "Échéance de rançon : %s ℔ dus à %s" % [_thousands(int(debt.get("installment", 0))), _faction_name(str(debt.get("creditor", "")))],
			"tooltip": "Rançon de %s, reste %s ℔. Sans trésor suffisant, la dette grossit de 10 %%." % [str(debt.get("name", "")), _thousands(int(debt.get("remaining", 0)))],
		})
	return result


## H9 : retours au défaut de la Table, Carême, blessés soignés, épidémies contenues (genres
## `table` / `medicine` du dernier tour), et plantes entrées dans l'herbier. L'herbier n'est
## synchronisé qu'une fois par lot d'événements (les rafraîchissements suivants réutilisent
## le message, qui reste affiché jusqu'au tour suivant).
static var _herb_events_key: int = 0
static var _herb_alert: Dictionary = {}


static func table_medicine_alerts(sim: Object, player: String, last_events: Array) -> Array:
	var result: Array = []
	var researched := false
	for event in last_events:
		if not (event is Dictionary):
			continue
		var kind := str(event.get("kind", ""))
		var faction := str(event.get("faction", ""))
		if kind == "technology_researched" and faction == player:
			researched = true
		if not SeasonReport.KIND_STYLES.has(kind) or (faction != "" and faction != player):
			continue
		var style: Dictionary = SeasonReport.KIND_STYLES[kind]
		result.append({
			"id": "%s:%d" % [kind, result.size()], "kind": kind, "glyph": style["glyph"],
			"severity": "warning" if kind == "table" else "info",
			"province": str(event.get("province", "")), "province_id": str(event.get("province", "")),
			"text": str(event.get("text_fr", style["label"])),
		})
	var key := last_events.hash()
	if key != _herb_events_key:
		_herb_events_key = key
		_herb_alert = {}
		if researched:
			var herbs := Herbarium.sync(sim, player)
			if not herbs.is_empty():
				_herb_alert = {
					"id": "herbarium", "kind": "herbarium", "glyph": "❦", "severity": "info",
					"text": Herbarium.message(herbs), "codex": str(herbs[0]),
					"tooltip": "Fiches ajoutées au Codex (touche K, onglet Médecine et herbier).",
				}
	if not _herb_alert.is_empty():
		result.append(_herb_alert)
	return result


static func _faction_name(faction_id: String) -> String:
	var tree := Engine.get_main_loop() as SceneTree
	var facade: Node = tree.root.get_node_or_null("/root/SimFacade") if tree != null else null
	return str(facade.call("faction_short_name", faction_id)) if facade != null else faction_id


static func _thousands(value: int) -> String:
	var digits := str(absi(value))
	var groups := PackedStringArray()
	while digits.length() > 3:
		groups.insert(0, digits.substr(digits.length() - 3))
		digits = digits.substr(0, digits.length() - 3)
	groups.insert(0, digits)
	return ("-" if value < 0 else "") + " ".join(groups)
