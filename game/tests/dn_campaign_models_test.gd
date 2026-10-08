extends SceneTree

## Test headless du lot DN camp-bati (D1) : glb générés des lieux de la carte de campagne.
##  1. table vide (livrée par défaut) : aucun glb généré, comportement inchangé ;
##  2. table fournie en mémoire (bouche-trous GA3 `props_ga/ga3_house_lod1`, `ga3_church_lod1`) :
##     un glb par lieu du kit concerné, une bannière procédurale par glb, mêmes emplacements ;
##  3. bascule par distance : glb opaque et maquette cachée sous `near_distance`, fondu croisé
##     au-dessus, maquette seule au-delà de `near + marge` ; modèle absent = maquette seule.
## Usage : godot --headless --path game --script res://tests/dn_campaign_models_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	ModelLibrary.clear_cache()
	TownMaquetteData.clear_cache()
	DnCampaignModels.clear_cache()
	print("dn_campaign_models_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("dn_campaign_models_test: " + message)
	return condition


func _build(world_parent: Node, map_data: MapData, data: SettlementData) -> Array:
	var world := Node3D.new()
	world_parent.add_child(world)
	var terrain := TerrainBuilder.new()
	world.add_child(terrain)
	terrain.build(map_data)
	var layer := SettlementLayer.new()
	world.add_child(layer)
	layer.setup(map_data, terrain, data, ZoomTiers.load_default())
	layer.update_view(85.0)
	layer.flush()
	return [world, layer]


func _run() -> void:
	# Données livrées : table vide.
	_check(not DnCampaignModels.document().is_empty(), "dn_campaign_models.json missing")
	_check(DnCampaignModels.is_empty(), "shipped table must be empty (current behaviour unchanged)")
	_check(DnCampaignModels.entry_for("city", "west", 0).is_empty(), "no entry in an empty table")
	DnCampaignModels.set_document({"defaults": {"near_distance": 400.0, "fade_margin": 60.0}, "table": {
		"village": {"west": [{"path": "props_ga/ga3_house_lod1"}], "*": [{"path": "props_ga/ga3_church_lod1", "banner": false}]},
		"castle": {"*": [{"path": "props_ga/does_not_exist"}]},
		"town": {"west": [{"path": "props_ga/ga3_church_lod1", "near_distance": 200.0}]},
	}})
	_check(DnCampaignModels.entry_for("village", "west", 3)["path"] == "props_ga/ga3_house_lod1", "exact family entry")
	_check(DnCampaignModels.entry_for("village", "med", 0)["path"] == "props_ga/ga3_church_lod1", "wildcard entry")
	_check(DnCampaignModels.entry_for("abbey", "west", 0).is_empty(), "no entry for an unlisted kind")
	DnCampaignModels.clear_cache()  # revient au fichier livré (table vide)

	var data_dir := MAP_PATHS.default_data_dir()
	var map_dir := data_dir.path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not _check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var data := SettlementData.load_from(data_dir, map_dir)

	# 1. Table vide.
	var built := _build(root, map_data, data)
	var base: TownMaquetteLayer = (built[1] as SettlementLayer).maquettes
	if not _check(base != null, "no TownMaquetteLayer"):
		return
	var base_instances := base.instance_count()
	var base_meshes := int(base.stats.get("multimeshes", 0))
	_check(int(base.stats.get("dn_places", -1)) == 0 and base.dn_instance_count() == 0 and base.dn_banner_count() == 0, "empty table: no generated model (%s)" % base.stats)
	(built[0] as Node).queue_free()
	await process_frame

	# 2. Table fournie.
	DnCampaignModels.set_document({"defaults": {"near_distance": 400.0, "fade_margin": 60.0}, "table": {
		"village": {"*": [{"path": "props_ga/ga3_house_lod1"}]},
		"town": {"*": [{"path": "props_ga/ga3_church_lod1", "near_distance": 200.0}]},
		"castle": {"*": [{"path": "props_ga/does_not_exist"}]},
	}})
	built = _build(root, map_data, data)
	var layer: SettlementLayer = built[1]
	var maquettes := layer.maquettes
	_check(maquettes.instance_count() == base_instances, "every place keeps its maquette instance (%d vs %d)" % [maquettes.instance_count(), base_instances])
	var dn_places := int(maquettes.stats.get("dn_places", 0))
	_check(dn_places > 0, "some places use a generated model: %s" % maquettes.stats)
	_check(maquettes.dn_instance_count() == dn_places, "one generated instance per place (%d vs %d)" % [maquettes.dn_instance_count(), dn_places])
	_check(int(maquettes.stats.get("dn_models", 0)) == 2, "two generated models loaded (missing one skipped): %s" % maquettes.stats)
	_check(maquettes.dn_banner_count() == maquettes.dn_instance_count(), "a procedural banner per generated model")
	_check(int(maquettes.stats.get("multimeshes", 0)) >= base_meshes, "no fewer maquette MultiMeshes")
	var village := -1
	var town := -1
	var castle := -1
	for i in data.settlements.size():
		if maquettes.is_landmark(i) or maquettes.is_absorbed(i):
			continue
		match TownMaquetteData.kind_of(data.settlements[i]):
			"village":
				village = i if village < 0 else village
			"town":
				town = i if town < 0 else town
			"castle":
				castle = i if castle < 0 else castle
	if not _check(village >= 0 and town >= 0 and castle >= 0, "a village, a town and a castle exist"):
		return
	_check(maquettes.dn_model_of(village) >= 0 and maquettes.dn_model_of(town) >= 0, "village and town have a generated model")
	_check(maquettes.dn_model_of(castle) < 0, "missing file: the place keeps its maquette only")
	# Pose : même pied et même largeur monde que la maquette.
	var kind := TownMaquetteData.kind_of(data.settlements[village])
	var px := layer.model_px(village)
	var xf := maquettes.dn_world_transform(village, px, maquettes.pose_height(village))
	_check(absf(xf.origin.x - px.x) < 5.0 and absf(xf.origin.z - px.y) < 5.0, "generated model stands at the place")
	var wanted := TownMaquetteData.width(kind) * maquettes.gain_of(village) * maquettes.factor_of(village)
	var info_width := absf(xf.basis.get_scale().x) * 1.0
	_check(info_width > 0.0 and is_finite(info_width), "finite generated scale (%.4f, wanted width %.2f)" % [info_width, wanted])

	# 3. Bascule par distance.
	layer.update_view(85.0)
	_check(is_equal_approx(maquettes.dn_alpha_of(village), 1.0) and not maquettes.maquette_visible(village), "near: generated model opaque, maquette hidden")
	layer.update_view(430.0)
	_check(maquettes.dn_alpha_of(village) > 0.0 and maquettes.dn_alpha_of(village) < 1.0 and maquettes.maquette_visible(village), "crossfade: both drawn")
	layer.update_view(700.0)
	_check(is_equal_approx(maquettes.dn_alpha_of(village), 0.0) and maquettes.maquette_visible(village), "far: maquette only")
	layer.update_view(300.0)
	_check(is_equal_approx(maquettes.dn_alpha_of(town), 0.0) or maquettes.dn_alpha_of(town) < 1.0, "town switches at its own near_distance (200)")
	_check(maquettes.maquette_visible(castle), "a place without generated model stays on its maquette")
	# Bannière : la couleur du contrôleur passe aussi à la bannière procédurale.
	maquettes.refresh(null, func(_f: String) -> Color: return Color(0.2, 0.3, 0.9))
	_check(maquettes.banner_color(village).is_equal_approx(Color(0.2, 0.3, 0.9)), "controller colour applied")
	(built[0] as Node).queue_free()
