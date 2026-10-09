extends TestCase

## Test headless du lot FC2 (arbres imposteurs de la carte) :
##  1. atlas albédo et normale : présents, importés en compression VRAM avec mipmaps, 8 vues ×
##     3 essences ; quadrilatère d'imposteur de 2 triangles par essence ;
##  2. carte de campagne (forêt d'Orléans, d = 25 ; VT3 : portées de détail réduites, les arbres 1:1
##     s'arrêtant à ≈ 30 unités) : tuiles lointaines en imposteurs (matériau
##     dédié), tuiles proches en maillage détaillé, haies inchangées ; triangles lointains avant /
##     après ;
##  3. repli `--no-fc2` (use_impostors = false puis reconstruction) : maillages bas au loin.
## Usage : godot --headless --path game --script res://tests/fc2_impostors_test.gd

const ORLEANS_FOREST := Vector2(2180.0, 3333.0)


func _init() -> void:
	await process_frame
	# HC1 (ADR 0161) : ce test porte sur les arbres 1:1 (style `real`, défaut d'avant HC1).
	MapPropScale.set_tree_style(MapPropScale.TREE_STYLE_REAL)
	Ga3Vegetation.force(false)  # GA3-L2 : ce test porte sur l'état FC (cartes proches, atlas FC2)
	_test_atlases()
	await _test_map()
	finish()


func _test_atlases() -> void:
	for path in [VegetationMeshes.IMPOSTOR_ALBEDO, VegetationMeshes.IMPOSTOR_NORMAL]:
		var texture := load(path) as Texture2D
		if not check(texture != null, "atlas loads (%s)" % path):
			continue
		check(texture is CompressedTexture2D, "atlas imported as CompressedTexture2D (%s)" % path)
		check(texture.get_width() == VegetationMeshes.IMPOSTOR_VIEWS * 256 and texture.get_height() == VegetationMeshes.IMPOSTOR_ROWS.size() * 256,
			"atlas layout 8 views x 3 essences at 256 px (%s: %dx%d)" % [path, texture.get_width(), texture.get_height()])
		var config := ConfigFile.new()
		if check(config.load(path + ".import") == OK, "import file (%s)" % path):
			check(int(config.get_value("params", "compress/mode", 0)) == 2, "VRAM compression (%s)" % path)
			check(bool(config.get_value("params", "mipmaps/generate", false)), "mipmaps (%s)" % path)
			var meta: Dictionary = config.get_value("remap", "metadata", {})
			check(bool(meta.get("vram_texture", false)), "imported as a VRAM texture (%s)" % path)
	for essence in VegetationMeshes.IMPOSTOR_ROWS:
		var mesh := VegetationMeshes.impostor(essence)
		var vertices: PackedVector3Array = mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
		var uv2: PackedVector2Array = mesh.surface_get_arrays(0)[Mesh.ARRAY_TEX_UV2]
		check(vertices.size() == 6, "impostor quad has 2 triangles (%s)" % essence)
		check(not uv2.is_empty() and is_equal_approx(uv2[0].x, float(VegetationMeshes.IMPOSTOR_ROWS.find(essence))), "impostor atlas row (%s)" % essence)


func _settle(rig: CampaignCamera, focus: Vector3, distance: float, vegetation: Vegetation) -> void:
	rig.look_at_point(focus, distance)
	rig.snap()
	for i in 40:
		await process_frame
		if i > 10 and vegetation.pending_jobs() == 0:
			break
	for i in 3:
		await process_frame


func _test_map() -> void:
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not check(map.get("load_ok") == true, "campaign map loads"):
		map.queue_free()
		return
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	var data: MapData = map.map_data
	var vegetation: Vegetation = map.get_node_or_null("Vegetation")
	if not check(vegetation != null, "vegetation node"):
		map.queue_free()
		return
	for i in 3:
		await process_frame
	check(vegetation.impostors_active(), "impostor material ready")
	var focus := Vector3(ORLEANS_FOREST.x, data.surface_world_at(ORLEANS_FOREST.x, ORLEANS_FOREST.y), ORLEANS_FOREST.y)
	# FC5 : cartes de feuillage sous `near_distance` (45) seulement.
	await _settle(rig, focus, 25.0, vegetation)
	var close := vegetation.lod_census()
	print("fc2: census at d=25 %s" % close)
	check(int(close.get("near", 0)) > 0, "FC5: nearest parts in leaf cards (d=25)")
	check(int(close["low"]) == 0 and int(close["detailed"]) == 0, "FC5: no 90/20-triangle meshes for oak/beech/fir (d=25)")
	# VT3 : arbres 1:1 coupés à ≈ 30 unités de la caméra, en deçà de `detail_distance` (170) :
	# portées de détail réduites pour exercer les niveaux lointains à d = 25.
	vegetation.detail_distance = 14.0
	vegetation.near_distance = 7.0
	await _settle(rig, focus, 25.0, vegetation)
	var after := vegetation.lod_census()
	print("fc2: census with impostors %s" % after)
	check(int(after["impostor"]) > 0, "far tiles use the impostor mesh")
	check(int(after["low"]) == 0, "no low mesh left for oak/beech/fir")
	# Hiver : variante ajourée du feuillage maillé, imposteurs dans le même shader (rendu fenêtré :
	# compile les deux).
	var life: Variant = map.get("life")
	var seasons: Variant = (life as Object).get("seasons") if life is Object else null
	if seasons is SeasonVisuals:
		(seasons as SeasonVisuals).set_season("winter", true)
		for i in 40:
			await process_frame
		check(vegetation.foliage_material().shader == Vegetation.FOLIAGE_WINTER_SHADER, "winter foliage variant")
		check(vegetation.impostor_material().shader == Vegetation.IMPOSTOR_SHADER, "impostors keep their shader in winter")
		(seasons as SeasonVisuals).set_season("summer", true)
	# FC5 repli (`--no-fc5`) : maillages détaillés de près, imposteurs au loin.
	vegetation.use_near_cards = false
	vegetation.build(data)
	await _settle(rig, focus, 25.0, vegetation)
	var no_cards := vegetation.lod_census()
	print("fc2: census without leaf cards (--no-fc5) %s" % no_cards)
	check(not vegetation.near_cards_active() and int(no_cards["near"]) == 0 and int(no_cards["detailed"]) > 0, "--no-fc5 falls back to the detailed meshes")
	# Repli : maillages bas au loin.
	vegetation.use_impostors = false
	vegetation.detail_distance = 1.0  # toutes les parties au niveau lointain
	vegetation.build(data)
	await _settle(rig, focus, 25.0, vegetation)
	var before := vegetation.lod_census()
	print("fc2: census without impostors (--no-fc2) %s" % before)
	check(not vegetation.impostors_active(), "--no-fc2 disables the impostor material")
	check(int(before["impostor"]) == 0 and int(before["low"]) > 0, "--no-fc2 falls back to the low meshes")
	if int(before["low_triangles"]) > 0 and int(after["impostor_triangles"]) > 0:
		print("fc2: far tree triangles %d (low meshes) -> %d (impostors)" % [before["low_triangles"], after["impostor_triangles"]])
		check(int(after["impostor_triangles"]) < int(before["low_triangles"]), "fewer far triangles with impostors")
	map.queue_free()
	await process_frame
