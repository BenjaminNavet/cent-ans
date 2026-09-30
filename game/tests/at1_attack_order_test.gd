extends SceneTree

## Test headless du lot AT1 (attaque depuis la carte de campagne) sur la vraie simulation :
##  1. une place ennemie gardée est une cible d'attaque (consigne « donner l'assaut ») ;
##  2. clic droit dessus avec une armée à portée → siège puis assaut aussitôt (bataille de
##     siège en attente ou assaut résolu) ;
##  3. une place d'une faction en paix → confirmation de déclaration de guerre ; la confirmer
##     met les deux factions en guerre ;
##  4. image du curseur « épées croisées » construite.
## Usage : godot --headless --path game --script res://tests/at1_attack_order_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("at1_attack_order_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("at1_attack_order_test: " + message)
	return condition


func _run() -> void:
	if not _check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_reachable_area"),
			"CampaignSim.get_reachable_area missing (run core/build.sh)"):
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
	if not _check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var ctl: Node = map.movement_ctl
	var sim: Object = map.sim
	var armies: PackedStringArray = map.player_army_ids()
	if not _check(ctl != null and ctl.available() and not armies.is_empty(), "controller inactive or no French army"):
		map.queue_free()
		return

	_check(AttackCursor.image() != null and AttackCursor.image().get_width() == AttackCursor.SIZE, "the attack cursor image should build")

	# 1-2. Place ennemie gardée : assaut dès l'arrivée.
	var at_war: PackedStringArray = sim.call("get_faction_summary", "fac_france").get("at_war_with", PackedStringArray())
	var fortress := _settlement_of(map, sim, at_war, true)
	if _check(fortress != "", "no garrisoned enemy settlement without an army inside"):
		var army_id := armies[0]
		map.select_army(army_id)
		var target := {"kind": "settlement", "id": fortress, "point": map.settlement_data.get_settlement(fortress)["px"]}
		_check(ctl.is_attack_target(target), "an enemy settlement should be an attack target")
		_check(ctl.attack_hint(target).contains("assaut"), "hint should offer the assault: %s" % ctl.attack_hint(target))
		var spot := _spot_near(sim, army_id, target["point"], 6.0)
		_check(sim.call("debug_place_army", army_id, spot.x, spot.y), "debug_place_army failed")
		map.refresh_all()
		var pending_before: int = (sim.call("get_pending_battles") as Array).size()
		var report: Dictionary = ctl.order_target(army_id, target)
		print("at1: attack on %s → %s" % [fortress, report])
		if _check(report.get("ok", false), "attack on the settlement refused: %s" % report.get("error", "")):
			var pending: Array = sim.call("get_pending_battles")
			var siege_battle := false
			for battle in pending:
				siege_battle = siege_battle or bool(battle.get("siege", false))
			var taken: bool = str(map.settlement_data.get_settlement(fortress).get("controller", "")) == "fac_france"
			_check(siege_battle or taken,
				"reaching the settlement should lead to the assault (pending %d → %d)" % [pending_before, pending.size()])
		map.call("_close_battle_dialog")

	# 3. Faction en paix : confirmation, puis guerre.
	var peaceful := PackedStringArray()
	for entry in sim.call("get_diplomacy", "fac_france"):
		if str(entry.get("status", "")) in ["peace", "truce"]:
			peaceful.append(str(entry["id"]))
	var neutral := _settlement_of(map, sim, peaceful, false)
	if _check(neutral != "", "no settlement of a faction at peace"):
		var army_id := armies[0]
		map.select_army(army_id)
		var target := {"kind": "settlement", "id": neutral, "point": map.settlement_data.get_settlement(neutral)["px"]}
		var faction: String = ctl.target_faction(target)
		_check(ctl.relation_to(faction) == "peace", "relation with %s should be peace" % faction)
		_check(ctl.attack_hint(target).contains("déclare la guerre"), "hint should warn of the war: %s" % ctl.attack_hint(target))
		ctl.ask_war(army_id, target)
		_check(ctl.war_dialog.visible, "the war declaration should ask for confirmation")
		ctl.war_dialog.call("_on_confirm")
		_check(not ctl.war_dialog.visible, "the dialog should close once confirmed")
		var now_at_war: PackedStringArray = sim.call("get_faction_summary", "fac_france").get("at_war_with", PackedStringArray())
		_check(now_at_war.has(faction), "confirming should declare war on %s" % faction)
		_check(ctl.relation_to(faction) == "war", "the relation cache should follow the declaration")

	# 4. Vassal : attaquable après déclaration de guerre, qui rompt l'hommage.
	var vassal := ""
	for entry in sim.call("get_diplomacy", "fac_france"):
		if str(entry.get("status", "")) == "vassal":
			vassal = str(entry["id"])
			break
	if _check(vassal != "", "France should have a vassal"):
		map.select_army(armies[0])
		_check(ctl.relation_to(vassal) == "peace", "a vassal should be attackable after a declaration")
		var target := {"kind": "army", "id": "", "point": Vector2.ZERO, "faction": vassal}
		_check(ctl.is_attack_target(target), "a vassal army should be an attack target")
		ctl.ask_war(armies[0], target)
		_check(ctl.war_dialog.visible, "attacking a vassal should ask for confirmation")
		ctl.war_dialog.call("_on_confirm")
		var at_war_now: PackedStringArray = sim.call("get_faction_summary", "fac_france").get("at_war_with", PackedStringArray())
		_check(at_war_now.has(vassal), "confirming should declare war on the vassal %s" % vassal)
	map.queue_free()
	await process_frame


## Place tenue par une des `factions` : gardée et sans armée à l'intérieur si `garrisoned`.
func _settlement_of(map: Node, sim: Object, factions: PackedStringArray, garrisoned: bool) -> String:
	for entry in map.settlement_data.settlements:
		if not factions.has(str(entry.get("controller", ""))):
			continue
		var id := str(entry["id"])
		if not garrisoned:
			return id
		var detail: Dictionary = sim.call("settlement_detail", id)
		if str(detail.get("kind", "")) == "village" or (detail.get("garrison", []) as Array).is_empty():
			continue
		var occupied := false
		for army_id in sim.call("get_army_ids"):
			occupied = occupied or str(sim.call("get_army", army_id).get("settlement", "")) == id
		if not occupied:
			return id
	return ""


## Point atteignable à `distance` pixels de `from`.
func _spot_near(sim: Object, army_id: String, from: Vector2, distance: float) -> Vector2:
	for i in 16:
		var point := from + Vector2.RIGHT.rotated(TAU * float(i) / 16.0) * distance
		var plan: Dictionary = sim.call("find_path_points", army_id, point.x, point.y)
		if plan.get("ok", false):
			return point
	return from + Vector2(distance, 0.0)
