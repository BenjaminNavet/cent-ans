extends TestCase

## TX T3/T4 : chargeurs de textures régionales (cartes au sol, herbe de bataille par groupe,
## écorces et feuilles par essence, matières de bâtiments par région) et table de régions.
## Usage : godot --headless --path game --script res://tests/tx_veg_build_test.gd


func _init() -> void:
	await process_frame
	_check_ground_cards()
	_check_grass_groups()
	_check_tree_textures()
	_check_building_regions()
	finish()


func _check_ground_cards() -> void:
	check(GroundCards.ready(), "ground cards pack loads")
	for biome in range(1, 15):
		var layers := GroundCards.layers_for_biome(biome)
		check(layers.size() >= 5, "biome %d has >= 5 cards (%d)" % [biome, layers.size()])
	check(GroundCards.pick(8, 12345) >= 0, "a card is picked for the desert")
	check(GroundCards.size_m(GroundCards.pick(8, 1)) > 0.0, "card size known")


func _check_grass_groups() -> void:
	var expected := {1: "green", 2: "green", 3: "dry", 4: "steppe", 5: "arctic", 6: "alpine", 7: "dry", 8: "steppe", 9: "arctic", 10: "green", 11: "green", 12: "green", 13: "green", 14: "dry"}
	for biome: int in expected:
		check(BattleGrassGroups.group_of(biome) == expected[biome], "biome %d -> %s (got %s)" % [biome, expected[biome], BattleGrassGroups.group_of(biome)])
	var cards := BattleGrassGroups.cards(4)
	for role in ["grass_blades", "grass_tufts", "grass_clump"]:
		check(cards.has(role), "steppe has %s" % role)
		check(BattleGrassGroups.texture(cards, role, null) != null, "steppe %s texture loads" % role)


func _check_tree_textures() -> void:
	for species in ["oak", "beech", "poplar", "willow", "birch", "holm_oak"]:
		check(TreeTextures.bark_layer(species) >= 0, "%s bark layer" % species)
		check(TreeTextures.leaves_layer(species) >= 0, "%s leaves layer" % species)
	check(TreeTextures.bark_layer("ash") == -1, "ash has no generated bark")
	check(TreeTextures.bark_albedo() != null and TreeTextures.leaves_albedo() != null, "bark and leaves arrays load")


func _check_building_regions() -> void:
	check(BuildingMaterials.regional_ready(), "regional building pack loads")
	var ids := BuildingRegions.region_ids()
	check(ids.size() > 30 and ids.size() <= BuildingMaterials.REGION_CAP, "regions fit the shader table (%d)" % ids.size())
	for region: String in ids:
		check(BuildingRegions.materials_for_region(region).has("Plaster"), "%s has regional plaster" % region)
	BuildingMaterials.set_region("maghreb")
	var atlas := BuildingMaterials.material("Building") as ShaderMaterial
	check(atlas != null and bool(atlas.get_shader_parameter("use_regional")), "atlas carries the regional tables")
	check(int(atlas.get_shader_parameter("region_id")) == ids.find("maghreb"), "atlas region id set")
	BuildingMaterials.set_region("")
	var plain := BuildingMaterials.material("Building") as ShaderMaterial
	check(int(plain.get_shader_parameter("region_id")) == -1, "no region: default materials")
	var style := BuildingRegions.style_for_region("france_nord")
	check(not style.is_empty() and not BuildingRegions.style_for_province("").has("region"), "style helpers intact")
