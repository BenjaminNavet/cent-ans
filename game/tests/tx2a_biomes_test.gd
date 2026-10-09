extends SceneTree

## Test headless du chantier TX 2a (ADR 0242) : 15 classes de biomes et repli sur le parent.
##  1. `BiomeParents` : table lue de `data/map/biome_parents.json`, `table_row`, `mask_allows` ;
##  2. `TreeSpecies` : un biome 8-14 sans entrée reprend poids et paramètres de son parent ;
##  3. `HbGround` : lignes 16-22 de la table du sol (entrées 8-14 du mélange), repli sur le parent
##     quand le mélange n'a pas d'entrée, paysages ME8 intacts sur les lignes 8-15 ;
##  4. `FieldPlan` (non modifié) : `HbGround._load_biomes` rend la carte repliée (indices 1-7), et
##     chaque biome régional donne une culture dominante.
## Usage : godot --headless --path game --script res://tests/tx2a_biomes_test.gd

const EXPECTED_PARENTS := [0, 1, 2, 3, 4, 5, 6, 7, 7, 5, 2, 1, 2, 5, 3]
## lon, lat, biome régional attendu dans `biomes.png`.
const PLACES := {
	"Biskra": [5.7, 34.85, 8], "Laponie": [25.0, 69.0, 9], "Kiev": [30.5, 50.4, 10],
	"Landes": [-0.8, 44.2, 11], "Hongrie": [19.5, 47.0, 12], "Riga": [24.1, 57.0, 13],
	"Péloponnèse": [22.4, 37.5, 14],
}

var _failures := 0


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("tx2a_biomes_test: " + message)
	return condition


func _init() -> void:
	await process_frame
	_test_parents()
	_test_tree_species()
	_test_ground_table()
	_test_field_plan()
	print("tx2a_biomes_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _test_parents() -> void:
	BiomeParents.reset()
	var table := BiomeParents.parents()
	_check(table.size() == BiomeParents.COUNT, "parents size %d" % table.size())
	for i in EXPECTED_PARENTS.size():
		_check(table[i] == EXPECTED_PARENTS[i], "parent of %d is %d" % [i, table[i]])
	# Sans fichier : chaque classe est son propre parent.
	var identity := BiomeParents.parse(null)
	_check(identity[10] == 10 and identity.size() == BiomeParents.COUNT, "identity table")
	_check(BiomeParents.table_row(7) == 7 and BiomeParents.table_row(8) == 16 and BiomeParents.table_row(14) == 22, "table rows")
	# Un masque qui admet le continental admet le continental est, pas l'inverse.
	var continental := 1 << 2
	_check(BiomeParents.mask_allows(continental, 10), "continental_east inherits continental")
	_check(BiomeParents.mask_allows(continental, 12), "pannonian inherits continental")
	_check(not BiomeParents.mask_allows(continental, 11), "atlantic_south is not continental")
	_check(not BiomeParents.mask_allows(1 << 10, 2), "a parent does not inherit its child")
	_check(not BiomeParents.mask_allows(continental, 0), "sea never allowed")


func _test_tree_species() -> void:
	var species := TreeSpecies.new()
	var doc := {
		"distribution": {"default_biome": 2},
		"biomes": {"2": {"forest": 0.9, "isolated": 0.001}, "9": {"forest": 0.05}},
		"species": [
			{"id": "a", "mesh": "oak", "biomes": {"2": 1.0}, "roles": {"massif": 1.0}},
			{"id": "b", "mesh": "oak", "biomes": {"2": 0.5, "10": 0.0}, "roles": {"massif": 1.0}},
			{"id": "c", "mesh": "conifer", "biomes": {"2": 0.3}, "roles": {"massif": 1.0}},
		],
	}
	_check(species.load_dict(doc), "synthetic table loads")
	_check(TreeSpecies.BIOME_COUNT == 15, "15 biomes")
	var n := species.count
	var role := TreeSpecies.Role.MASSIF
	# Biome 12 (pannonien) sans entrée : poids du continental (2) ; 10 : poids 0.0 explicite coupé.
	_check(species.base[(12 * TreeSpecies.ROLE_COUNT + role) * n] == species.base[(2 * TreeSpecies.ROLE_COUNT + role) * n], "weight inherited")
	_check(species.base[(10 * TreeSpecies.ROLE_COUNT + role) * n + 1] == 0.0, "explicit zero cuts the weight")
	_check(species.base[(10 * TreeSpecies.ROLE_COUNT + role) * n] > 0.0, "other species inherited")
	_check(is_equal_approx(species.biome_param(12, 0), 0.9), "params inherited")
	_check(is_equal_approx(species.biome_param(9, 0), 0.05), "explicit params kept")
	_check(species.table()["biome_parent"].size() == 15, "table carries the parents")


func _test_ground_table() -> void:
	var data_dir := OutbuildingLayer.data_dir()
	var arrays := GroundMaterials.load_arrays()
	var mix: Variant = JSON.parse_string(FileAccess.get_file_as_string(data_dir.path_join(HbGround.MIX_FILE)))
	if arrays.is_empty() or not mix is Dictionary:
		print("tx2a_biomes_test: matières ou mélange absents, table du sol ignorée")
		return
	var layers: Dictionary = arrays["layers"]
	var biomes: Dictionary = (mix as Dictionary)["biomes"]
	for b in range(1, BiomeParents.COUNT):
		_check(biomes.has(str(b)), "mix entry for biome %d" % b)
	var rows := HbGround.mix_rows(mix as Dictionary)
	_check((rows["biomes"] as Dictionary).has("16") and (rows["biomes"] as Dictionary).has("22"), "regional rows 16..22")
	_check(not (rows["biomes"] as Dictionary).has("8"), "rows 8..15 left to the landscapes")
	var table := HbGround.build_table(rows, layers)
	if not _check(table != null and table.get_height() == HbGround.TABLE_HEIGHT, "table built"):
		return
	for b in range(BiomeParents.FIRST_REGIONAL, BiomeParents.COUNT):
		var entry: Dictionary = biomes[str(b)]
		var row := BiomeParents.table_row(b)
		var canopy := int(round(table.get_pixel(12, row).r * HbGround.LAYER_NORM))
		_check(canopy == int(layers[entry["canopy"]]), "biome %d canopy layer" % b)
	# Repli : sans entrée pour le biome 10, la ligne 18 reprend celle du continental (2).
	var thin := (mix as Dictionary).duplicate(true)
	(thin["biomes"] as Dictionary).erase("10")
	var thin_table := HbGround.build_table(HbGround.mix_rows(thin), layers)
	for x in HbGround.TABLE_WIDTH:
		_check(thin_table.get_pixel(x, 18) == thin_table.get_pixel(x, 2), "fallback row 18 == row 2 at x=%d" % x)
	# Les paysages ME8 gardent les lignes 8..15 : une fois fusionnés, rien ne les écrase.
	var doc := HbGround._read_json(data_dir.path_join(HbGround.AGRI_FILE))
	if not doc.is_empty():
		var merged := HbGround.mix_rows(mix as Dictionary)
		var resolved := HbGround.resolve_landscapes(doc, layers)
		for key in resolved:
			(merged["biomes"] as Dictionary)[key] = resolved[key]
		var full := HbGround.build_table(merged, layers)
		_check(full != null and full.get_height() == HbGround.TABLE_HEIGHT, "table with landscapes")
		_check(resolved.size() <= 8, "landscapes fit rows 8..15")


func _test_field_plan() -> void:
	var data_dir := OutbuildingLayer.data_dir()
	var config := FieldLayer._read_json(data_dir.path_join(FieldLayer.CONFIG_FILE))
	var map_data := MapData.load_from_dir(data_dir.path_join("map"))
	if config.is_empty() or map_data.load_error != "":
		print("tx2a_biomes_test: carte ou configuration des champs absente, FieldPlan ignoré")
		return
	var plan := FieldPlan.new()
	plan.setup(config, map_data, FieldLayer._read_json(data_dir.path_join(HbGround.MIX_FILE)), FieldLayer._read_json(data_dir.path_join(HbGround.AGRI_FILE)),
		HbGround._load_biomes(data_dir.path_join(HbGround.BIOMES_FILE)), HbGround._load_biomes(data_dir.path_join(HbGround.AGRI_MASK_FILE)), PackedVector2Array())
	_check(plan.is_ready(), "plan ready")
	var raw := Image.load_from_file(data_dir.path_join(HbGround.BIOMES_FILE))
	var folded := HbGround._load_biomes(data_dir.path_join(HbGround.BIOMES_FILE))
	_check(raw != null and folded != null and raw.get_size() == folded.get_size(), "same raster size")
	var data := folded.get_data()
	var step := 97
	for i in range(0, data.size(), step):
		if data[i] > 7:
			_check(false, "folded biome raster holds %d" % data[i])
			break
	for place in PLACES:
		var info: Array = PLACES[place]
		var px := FaunaLayer.lonlat_to_px(float(info[0]), float(info[1]), map_data)
		var x := clampi(int(px.x / map_data.size.x * raw.get_width()), 0, raw.get_width() - 1)
		var y := clampi(int(px.y / map_data.size.y * raw.get_height()), 0, raw.get_height() - 1)
		var biome := int(round(raw.get_pixel(x, y).r * 255.0))
		_check(biome == int(info[2]), "%s: raw biome %d, expected %d" % [place, biome, info[2]])
		var row := plan.ground_row(px)
		_check(row >= 1 and row <= 15, "%s: ground row %d" % [place, row])
		_check(plan.dominant_material(px) != "", "%s: a dominant field material" % place)
