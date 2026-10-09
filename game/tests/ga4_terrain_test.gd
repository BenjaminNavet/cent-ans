extends TestCase

## Lot GA4 (réduit par ADR 0244) : normale d'eau et uniformes GA4 posés sur les matériaux terrain
## et mer. Les tableaux du sol de campagne sont couverts par `tx_campaign_test.gd`.
## Usage : godot --headless --path game --script res://tests/ga4_terrain_test.gd
## Sans `--headless`, la boucle rend deux images avec les deux shaders (erreurs de compilation
## dans la sortie).


func _init() -> void:
	var spec := CampaignTextures.spec()
	if spec.is_empty():
		print("GA4: données absentes")
		failures += 1
		finish()
		return
	if spec.has("layers") or spec.has("albedo_size"):
		check(false, "GA4: les couches Poly Haven globales doivent avoir disparu (ADR 0244)")

	var water_tex := load(CampaignTextures.WATER_NORMAL_PATH) as Texture2D
	if water_tex == null:
		check(false, "GA4: normale d'eau absente")
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
		failures += 1

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

	finish()
