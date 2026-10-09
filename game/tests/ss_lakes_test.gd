extends TestCase

## Test headless du lot SS3 (ADR 0142 §4, lacs maillés), sur les vraies données :
##  1. `data/map/lakes.json` chargé par `LakesRenderer`, au moins un lac maillé, dont le Léman ;
##  2. niveau cohérent avec le terrain affiché (`MapData.height_m_at` au centre de masse : Léman
##     à `LEVEL_TOLERANCE_M` près, 90 % des lacs aussi — le centre d'un lac en croissant ou d'un
##     bassin d'une chaîne de lacs peut tomber hors de sa nappe) ; aucun lac sous le niveau de la
##     mer ; centre du Léman sur l'eau ;
##  3. matériau `river_water.gdshader` en mode nappe ; visible en vue 3D, masqué sur le parchemin.
## Usage : godot --headless --path game --script res://tests/ss_lakes_test.gd

## Écart toléré entre le niveau du lac et l'altitude du terrain au centre (m).
const LEVEL_TOLERANCE_M := 5.0


func _init() -> void:
	await process_frame
	_run()
	finish()


func _run() -> void:
	var map_dir := MAP_PATHS.default_data_dir().path_join("map")
	var map_data := MapData.load_from_dir(map_dir)
	if not check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var lakes := LakesRenderer.new()
	root.add_child(lakes)
	lakes.build(map_data)
	var stats := lakes.stats
	check(int(stats.get("meshed", 0)) >= 1, "no lake meshed: %s" % stats)
	check(int(stats.get("failed", 0)) * 50 <= int(stats.get("lakes", 0)), "too many triangulation failures: %s" % stats)
	check(lakes.mesh != null and lakes.mesh.get_surface_count() == 1, "lake mesh missing")

	var geneva := lakes.lake_named("Léman")
	if check(not geneva.is_empty(), "Léman missing from lakes.json"):
		check(int(geneva["triangles"]) > 10, "Léman not meshed: %s triangles" % geneva["triangles"])
		var center: Vector2 = geneva["center"]
		check(not map_data.is_land_px(int(center.x), int(center.y)), "Léman centre on land %s" % center)
		check(absf(float(geneva["level_m"]) - 372.0) < 20.0, "Léman level %.1f m (real 372 m)" % geneva["level_m"])
		var gap := absf(map_data.height_m_at(center.x, center.y) - float(geneva["level_m"]))
		check(gap < LEVEL_TOLERANCE_M, "Léman level off the terrain by %.1f m" % gap)

	var worst := 0.0
	var worst_id := ""
	var checked := 0
	var close := 0
	for lake: Dictionary in lakes.lakes:
		check(float(lake["level_m"]) > 0.0, "%s under sea level (%.1f m)" % [lake["id"], lake["level_m"]])
		var center: Vector2 = lake["center"]
		if map_data.is_land_px(int(center.x), int(center.y)):
			continue  # centre de masse hors de l'eau (lac en croissant)
		checked += 1
		var gap := absf(map_data.height_m_at(center.x, center.y) - float(lake["level_m"]))
		if gap < LEVEL_TOLERANCE_M:
			close += 1
		if gap > worst:
			worst = gap
			worst_id = lake["id"]
	check(checked * 2 > lakes.lakes.size(), "too few lake centres on water: %d" % checked)
	check(close * 10 >= checked * 9, "only %d / %d lake levels match the terrain" % [close, checked])

	var material := lakes.material_override as ShaderMaterial
	if check(material != null, "no water material"):
		check(material.shader.resource_path.ends_with("river_water.gdshader"), "not river_water.gdshader")
		check(material.get_shader_parameter("sheet") == true, "sheet mode off")
		var uniforms: Array[String] = []
		for uniform: Dictionary in material.shader.get_shader_uniform_list():
			uniforms.append(str(uniform["name"]))
		# Liste vide si le shader ne compile pas (erreur de syntaxe du mode nappe).
		check(uniforms.has("sheet") and uniforms.has("min_px"), "river_water.gdshader uniforms: %s" % [uniforms])
	lakes.update_view(0.0)
	check(lakes.visible, "hidden in the 3D view")
	lakes.update_view(1.0)
	check(not lakes.visible, "visible on the parchment")
	print("ss_lakes_test: %s, %d / %d centres within %.0f m, worst gap %.1f m (%s)" % [JSON.stringify(stats), close, checked, LEVEL_TOLERANCE_M, worst, worst_id])
	lakes.queue_free()
