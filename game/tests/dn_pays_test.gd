extends SceneTree

## Test headless du lot DN-PAYS (campagne vivante hors champs, `CountrysideLayer`) :
##  1. données `data/map/map_countryside.json` : modèles, régions et règles résolus ;
##  2. semis sur toute la carte : aucun objet en mer, dans un fleuve ou sur un lac, règles actives
##     (bocage, puits, piloris, croix, moulins, salines, ruines, charrettes, caravanes) et
##     localisées (bocage à l'Ouest, caravanes au Sud et à l'Est, salines sur les côtes) ;
##  3. rendu : appels de dessin bornés, rien au-delà de la portée (vue parchemin), mesures imprimées.
## Usage : godot --headless --path game --script res://tests/dn_pays_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	_test_data()
	await _test_map()
	if _failures > 0:
		push_error("dn_pays_test: %d failure(s)" % _failures)
		quit(1)
		return
	print("dn_pays_test: OK")
	quit(0)


func _check(condition: bool, label: String) -> bool:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
	return condition


func _test_data() -> void:
	var config := CountrysideLayer.load_config()
	_check(not config.is_empty(), "map_countryside.json loads")
	var props: Dictionary = config.get("props", {})
	var regions: Dictionary = config.get("regions", {})
	_check(props.size() >= 30, "props table (%d)" % props.size())
	for rule: Dictionary in config.get("rules", []):
		for prop_id: String in rule["props"]:
			_check(props.has(prop_id), "rule %s prop %s" % [rule["id"], prop_id])
		for region_id: String in rule.get("regions", []):
			_check(regions.has(region_id), "rule %s region %s" % [rule["id"], region_id])


func _test_map() -> void:
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not _check(map.get("load_ok") == true, "campaign map loads"):
		map.queue_free()
		return
	var data: MapData = map.map_data
	var life: CampaignLife = map.life
	var layer: CountrysideLayer = life.countryside
	if not _check(layer != null, "Countryside node under CampaignLife"):
		map.queue_free()
		return
	_check(layer.rule_count() >= 20, "rules resolved (%d)" % layer.rule_count())
	var t0 := Time.get_ticks_msec()
	var by_rule := {}
	var by_prop := {}
	var bad_sea := 0
	var in_river := 0
	var in_lake := 0
	var ranks_ok := true
	var total := 0
	var side := 96
	var cells := 0
	var slowest_ms := 0.0
	var all: Array = []
	for cy in range(0, ceili(data.size.y / float(side))):
		for cx in range(0, ceili(data.size.x / float(side))):
			var tc := Time.get_ticks_usec()
			var found := layer.cell_instances(Vector2i(cx, cy))
			slowest_ms = maxf(slowest_ms, (Time.get_ticks_usec() - tc) / 1000.0)
			cells += 1
			for inst: Dictionary in found:
				total += 1
				var at: Vector2 = inst["pos"]
				by_rule[inst["rule"]] = int(by_rule.get(inst["rule"], 0)) + 1
				by_prop[inst["prop"]] = int(by_prop.get(inst["prop"], 0)) + 1
				bad_sea += 0 if data.is_land_px(int(at.x), int(at.y)) else 1
				in_river += 1 if data.river_sd_at(at.x, at.y) < 0.3 else 0
				in_lake += 1 if life.fauna != null and life.fauna.in_lake(at) else 0
				ranks_ok = ranks_ok and float(inst["rank"]) >= 0.0 and float(inst["rank"]) < 1.0
				all.append(inst)
	print("pays: %d instances in %d cells, %d ms (slowest cell %.1f ms)" % [total, cells, Time.get_ticks_msec() - t0, slowest_ms])
	print("pays: by rule %s" % [by_rule])
	_check(total > 3000, "enough instances (%d)" % total)
	_check(bad_sea == 0, "nothing on the sea (%d)" % bad_sea)
	_check(in_river == 0, "nothing in a river bed (%d)" % in_river)
	_check(in_lake == 0, "nothing on a lake (%d)" % in_lake)
	_check(ranks_ok, "ranks in [0, 1)")
	for rule_id in ["bocage_enclos", "puits_ouest", "piloris", "croix_de_chemin", "calvaires_bretons", "moulins_a_vent_nord", "moulins_polder", "moulins_sud", "salines_atlantique", "salines_mediterranee", "ruines_villages", "charrettes_bord_de_route", "carretas", "caravanes_chameaux", "arbas_steppe", "cabanes_de_bergers"]:
		_check(int(by_rule.get(rule_id, 0)) > 0, "rule %s places something (%d)" % [rule_id, int(by_rule.get(rule_id, 0))])
	# Localisation.
	var west := FaunaLayer.lonlat_to_px(-1.0, 47.8, data)
	var bocage_west := 0
	var bocage_far := 0
	var caravans_south := 0
	var caravans_north := 0
	var haywain_profiles := {}
	for inst: Dictionary in all:
		var at: Vector2 = inst["pos"]
		if inst["rule"] == "bocage_enclos":
			if at.distance_to(west) < 500.0:
				bocage_west += 1
			if at.x > west.x + 1800.0:
				bocage_far += 1
		if inst["rule"] == "caravanes_chameaux":
			if FaunaLayer.lonlat_to_px(20.0, 45.0, data).y < at.y:
				caravans_south += 1
			else:
				caravans_north += 1
		if inst["rule"] == "fenaison":
			haywain_profiles[int(inst["profile"])] = true
	_check(bocage_west > 100 and bocage_far == 0, "bocage only in the west (%d west, %d far)" % [bocage_west, bocage_far])
	_check(caravans_south > 0 and caravans_north == 0, "camel caravans only south of 45N (%d, %d)" % [caravans_south, caravans_north])
	_check(not haywain_profiles.is_empty(), "haywains exist")
	# Déterminisme.
	var again := layer.cell_instances(Vector2i(floori(west.x / side), floori(west.y / side)))
	var first := layer.cell_instances(Vector2i(floori(west.x / side), floori(west.y / side)))
	_check(again.size() == first.size() and again.size() > 0, "deterministic cell (%d)" % again.size())
	# Rendu.
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	var guerande := FaunaLayer.lonlat_to_px(-2.45, 47.3, data)
	for view: Array in [[west, 12.0], [west, 30.0], [guerande, 14.0], [FaunaLayer.lonlat_to_px(4.3, 52.0, data), 40.0]]:
		await _settle(rig, view[0], view[1], data)
		print("pays: d=%.0f at %s: %s" % [view[1], view[0], layer.stats])
		_check(int(layer.stats["draw_calls"]) <= 100, "draw calls bounded at %.0f (%d)" % [view[1], int(layer.stats["draw_calls"])])
	_check(int(layer.stats["visible"]) > 0, "something drawn")
	await _settle(rig, west, 1500.0, data)
	_check(not layer.visible, "nothing on the parchment view")
	map.queue_free()


func _settle(rig: CampaignCamera, at: Vector2, distance: float, data: MapData) -> void:
	rig.look_at_point(Vector3(at.x, data.surface_world_at(at.x, at.y), at.y), distance)
	rig.snap()
	for i in 40:
		await process_frame
