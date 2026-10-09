extends SceneTree

## Test headless du chantier TX 2b (ADR 0243) : sols de campagne régionaux.
##  1. `CampaignTextures.build_layer_table` : chaque biome 1-14 reçoit sa couche de chaque rôle
##     (herbe, sous-bois, roche locales ; neige et sable partagés), surcharge de lande respectée ;
##  2. cartes de fondu des biomes : tailles, indices 1-14, distance nulle ailleurs qu'en bordure ;
##  3. paquet de grains : un grain par rôle du shader ;
##  4. parcellaire : `parcels_source` vaut « hb » par défaut, les surcharges « tx » ne citent que
##     des matières du paquet TX et la table du sol se construit avec ce paquet.
## Usage : godot --headless --path game --script res://tests/tx_campaign_test.gd

var _failures := 0


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("tx_campaign_test: " + message)
	return condition


func _init() -> void:
	await process_frame
	_test_layer_table()
	_test_blend_maps()
	_test_micro()
	_test_parcels()
	print("tx_campaign_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _layer_info(layers: Array, layer: int) -> Dictionary:
	for entry in layers:
		if int((entry as Dictionary)["layer"]) == layer:
			return entry
	return {}


func _test_layer_table() -> void:
	var regional := CampaignTextures.regional_spec()
	if not _check(not regional.is_empty(), "bloc regional absent des données"):
		return
	var manifest := CampaignTextures.pack_manifest(str(regional["pack"]))
	var layers: Array = manifest.get("layers", [])
	_check(layers.size() == 45, "45 couches de fond attendues, %d" % layers.size())
	var table := CampaignTextures.build_layer_table(layers, regional)
	_check(table.size() == CampaignTextures.BIOME_COUNT * 7, "table de %d entrées" % table.size())
	var expected := ["grass_bare", "grass_bare", "understory", "rock", "grass_bare", "snow", "sand"]
	for biome in range(1, CampaignTextures.BIOME_COUNT):
		for role in 7:
			var info := _layer_info(layers, table[biome * 7 + role])
			var want: String = expected[role]
			if role == 4 and (regional["overrides"] as Dictionary).has(str(biome)):
				want = "understory"
			if not _check(str(info.get("role", "")) == want, "biome %d rôle %d : couche %s (%s attendu)" % [biome, role, info.get("id", "?"), want]):
				continue
			if want in ["grass_bare", "understory", "rock"]:
				var listed: Array = info.get("biomes", [])
				var own := false
				for b in listed:
					own = own or int(b) == biome
				_check(own, "biome %d rôle %d : couche %s d'un autre biome" % [biome, role, info.get("id", "?")])
	# Les couches partagées (neige, sable) sont les mêmes partout.
	for role in [5, 6]:
		for biome in range(2, CampaignTextures.BIOME_COUNT):
			_check(table[biome * 7 + role] == table[7 + role], "rôle partagé %d différent au biome %d" % [role, biome])
	# Un biome absent du paquet prend le parent : on retire les couches du biome 8 (désert -> 7).
	var reduced: Array = layers.filter(func(entry: Dictionary) -> bool: return not (entry["biomes"] as Array).has(8.0) or (entry["biomes"] as Array).size() > 1)
	var fallback := CampaignTextures.build_layer_table(reduced, regional)
	var parent_layer := _layer_info(reduced, fallback[8 * 7])
	_check((parent_layer.get("biomes", []) as Array).has(7.0), "le biome 8 sans couche doit prendre le parent 7")


func _test_blend_maps() -> void:
	var regional := CampaignTextures.regional_spec()
	var ab := Image.load_from_file(DataFile.path_of(str(regional["blend_ab"])))
	var dist := Image.load_from_file(DataFile.path_of(str(regional["blend_dist"])))
	if not _check(ab != null and dist != null, "cartes de fondu absentes (cent-ans geo biome-blend)"):
		return
	_check(ab.get_size() == dist.get_size(), "cartes de fondu de tailles différentes")
	var size := ab.get_size()
	_check(size.x * int(regional["blend_texel_px"]) == 7168, "largeur des cartes de fondu %d" % size.x)
	var bad := 0
	for y in range(0, size.y, 37):
		for x in range(0, size.x, 37):
			var packed := int(ab.get_pixel(x, y).r * 255.0 + 0.5)
			if packed % 16 < 1 or packed % 16 > 14 or packed / 16 < 1 or packed / 16 > 14:
				bad += 1
	_check(bad == 0, "%d échantillons de la carte de fondu hors 1-14" % bad)
	# Au moins une frontière (distance faible) et de grandes zones loin de toute frontière.
	var near := 0
	var far := 0
	for y in range(0, size.y, 29):
		for x in range(0, size.x, 29):
			var d := dist.get_pixel(x, y).r * 255.0
			near += 1 if d < 3.0 else 0
			far += 1 if d > 20.0 else 0
	_check(near > 0 and far > 0, "distances de fondu incohérentes (près %d, loin %d)" % [near, far])


func _test_micro() -> void:
	var block: Dictionary = CampaignTextures.spec().get("micro", {})
	if not _check(not block.is_empty(), "bloc micro absent des données"):
		return
	var manifest := CampaignTextures.pack_manifest(str(block["pack"]))
	var ids := []
	for entry in manifest.get("layers", []):
		ids.append(str((entry as Dictionary)["id"]))
	for role in CampaignTextures.SHADER_LAYER_ORDER:
		_check(ids.has(str((block["roles"] as Dictionary).get(role, ""))), "grain du rôle %s absent du paquet" % role)
	var fade: Array = block["fade_footprint"]
	_check(float(fade[0]) < float(fade[1]), "fondu du grain : début >= fin")


func _test_parcels() -> void:
	var mix_doc: Variant = DataFile.read_json(HbGround.MIX_FILE)
	if not _check(mix_doc is Dictionary, "ground_biome_mix.json illisible"):
		return
	var mix := mix_doc as Dictionary
	_check(HbGround.parcels_source(mix) == "tx", "parcels_source par défaut doit être tx (ADR 0243)")
	var manifest := CampaignTextures.pack_manifest(GroundMaterials.TX_MANIFEST_FILE)
	var layer_ids := {}
	for entry in manifest.get("layers", []):
		layer_ids[str((entry as Dictionary)["id"])] = int((entry as Dictionary)["layer"])
	_check(layer_ids.size() == 42, "42 matières de parcellaire TX attendues, %d" % layer_ids.size())
	var merged := HbGround.with_tx_overrides(mix)
	var table := HbGround.build_table(HbGround.mix_rows(merged), layer_ids)
	_check(table != null, "table du sol impossible avec le paquet TX")
	var biome8: Dictionary = (merged["biomes"] as Dictionary)["8"]
	_check((biome8["wild"] as Array).has("dunes"), "surcharge TX du désert non appliquée")
	# Le mélange d'origine reste intact.
	_check(not ((mix["biomes"] as Dictionary)["8"]["wild"] as Array).has("dunes"), "with_tx_overrides a modifié le mélange d'origine")
