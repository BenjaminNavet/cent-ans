extends TestCase

## Test headless du lot GR 4 (habitat rural selon le paysage agraire) :
##  1. données : les règles d'habitat existent et filtrent par paysage connu (`agri_landscapes.json`) ;
##  2. filtre `landscapes` / `exclude_landscapes` lu dans `agri_regions.png` : la Beauce (openfield)
##     refuse une règle de bocage et accepte une ferme d'openfield exclue, la Bretagne (bocage) l'inverse ;
##  3. semis : aucune ferme isolée de bocage en openfield, des fermes en bocage.
## Usage : godot --headless --path game --script res://tests/gr_habitat_test.gd

const HABITAT_RULES := ["bocage_fermes_isolees", "bocage_haies_denses", "mas_sud", "cabanons_de_vigne", "murets_de_terrasses", "fermes_de_clairiere"]


func _init() -> void:
	await process_frame
	_test_data()
	await _test_map()
	finish()


func _rule(config: Dictionary, rule_id: String) -> Dictionary:
	for rule: Dictionary in config.get("rules", []):
		if str(rule["id"]) == rule_id:
			return rule
	return {}


func _test_data() -> void:
	var config := CountrysideLayer.load_config()
	var landscapes: Dictionary = (DataFile.read_json("map/agri_landscapes.json") as Dictionary).get("landscapes", {})
	for rule_id: String in HABITAT_RULES:
		var rule := _rule(config, rule_id)
		if not check(not rule.is_empty(), "rule %s exists" % rule_id):
			continue
		var filters: Dictionary = rule.get("filters", {})
		check(not (filters.get("landscapes", []) as Array).is_empty(), "rule %s filters by landscape" % rule_id)
		for landscape_id: String in (filters.get("landscapes", []) as Array) + (filters.get("exclude_landscapes", []) as Array):
			check(landscapes.has(landscape_id), "rule %s landscape %s known" % [rule_id, landscape_id])
	var enclos := _rule(config, "bocage_enclos")
	check((enclos.get("filters", {}).get("exclude_landscapes", []) as Array).has("openfield_north"), "bocage_enclos excludes openfield_north")


func _test_map() -> void:
	var data := MapData.load_from_dir(MAP_PATHS.default_data_dir().path_join("map"))
	if not check(data.load_error == "", "map loads: %s" % data.load_error):
		return
	var layer := CountrysideLayer.new()
	root.add_child(layer)
	layer.setup(data)
	for i in 3:
		await process_frame
	var config := layer.config
	var beauce := FaunaLayer.lonlat_to_px(1.9, 48.2, data)
	var bretagne := FaunaLayer.lonlat_to_px(-2.9, 48.1, data)
	var farms := _rule(config, "bocage_fermes_isolees")
	var enclos := _rule(config, "bocage_enclos")
	var clairiere := _rule(config, "fermes_de_clairiere")
	print("agri rows: beauce=%d bretagne=%d" % [layer.agri_row_at(beauce), layer.agri_row_at(bretagne)])
	check(layer.agri_row_at(beauce) > 0, "Beauce has an agrarian landscape")
	check(not layer.landscape_ok(beauce, farms), "Beauce rejects the bocage farm rule")
	check(layer.landscape_ok(bretagne, farms), "Bretagne accepts the bocage farm rule")
	check(not layer.landscape_ok(beauce, enclos), "Beauce is excluded from bocage_enclos")
	check(layer.landscape_ok(bretagne, enclos), "Bretagne keeps bocage_enclos")
	check(not layer.landscape_ok(beauce, clairiere) and not layer.landscape_ok(bretagne, clairiere), "swidden rule rejects west Europe")
	check(layer.landscape_ok(beauce, {"id": "free"}), "no landscape filter = everywhere")
	# Semis : sur toute la carte, les fermes de bocage ne tombent que dans le bocage.
	var side := 96
	var farm_count := 0
	var wrong := 0
	var bocage_row := layer.agri_row_at(bretagne)
	for cy in range(0, ceili(data.size.y / float(side))):
		for cx in range(0, ceili(data.size.x / float(side))):
			for inst: Dictionary in layer.cell_instances(Vector2i(cx, cy)):
				if str(inst["rule"]) == "bocage_fermes_isolees":
					farm_count += 1
					if layer.agri_row_at(inst["pos"]) != bocage_row:
						wrong += 1
	print("bocage farms: %d" % farm_count)
	check(farm_count > 0, "bocage farms are scattered")
	check(wrong == 0, "no bocage farm outside the bocage landscape")
	layer.queue_free()
