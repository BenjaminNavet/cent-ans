extends SceneTree

## Test headless du lot ME5 (atmosphère de la carte) : données, fondus, bancs de fleuve, rideaux de
## pluie, construction des familles sans nœud par instance.
## Usage : godot --headless --path game --script res://tests/me5_atmosphere_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	_run()
	print("me5_atmosphere_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("me5_atmosphere_test: " + message)
	return condition


func _run() -> void:
	var data := MapAtmosphere.data()
	if not _check(bool(data.get("enabled", false)), "data/fx/map_atmosphere.json absent ou éteint"):
		return
	# Fondus : bornes et monotonie.
	var in_band := Vector2(100, 200)
	var out_band := Vector2(500, 600)
	_check(MapAtmosphere.band_fade(50, in_band, out_band) == 0.0, "fondu nul avant l'entrée")
	_check(MapAtmosphere.band_fade(350, in_band, out_band) == 1.0, "fondu plein au milieu")
	_check(MapAtmosphere.band_fade(700, in_band, out_band) == 0.0, "fondu nul après la sortie")
	_check(MapAtmosphere.band_fade(150, in_band, out_band) < MapAtmosphere.band_fade(180, in_band, out_band), "fondu monotone")
	# Saison et aurore.
	_check(is_equal_approx(MapAtmosphere.season_share(Vector4(0, 0, 0, 1), [0.3, 0.1, 0.8, 1.0]), 1.0), "brouillard d'hiver")
	_check(MapAtmosphere.aurora_strength(0.0, 1.0, 0.35) == 0.0, "pas d'aurore hors hiver")
	_check(MapAtmosphere.aurora_strength(1.0, 0.0, 0.35) > 0.3 and MapAtmosphere.aurora_strength(1.0, 1.0, 0.35) == 1.0, "aurore d'hiver : plancher puis soir")
	# Bancs de fleuve : plafond respecté, déterministe, seuil d'importance.
	var line := PackedVector2Array()
	for index in 200:
		line.append(Vector2(index * 10.0, 5.0))
	var rivers: Array = [{"importance": 4, "points": line}, {"importance": 1, "points": line}]
	var all_points := MapAtmosphere.river_bank_points(rivers, 3, 38.0, 1000)
	_check(all_points.size() > 20, "bancs posés le long du fleuve (%d)" % all_points.size())
	_check(MapAtmosphere.river_bank_points(rivers, 3, 38.0, 10).size() == 10, "plafond de bancs")
	_check(MapAtmosphere.river_bank_points(rivers, 5, 38.0, 100).is_empty(), "aucun fleuve assez important")
	_check(MapAtmosphere.river_bank_points(rivers, 3, 38.0, 1000) == all_points, "déterministe")
	_check_build()


func _check_build() -> void:
	var map_data := MapData.load_from_dir((load("res://scripts/map/map_paths.gd") as GDScript).call("default_data_dir").path_join("map"))
	if not _check(map_data.load_error == "", "carte : " + map_data.load_error):
		return
	var stub := GDScript.new()
	stub.source_code = "extends Node\nvar map_data\n"
	stub.reload()
	var map: Node = stub.new()
	map.set("map_data", map_data)
	root.add_child(map)
	var atmosphere := MapAtmosphere.new()
	map.add_child(atmosphere)
	atmosphere.setup(map, null)
	_check(atmosphere.enabled, "atmosphère active")
	var cumulus := atmosphere.get_node_or_null("CumulusField") as MultiMeshInstance3D
	_check(cumulus != null and cumulus.multimesh.instance_count == 3 and atmosphere.cumulus_material() != null, "champ de cumulus en trois plans")
	_check(atmosphere.get_node_or_null("Cirrus") != null, "cirrus")
	_check((atmosphere.get_node_or_null("Aurora") != null) == bool(MapAtmosphere.data()["aurora"]["enabled"]), "aurore selon les données")
	var fog := atmosphere.get_node_or_null("RiverFog") as MultiMeshInstance3D
	_check(fog != null and fog.multimesh.instance_count <= int(MapAtmosphere.data()["river_fog"]["max_banks"]), "bancs de fleuve plafonnés")
	# Rideaux : provinces en pluie ou orage seulement, plafond, orages d'abord.
	var weather := {}
	for index in range(1, 40):
		var id := str(map_data.get_province(index).get("id", ""))
		weather[id] = {"kind": ["rain", "storm", "clear", "snow"][index % 4], "intensity": 0.8, "label": ""}
	atmosphere.refresh(weather)
	var rule: Dictionary = MapAtmosphere.data()["rain_curtains"]
	var placements := atmosphere.rain_placements(weather, rule)
	var wet := 0
	for entry: Dictionary in weather.values():
		if entry["kind"] in ["rain", "storm"]:
			wet += 1
	_check(placements.size() == mini(wet * int(rule["per_province"]), int(rule["max_curtains"])), "un rideau par pluie/orage (%d)" % placements.size())
	_check(placements.size() > 0 and float(placements[0]["storm"]) == 1.0, "orages d'abord")
	var curtains := atmosphere.get_node_or_null("RainCurtains") as MultiMeshInstance3D
	_check(curtains != null and curtains.multimesh.instance_count == placements.size(), "rideaux en un MultiMesh")
	var nodes := 0
	for child in atmosphere.get_children():
		nodes += 1
	_check(nodes <= 6, "pas de nœud par instance (%d)" % nodes)
	atmosphere.refresh({})
