extends SceneTree

## Lot GA4 : mémoire des `Texture2DArray` du terrain de campagne (albédo 2k, normale +
## rugosité 2k, compressés en VRAM + mipmaps) comparée à l'ancien chemin (JPEG 1k par couche,
## RGBA8 non compressé), cohérence des moyennes des données avec les textures, normale d'eau et
## uniformes GA4 posés sur les matériaux terrain et mer. Modèle : `ga2_ground_test.gd`.
## Usage : godot --headless --path game --script res://tests/ga4_terrain_test.gd
## Sans `--headless`, la boucle rend deux images avec les deux shaders (erreurs de compilation
## dans la sortie).

const CEILING_MB := 60.0


func _bytes_per_texel(fmt: int) -> float:
	# Format VRAM réel mesuré (cf. GA2 : DXT1/BC1 pour les couches sans alpha).
	if fmt == Image.FORMAT_BPTC_RGBA or fmt == Image.FORMAT_ASTC_4x4 or fmt == Image.FORMAT_DXT3 or fmt == Image.FORMAT_DXT5 or fmt == Image.FORMAT_RGTC_RG:
		return 1.0
	if fmt == Image.FORMAT_DXT1 or fmt == Image.FORMAT_RGTC_R or fmt == Image.FORMAT_ETC2_RGB8:
		return 0.5
	return 4.0


func _init() -> void:
	var ok := true
	var spec := CampaignTextures.spec()
	if spec.is_empty():
		print("GA4: données absentes")
		quit(1)
		return
	print("GA4 couches : %s" % [CampaignTextures.layer_ids()])
	var arrays := CampaignTextures.load_arrays()
	if arrays.is_empty():
		print("GA4: tableaux indisponibles")
		quit(1)
		return
	var total := 0
	for key in ["albedo", "normal"]:
		var arr := arrays[key] as TextureLayered
		var bytes := int(arr.get_width() * arr.get_height() * arr.get_layers() * _bytes_per_texel(arr.get_format()) * 4.0 / 3.0)
		total += bytes
		print("GA4 %s : %dx%d x%d, format %d, %.2f Mo (mipmaps compris)" % [key, arr.get_width(), arr.get_height(), arr.get_layers(), arr.get_format(), bytes / 1048576.0])
		var expected := int(spec["albedo_size"] if key == "albedo" else spec["normal_size"])
		if arr.get_width() != expected or arr.get_layers() != 7:
			print("GA4: %s attend %d px x 7 couches" % [key, expected])
			ok = false
	var legacy := int(1024 * 1024 * 4 * 7 * 2 * 4.0 / 3.0)
	print("GA4 mémoire du terrain : %.2f Mo (ancien chemin 1k RGBA8 : %.2f Mo)" % [total / 1048576.0, legacy / 1048576.0])
	if total > CEILING_MB * 1048576 or total > legacy:
		print("GA4: mémoire au-dessus du plafond ou de l'ancien chemin")
		ok = false

	# Moyennes des données contre la source empilée (dernier mipmap de chaque tranche, comme
	# l'ancien chemin de `TerrainBuilder`) : le tableau importé ne garde pas ses pixels côté CPU.
	var albedo_arr := arrays["albedo"] as TextureLayered
	var means: PackedVector3Array = arrays["means"]
	var sheet := Image.load_from_file(ProjectSettings.globalize_path(CampaignTextures.ALBEDO_ARRAY_PATH))
	if sheet == null or sheet.is_empty():
		print("GA4: source de l'albédo illisible")
		ok = false
	else:
		sheet.convert(Image.FORMAT_RGBA8)
		var side := albedo_arr.get_width()
		for i in albedo_arr.get_layers():
			var image := sheet.get_region(Rect2i(0, i * side, side, side))
			image.generate_mipmaps()
			var last := image.get_mipmap_count()
			var offset := image.get_mipmap_offset(last)
			var data := image.get_data()
			var c := Color8(data[offset], data[offset + 1], data[offset + 2]).srgb_to_linear()
			var measured := Vector3(c.r, c.g, c.b)
			print("GA4 moyenne %s : données %s, texture %s" % [CampaignTextures.layer_ids()[i], means[i], measured])
			if (measured - means[i]).length() > 0.03 or means[i].length() <= 0.0:
				print("GA4: moyenne de %s incohérente (relancer `cent-ans geo textures`)" % CampaignTextures.layer_ids()[i])
				ok = false

	var water_tex := load(CampaignTextures.WATER_NORMAL_PATH) as Texture2D
	if water_tex == null:
		print("GA4: normale d'eau absente")
		ok = false
	else:
		var wb := int(water_tex.get_width() * water_tex.get_height() * 1.0 * 4.0 / 3.0)
		print("GA4 normale d'eau : %dx%d, %.2f Mo" % [water_tex.get_width(), water_tex.get_height(), wb / 1048576.0])

	var terrain_mat := ShaderMaterial.new()
	terrain_mat.shader = load("res://shaders/terrain.gdshader")
	CampaignTextures.apply_terrain(terrain_mat)
	var water_mat := ShaderMaterial.new()
	water_mat.shader = load("res://shaders/water.gdshader")
	CampaignTextures.apply_water(water_mat)
	print("GA4 uniformes : terrain ga4_on=%s tile_screen_px=%s, mer ga4_on=%s normale=%s" % [
		terrain_mat.get_shader_parameter("ga4_on"), terrain_mat.get_shader_parameter("tile_screen_px"),
		water_mat.get_shader_parameter("ga4_on"), water_mat.get_shader_parameter("water_normal") != null])
	if float(terrain_mat.get_shader_parameter("ga4_on")) < 0.5 or float(water_mat.get_shader_parameter("ga4_on")) < 0.5:
		ok = false

	# Rendu (hors headless) : deux plans avec les deux shaders pour forcer leur compilation.
	var world := Node3D.new()
	root.add_child(world)
	var cam := Camera3D.new()
	world.add_child(cam)
	cam.look_at_from_position(Vector3(0, 5, 5), Vector3.ZERO)
	for mat in [terrain_mat, water_mat]:
		var plane := MeshInstance3D.new()
		plane.mesh = PlaneMesh.new()
		plane.material_override = mat
		world.add_child(plane)
	await process_frame
	await process_frame

	print("GA4 OK" if ok else "GA4 ECHEC")
	quit(0 if ok else 1)
