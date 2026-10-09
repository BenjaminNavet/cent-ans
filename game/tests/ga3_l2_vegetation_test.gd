extends TestCase

## Test headless du lot GA3-L2 (végétation générée de la carte de campagne) :
##  1. textures GA3 : grille d'imposteurs 8 azimuts × une ligne par essence (HB4 ; lignes 0-2
##     contrôlées ici, les autres par `hb4_species_test.gd`) de 256² (albédo + normale, VRAM, mipmaps),
##     atlas de feuilles 1024 × 512, touffe d'herbe 512 × 256 dense (couverture ≫ 13 %) ;
##  2. option : `Ga3Vegetation.force(false)` (= `--no-ga3-veg`) rend les ressources FC ;
##  3. rochers : 3 variantes × 3 niveaux (≤ 120 / 60 / 18 triangles), semis de `GroundClutter`,
##     un appel de dessin par cellule rocheuse, niveau de détail selon la distance ;
##  4. carte (forêt d'Orléans, d = 25) : arbres proches en imposteurs (plus de cartes), triangles
##     des arbres avant / après imprimés.
## Usage : godot --headless --path game --script res://tests/ga3_l2_vegetation_test.gd

const ORLEANS_FOREST := Vector2(2180.0, 3333.0)
const ROCK_TRIANGLES: Array[int] = [120, 60, 18]


func _init() -> void:
	await process_frame
	# HC1 (ADR 0161) : ce test porte sur les arbres 1:1 (style `real`, défaut d'avant HC1).
	MapPropScale.set_tree_style(MapPropScale.TREE_STYLE_REAL)
	_test_textures()
	_test_option()
	_test_rocks()
	await _test_map()
	Ga3Vegetation.force(null)
	RenderQuality.override_level = ""
	finish()


func _test_textures() -> void:
	# Lot HB4 : une ligne par essence du catalogue (lignes 0-2 : chêne, hêtre, sapin de FC2).
	var rows := maxi(VegetationMeshes.IMPOSTOR_ROWS.size(), TreeSpecies.shared().count)
	var sizes := {
		Ga3Vegetation.IMPOSTOR_ALBEDO: Vector2i(VegetationMeshes.IMPOSTOR_VIEWS * 256, rows * 256),
		Ga3Vegetation.IMPOSTOR_NORMAL: Vector2i(VegetationMeshes.IMPOSTOR_VIEWS * 256, rows * 256),
		Ga3Vegetation.LEAF_CARDS: Vector2i(1024, 512),
		Ga3Vegetation.GRASS_TUFT: Vector2i(512, 256),
	}
	for path: String in sizes:
		var texture := load(path) as Texture2D
		if not check(texture != null, "texture loads (%s)" % path):
			continue
		check(texture is CompressedTexture2D, "VRAM texture (%s)" % path)
		check(Vector2i(texture.get_width(), texture.get_height()) == sizes[path], "size %s (%s)" % [sizes[path], path])
		var config := ConfigFile.new()
		if check(config.load(path + ".import") == OK, "import file (%s)" % path):
			check(bool(config.get_value("params", "mipmaps/generate", false)), "mipmaps (%s)" % path)
	# Grille : chaque cellule porte un arbre (couverture), pied en bas de cellule comme FC2.
	var atlas := Image.load_from_file(ProjectSettings.globalize_path(Ga3Vegetation.IMPOSTOR_ALBEDO))
	if check(atlas != null, "albedo atlas image"):
		for row in VegetationMeshes.IMPOSTOR_ROWS.size():
			for view in VegetationMeshes.IMPOSTOR_VIEWS:
				var cov := _coverage(atlas, Rect2i(view * 256, row * 256, 256, 256))
				check(cov > 0.06 and cov < 0.6, "cell %d,%d covered (%.3f)" % [row, view, cov])
	var tuft := Image.load_from_file(ProjectSettings.globalize_path(Ga3Vegetation.GRASS_TUFT))
	var old := Image.load_from_file(ProjectSettings.globalize_path(GroundClutter.GRASS_TEXTURE))
	if check(tuft != null and old != null, "grass images"):
		var cov := _coverage(tuft, Rect2i(0, 0, tuft.get_width(), tuft.get_height()))
		print("ga3_l2: grass coverage %.3f (FC5 %.3f)" % [cov, _coverage(old, Rect2i(0, 0, old.get_width(), old.get_height()))])
		check(cov > 0.3, "dense grass tuft (coverage %.3f > 0.3)" % cov)


func _coverage(image: Image, rect: Rect2i) -> float:
	var n := 0
	for y in range(rect.position.y, rect.end.y, 2):
		for x in range(rect.position.x, rect.end.x, 2):
			if image.get_pixel(x, y).a > 0.5:
				n += 1
	return float(n) / float((rect.size.x / 2) * (rect.size.y / 2))


func _test_option() -> void:
	Ga3Vegetation.force(true)
	check(Ga3Vegetation.enabled() and Ga3Vegetation.near_impostors(), "GA3 on by default")
	check(Ga3Vegetation.pick(Ga3Vegetation.GRASS_TUFT, "x") == Ga3Vegetation.GRASS_TUFT, "GA3 grass picked")
	var material := Vegetation._make_impostor_material()
	check(material != null and (material.get_shader_parameter("albedo_atlas") as Texture2D).resource_path == Ga3Vegetation.IMPOSTOR_ALBEDO, "GA3 impostor atlas")
	Ga3Vegetation.force(false)
	check(not Ga3Vegetation.near_impostors(), "--no-ga3-veg: no near impostors")
	check(Ga3Vegetation.pick(Ga3Vegetation.GRASS_TUFT, "x") == "x", "--no-ga3-veg: FC grass")
	check(Ga3Vegetation.rock_meshes().is_empty(), "--no-ga3-veg: no rocks")
	material = Vegetation._make_impostor_material()
	check(material != null and (material.get_shader_parameter("albedo_atlas") as Texture2D).resource_path == VegetationMeshes.IMPOSTOR_ALBEDO, "--no-ga3-veg: FC impostor atlas")
	Ga3Vegetation.force(true)


func _triangles(mesh: Mesh) -> int:
	var total := 0
	for s in mesh.get_surface_count():
		var arrays := mesh.surface_get_arrays(s)
		var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX] if arrays[Mesh.ARRAY_INDEX] != null else PackedInt32Array()
		total += indices.size() / 3 if not indices.is_empty() else (arrays[Mesh.ARRAY_VERTEX] as PackedVector3Array).size() / 3
	return total


func _make_clutter() -> GroundClutter:
	RenderQuality.override_level = "high"
	var clutter := GroundClutter.new()
	root.add_child(clutter)
	clutter.max_cells_per_frame = 1000
	clutter.threaded = false
	clutter.weight_sampler = func(_x: float, _y: float) -> Vector2: return Vector2(0.5, 0.1)
	clutter.rock_sampler = func(x: float, _y: float) -> float: return 0.6 if x > 500.0 else 0.0
	clutter.height_sampler = func(points: PackedVector2Array) -> PackedFloat32Array:
		var h := PackedFloat32Array()
		h.resize(points.size())
		return h
	return clutter


func _test_rocks() -> void:
	Ga3Vegetation.force(true)
	var meshes := Ga3Vegetation.rock_meshes()
	check(meshes.size() == Ga3Vegetation.ROCKS.size(), "%d rock variants (%d)" % [Ga3Vegetation.ROCKS.size(), meshes.size()])
	for v in meshes.size():
		for lod in Ga3Vegetation.ROCK_LODS:
			var tris := _triangles(meshes[v][lod])
			check(tris > 8 and tris <= ROCK_TRIANGLES[lod] + 4, "rock %d lod %d: %d triangles (<= %d)" % [v, lod, tris, ROCK_TRIANGLES[lod]])
	var clutter := _make_clutter()
	clutter.update_view(Vector2(500, 500), 0.5)
	var near := clutter.rock_stats()
	print("ga3_l2: rocks at d=10 %s, cells %d" % [near, clutter.visible_cells().size()])
	check(clutter.rock_variants() == meshes.size(), "clutter loads the rock variants")
	check(int(near["rocks"]) > 0 and int(near["lod"]) == 0, "rocks seeded on rocky ground, LOD0 close")
	check(int(near["draw_calls"]) <= clutter.visible_cells().size(), "at most one rock draw call per cell")
	# Semis sur la moitié rocheuse seulement (x > 500).
	for entry: Dictionary in clutter._cells.values():
		if entry.has("rocks"):
			for p: Vector2 in (entry["rocks"]["points"] as PackedVector2Array):
				if p.x <= 500.0:
					check(false, "rock outside the rocky half at %s" % p)
					break
	clutter.update_view(Vector2(500, 500), 2.0)
	var far := clutter.rock_stats()
	check(int(far["lod"]) == 2, "LOD2 at d=2 (VT3 1:1 range)")
	if int(far["rocks"]) > 0 and int(near["rocks"]) > 0:
		check(float(far["triangles"]) / float(far["rocks"]) < float(near["triangles"]) / float(near["rocks"]), "fewer triangles per rock far away")
	clutter.free()
	Ga3Vegetation.force(false)
	var fc := _make_clutter()
	fc.update_view(Vector2(500, 500), 0.5)
	check(fc.rock_variants() == 0 and int(fc.rock_stats()["rocks"]) == 0, "--no-ga3-veg: no rocks")
	check(fc.visible_instances() > 0, "--no-ga3-veg: grass still there")
	fc.free()
	Ga3Vegetation.force(true)


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
	var focus := Vector3(ORLEANS_FOREST.x, data.surface_world_at(ORLEANS_FOREST.x, ORLEANS_FOREST.y), ORLEANS_FOREST.y)
	Ga3Vegetation.force(false)
	vegetation.build(data)
	await _settle(rig, focus, 25.0, vegetation)
	var before := vegetation.lod_census()
	Ga3Vegetation.force(true)
	vegetation.build(data)
	await _settle(rig, focus, 25.0, vegetation)
	var after := vegetation.lod_census()
	print("ga3_l2: census d=25 FC %s" % before)
	print("ga3_l2: census d=25 GA3 %s" % after)
	check(vegetation.ga3_near_impostors, "GA3: near impostors active")
	check(int(after["near"]) == 0 and int(after["detailed"]) == 0 and int(after["low"]) == 0, "GA3: no leaf cards nor meshes for oak/beech/fir (d=25)")
	check(int(after["impostor"]) > 0, "GA3: impostors at d=25")
	var tri_before := int(before["near_triangles"]) + int(before["impostor_triangles"]) + int(before["detailed_triangles"]) + int(before["low_triangles"])
	var tri_after := int(after["near_triangles"]) + int(after["impostor_triangles"]) + int(after["detailed_triangles"]) + int(after["low_triangles"])
	print("ga3_l2: tree triangles d=25 FC %d -> GA3 %d" % [tri_before, tri_after])
	if int(before["near"]) > 0:
		check(tri_after < tri_before, "GA3: fewer tree triangles close up")
	map.queue_free()
	await process_frame
