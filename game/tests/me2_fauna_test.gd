extends SceneTree

## Test headless du lot DN-ME2 (troupeaux et faune, `FaunaLayer`) :
##  1. données `data/map/map_fauna.json` : espèces ⊂ catalogue, zones et allures résolues ;
##  2. projection lon/lat → EPSG:3035 (Paris) ;
##  3. semis : chevaux et taureaux en Camargue, rennes au nord, dromadaires au sud, aucun troupeau
##     en mer, près d'une colonie ou dans un fleuve, transhumance (profils de saison) ;
##  4. rendu : cellules à d = 8 et 60, un MultiMesh par espèce et par cellule (appels de dessin
##     bornés), rien au-delà de `max_distance` (vue parchemin) ; temps de construction imprimés.
## Usage : godot --headless --path game --script res://tests/me2_fauna_test.gd

const CAMARGUE := Vector2(0.0, 0.0)

var _failures := 0


func _init() -> void:
	await process_frame
	_test_data()
	await _test_map()
	if _failures > 0:
		push_error("me2_fauna_test: %d failure(s)" % _failures)
		quit(1)
		return
	print("me2_fauna_test: OK")
	quit(0)


func _check(condition: bool, label: String) -> bool:
	if not condition:
		_failures += 1
		push_error("FAIL: " + label)
	return condition


func _test_data() -> void:
	var config := FaunaLayer.load_config()
	_check(not config.is_empty(), "map_fauna.json loads")
	var species: Dictionary = config.get("species", {})
	_check(species.size() >= 25, "species table (%d)" % species.size())
	var gaits: Dictionary = (config.get("render", {}) as Dictionary).get("gaits", {})
	for id: String in species:
		_check(id.begins_with("animal_"), "%s is a catalogue animal id" % id)
		_check(gaits.has(str((species[id] as Dictionary)["gait"])), "%s gait exists" % id)
	for zone: Dictionary in config.get("zones", []):
		for item: Dictionary in zone["species"]:
			_check(species.has(str(item["species"])), "zone %s species %s" % [zone["id"], item["species"]])
	var paris := FaunaLayer.laea_3035(2.35, 48.85)
	_check(absf(paris.x - 3760536.8) < 10.0 and absf(paris.y - 2888771.0) < 10.0, "LAEA Paris %s" % paris)


func _ring_land(data: MapData, p: Vector2) -> bool:
	return data.is_land_px(int(p.x), int(p.y))


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
	var fauna: FaunaLayer = life.fauna
	if not _check(fauna != null, "Fauna node under CampaignLife"):
		map.queue_free()
		return
	_check(fauna.zone_count() >= 40, "zones (%d)" % fauna.zone_count())
	# Semis sur toute la carte.
	var t0 := Time.get_ticks_msec()
	var totals := {}
	var herds_all := 0
	var bad_sea := 0
	var in_river := 0
	var side := 96
	var ranks_ok := true
	var profiles := {}
	for cy in range(0, ceili(data.size.y / float(side))):
		for cx in range(0, ceili(data.size.x / float(side))):
			for herd: Dictionary in fauna.cell_herds(Vector2i(cx, cy)):
				herds_all += 1
				var at: Vector2 = herd["center"]
				totals[herd["species"]] = int(totals.get(herd["species"], 0)) + 1
				bad_sea += 0 if _ring_land(data, at) or str(herd["species"]) in ["animal_seal_grey", "animal_walrus"] else 1
				in_river += 1 if data.river_sd_at(at.x, at.y) < 0.3 else 0
				ranks_ok = ranks_ok and float(herd["rank"]) >= 0.0 and float(herd["rank"]) < 1.0
				profiles[int(herd["profile"])] = true
	print("me2: %d herds on the map in %d ms; %s" % [herds_all, Time.get_ticks_msec() - t0, totals])
	_check(herds_all > 300, "enough herds (%d)" % herds_all)
	_check(bad_sea == 0, "no herd on water (%d)" % bad_sea)
	_check(in_river == 0, "no herd in a river bed (%d)" % in_river)
	_check(ranks_ok, "herd ranks in [0, 1)")
	_check(profiles.size() >= 3, "season profiles for transhumance (%d)" % profiles.size())
	# Régions : Camargue, nord, sud.
	var camargue := FaunaLayer.lonlat_to_px(4.55, 43.52, data)
	var horses := _herds_near(fauna, camargue, 40.0, "animal_camargue_horse")
	var bulls := _herds_near(fauna, camargue, 40.0, "animal_camargue_bull")
	_check(horses.size() >= 2, "Camargue horses (%d)" % horses.size())
	_check(bulls.size() >= 2, "Camargue bulls (%d)" % bulls.size())
	var north := FaunaLayer.lonlat_to_px(19.0, 64.5, data)
	_check(_herds_near(fauna, north, 150.0, "animal_reindeer").size() >= 3, "reindeer in the far north")
	_check(_herds_near(fauna, camargue, 200.0, "animal_reindeer").is_empty(), "no reindeer in Provence")
	var south := FaunaLayer.lonlat_to_px(40.0, 33.5, data)
	_check(_herds_near(fauna, south, 250.0, "animal_camel_dromedary").size() >= 2, "dromedaries in the Levant")
	# Rendu.
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	await _settle(rig, camargue, 8.0, data, fauna)
	print("me2: d=8 Camargue: %s" % [fauna.stats])
	_check(int(fauna.stats["visible_cells"]) >= 1 and int(fauna.stats["visible"]) > 0, "animals drawn near the Camargue")
	_check(int(fauna.stats["draw_calls"]) <= 25, "draw calls bounded (%d)" % int(fauna.stats["draw_calls"]))
	await _settle(rig, camargue, 60.0, data, fauna)
	print("me2: d=60 Camargue: %s" % [fauna.stats])
	_check(int(fauna.stats["draw_calls"]) <= 60, "draw calls bounded at 60 (%d)" % int(fauna.stats["draw_calls"]))
	await _settle(rig, camargue, 1500.0, data, fauna)
	_check(not fauna.visible, "nothing on the parchment view")
	map.queue_free()


func _herds_near(fauna: FaunaLayer, center: Vector2, radius: float, species: String) -> Array:
	var side := 96.0
	var found: Array = []
	for cy in range(floori((center.y - radius) / side), floori((center.y + radius) / side) + 1):
		for cx in range(floori((center.x - radius) / side), floori((center.x + radius) / side) + 1):
			for herd: Dictionary in fauna.cell_herds(Vector2i(cx, cy)):
				if herd["species"] == species and (herd["center"] as Vector2).distance_to(center) <= radius:
					found.append(herd)
	return found


func _settle(rig: CampaignCamera, at: Vector2, distance: float, data: MapData, _fauna: FaunaLayer) -> void:
	rig.look_at_point(Vector3(at.x, data.surface_world_at(at.x, at.y), at.y), distance)
	rig.snap()
	for i in 40:
		await process_frame
