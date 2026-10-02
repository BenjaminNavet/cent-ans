extends RefCounted

## Lot HC2 (ADR 0161 §3) : outils communs à `hc_water_test.gd` et `hc_water_shots.gd` — carte de
## campagne instanciée, caméra posée sur une vue, et mesure de la part d'eau intérieure d'une image
## sans la lire : les couleurs d'eau (lacs, étangs, mares, nappe des lacs) passent en magenta le
## temps d'une image, on compte les pixels magenta, puis les couleurs d'origine sont rétablies.
## Fleuves et mer ne sont pas comptés (autres matériaux).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
## Centres des zones de `data/map/wetlands.json` (lon/lat) convertis en px carte par
## `MapGrid.lonlat_to_pixel` (tools/cent_ans_tools/geo/project.py) : Dombes (5,05 ; 45,99),
## Sologne (1,90 ; 47,55), Brenne (1,20 ; 46,72), Fens (0,10 ; 52,60, centre du polygone).
const DOMBES := Vector2(2458.8, 3669.7)
const SOLOGNE := Vector2(2145.8, 3400.1)
const BRENNE := Vector2(2058.3, 3519.2)
const FENS := Vector2(2062.8, 2601.9)
## Uniformes de couleur d'eau du terrain (lacs, étangs, mares) et de la nappe des lacs.
const TERRAIN_WATER_COLORS := ["lake_color", "lake_shallow_color", "lake_sky_color", "rl_marsh_water"]
const SHEET_WATER_COLORS := ["shallow_color", "deep_color", "sky_color", "foam_color"]
const MARK := Vector3(1.0, 0.0, 1.0)


## Instancie la carte de campagne (France, graine fixe, tutoriel coupé) ; null si le chargement échoue.
static func open_map(tree: SceneTree) -> Node3D:
	var root := tree.root
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await tree.process_frame
	if not map.get("load_ok"):
		return null
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	return map


## Matériaux `terrain.gdshader` de la carte (principal et tuiles).
static func terrain_materials(map: Node3D) -> Array[ShaderMaterial]:
	var found: Array[ShaderMaterial] = []
	var terrain: TerrainBuilder = map.get("terrain")
	if terrain != null and terrain.material != null:
		found.append(terrain.material)
	for node in map.find_children("*", "MeshInstance3D", true, false):
		var material := (node as MeshInstance3D).material_override as ShaderMaterial
		if material != null and material.shader != null and material.shader.resource_path.ends_with("terrain.gdshader") and not found.has(material):
			found.append(material)
	return found


## Pose la caméra sur le point `at` (px carte) à la distance de rig `distance`, puis attend le
## terrain fin et l'occupation du sol. Le brouillard de guerre est coupé (vues hors de France).
static func frame(tree: SceneTree, map: Node3D, at: Vector2, distance: float) -> void:
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.get("map_data")
	var focus := Vector3(at.x, data.surface_world_at(at.x, at.y), at.y)
	rig.look_at_point(focus, maxf(distance, rig.min_distance_at(focus)))
	rig.target_yaw = 0.0
	rig.snap()
	var terrain: TerrainBuilder = map.get("terrain")
	for i in 30:
		await tree.process_frame
	var guard := 0
	while guard < 1500:
		guard += 1
		var busy := ReliefLandcover.pending()
		if terrain != null and (not terrain.fine_ready() or terrain.pending_rescales() > 0):
			busy = true
		if not busy:
			break
		await tree.process_frame
	for i in 40:
		for material in terrain_materials(map):
			material.set_shader_parameter("fog_enabled", false)
		await tree.process_frame


## Image de la fenêtre après le prochain rendu.
static func grab(tree: SceneTree) -> Image:
	await RenderingServer.frame_post_draw
	return tree.root.get_viewport().get_texture().get_image()


## Pixels écran par px carte au point visé (axe est-ouest, axe nord-sud raccourci par le tangage).
static func screen_px_per_map_px(tree: SceneTree, map: Node3D, at: Vector2) -> Vector2:
	var data: MapData = map.get("map_data")
	var camera := tree.root.get_viewport().get_camera_3d()
	var y := data.surface_world_at(at.x, at.y)
	var origin := camera.unproject_position(Vector3(at.x, y, at.y))
	var east := camera.unproject_position(Vector3(at.x + 1.0, y, at.y))
	var south := camera.unproject_position(Vector3(at.x, y, at.y + 1.0))
	return Vector2(origin.distance_to(east), origin.distance_to(south))


## Masque des pixels d'eau intérieure de la vue courante (L8 : 255 = eau), par le passage magenta.
static func water_mask(tree: SceneTree, map: Node3D) -> Image:
	var saved := []
	for material in terrain_materials(map):
		for key: String in TERRAIN_WATER_COLORS:
			saved.append([material, key, material.get_shader_parameter(key)])
			material.set_shader_parameter(key, MARK)
		saved.append([material, "lake_bank_color", material.get_shader_parameter("lake_bank_color")])
	var lakes: LakesRenderer = map.get("lakes")
	if lakes != null and lakes.water_material != null:
		for key: String in SHEET_WATER_COLORS:
			saved.append([lakes.water_material, key, lakes.water_material.get_shader_parameter(key)])
			lakes.water_material.set_shader_parameter(key, Color(1.0, 0.0, 1.0))
	for i in 4:
		await tree.process_frame
	var marked := await grab(tree)
	for entry: Array in saved:
		(entry[0] as ShaderMaterial).set_shader_parameter(entry[1], entry[2])
	for i in 4:
		await tree.process_frame
	var mask := Image.create(marked.get_width(), marked.get_height(), false, Image.FORMAT_L8)
	for y in marked.get_height():
		for x in marked.get_width():
			var c := marked.get_pixel(x, y)
			if c.r > 0.25 and c.r > c.g * 1.6 and c.b > c.g * 1.6:
				mask.set_pixel(x, y, Color.WHITE)
	return mask


## Statistiques d'une vue : part d'eau, couleur moyenne (sRGB 0-255) de l'eau, du cœur des nappes et du reste, plus
## grande nappe (px), nombre de nappes d'au moins `min_blob_px` pixels (composantes 4-connexes).
static func water_stats(image: Image, mask: Image, min_blob_px: int = 12) -> Dictionary:
	var w := mask.get_width()
	var h := mask.get_height()
	var water := Vector3.ZERO
	var land := Vector3.ZERO
	var core := Vector3.ZERO
	var n_core := 0
	var n_water := 0
	var n_land := 0
	var wet := PackedByteArray()
	wet.resize(w * h)
	for y in h:
		for x in w:
			var c := image.get_pixel(x, y)
			var v := Vector3(c.r, c.g, c.b) * 255.0
			if mask.get_pixel(x, y).r > 0.5:
				water += v
				n_water += 1
				wet[y * w + x] = 1
				# Cœur de nappe : les quatre voisins sont aussi de l'eau (couleur sans mélange de bord).
				if x > 0 and y > 0 and x < w - 1 and y < h - 1 and mask.get_pixel(x - 1, y).r > 0.5 and mask.get_pixel(x + 1, y).r > 0.5 \
						and mask.get_pixel(x, y - 1).r > 0.5 and mask.get_pixel(x, y + 1).r > 0.5:
					core += v
					n_core += 1
			else:
				land += v
				n_land += 1
	var blobs := 0
	var biggest := 0
	var stack := PackedInt32Array()
	for start in w * h:
		if wet[start] != 1:
			continue
		var size := 0
		stack.append(start)
		wet[start] = 2
		while not stack.is_empty():
			var index := stack[stack.size() - 1]
			stack.remove_at(stack.size() - 1)
			size += 1
			var x := index % w
			for next: int in [index - 1 if x > 0 else -1, index + 1 if x < w - 1 else -1, index - w, index + w]:
				if next >= 0 and next < w * h and wet[next] == 1:
					wet[next] = 2
					stack.append(next)
		if size >= min_blob_px:
			blobs += 1
		biggest = maxi(biggest, size)
	return {
		"share": float(n_water) / maxf(float(w * h), 1.0),
		"water_rgb": (water / maxf(n_water, 1.0)).round(),
		"core_rgb": (core / maxf(n_core, 1.0)).round(),
		"land_rgb": (land / maxf(n_land, 1.0)).round(),
		"blobs": blobs,
		"biggest_px": biggest,
		"size": Vector2i(w, h),
	}
