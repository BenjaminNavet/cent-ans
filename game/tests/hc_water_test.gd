extends SceneTree

## Test du lot HC2 (ADR 0161 §3, eaux lisibles à hauteur de jeu), sur les vraies données :
##  1. `data/map/lakes.json` (seuil de surface abaissé) : plus de lacs qu'avant HC2 (762), Léman
##     présent et maillé, coût de `LakesRenderer` borné (triangles, échecs de triangulation) ;
##  2. `terrain.gdshader` compile et expose les réglages de l'eau intérieure (couleurs des lacs,
##     étangs et mares généralisés) ; eau des lacs nettement plus claire que la forêt, bleue ;
##     nappe des lacs aux couleurs propres aux lacs ;
##  3. avec une fenêtre réelle seulement (sans `--headless`) : part de pixels d'eau et nombre de
##     nappes distinctes dues aux étangs dans la Dombes à rig 150 et 300, comptés par passage
##     magenta (`hc_water_view.gd`) — au dézoom les étangs restent des nappes, pas une teinte
##     moyenne — et eau plus bleue que le sol alentour.
## Usage : godot --headless --path game --script res://tests/hc_water_test.gd
##         godot --path game --resolution 640x400 --script res://tests/hc_water_test.gd --
##           --hide-armies --map-weather=clear        (contrôle chiffré sur image en plus)

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const VIEW := preload("res://tests/hc_water_view.gd")
const TERRAIN_SHADER := preload("res://shaders/terrain.gdshader")
const LANDCOVER_INCLUDE := "res://shaders/relief_landcover.gdshaderinc"
## Lacs de `lakes.json` avant HC2 (`min_area_px` 30).
const LAKES_BEFORE := 762
## Plafond de triangles de la nappe des lacs (≈ 15 500 mesurés au seuil 8).
const MAX_TRIANGLES := 30000
const POND_UNIFORMS := [
	"rl_water_ref_height", "rl_pond_cell_px", "rl_pond_min_px", "rl_pond_max_cell_px", "rl_pond_fill",
	"rl_pond_bank", "rl_pond_bank_amount", "rl_pond_far_cover", "rl_pool_min_px", "rl_pool_amount",
	"rl_pool_opacity", "rl_pool_max_scale", "rl_pool_lake_tint",
]
const LAKE_UNIFORMS := ["lake_color", "lake_shallow_color", "lake_bank_color", "lake_sky_color", "lake_sky_reflect"]
## Dombes, par distance de rig : nappes d'étang d'au moins `POND_BLOB_PX` pixels (image 640×400)
## et part d'image gagnées par rapport à la même vue sans étangs (`rl_pond_fill` = 0 ; les lacs de
## l'arrière-plan sont ainsi décomptés). À rig 150 la zone ne couvre que ≈ 100×40 px de l'image.
const DOMBES_CHECKS := [[150.0, 6, 0.0006], [300.0, 2, 0.0001]]
const POND_BLOB_PX := 6

var _failures := 0


func _init() -> void:
	await process_frame
	_check_lakes()
	_check_shader()
	if DisplayServer.get_name() == "headless":
		print("hc_water_test: image check skipped (headless, run with a window for it)")
	else:
		await _check_image()
	print("hc_water_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("hc_water_test: " + message)
	return condition


func _check_lakes() -> void:
	var map_data := MapData.load_from_dir(MAP_PATHS.default_data_dir().path_join("map"))
	if not _check(map_data.load_error == "", "map load failed: %s" % map_data.load_error):
		return
	var lakes := LakesRenderer.new()
	root.add_child(lakes)
	lakes.build(map_data)
	var stats := lakes.stats
	_check(int(stats.get("lakes", 0)) > LAKES_BEFORE, "no more lakes than before HC2: %s" % stats)
	_check(int(stats.get("failed", 0)) * 50 <= int(stats.get("lakes", 0)), "too many triangulation failures: %s" % stats)
	_check(int(stats.get("triangles", 0)) < MAX_TRIANGLES, "lake sheet too heavy: %s" % stats)
	var geneva := lakes.lake_named("Léman")
	if _check(not geneva.is_empty(), "Léman missing from lakes.json"):
		_check(int(geneva["triangles"]) > 10, "Léman not meshed: %s triangles" % geneva["triangles"])
	var small := 0
	for lake: Dictionary in lakes.lakes:
		if absf(LakesRenderer._signed_area(lake["polygon"])) < 30.0:
			small += 1
	_check(small > 100, "only %d lakes under 30 px²" % small)
	var material := lakes.water_material
	if _check(material != null, "no lake sheet material"):
		_check(material.get_shader_parameter("deep_color") == lakes.deep_color, "lake sheet deep colour not set")
		_check(material.get_shader_parameter("shallow_color") == lakes.shallow_color, "lake sheet shallow colour not set")
	print("hc_water_test: lakes %s, %d under 30 px²" % [JSON.stringify(stats), small])
	lakes.queue_free()


func _check_shader() -> void:
	var uniforms: Array[String] = []
	for uniform: Dictionary in TERRAIN_SHADER.get_shader_uniform_list():
		uniforms.append(str(uniform["name"]))
	# Liste vide si le shader ne compile pas.
	if not _check(not uniforms.is_empty(), "terrain.gdshader does not compile"):
		return
	for uniform_name: String in POND_UNIFORMS + LAKE_UNIFORMS:
		_check(uniforms.has(uniform_name), "terrain.gdshader lacks uniform %s" % uniform_name)
	# Valeurs par défaut lues dans le source (le serveur de rendu headless ne les rend pas).
	var lake := _default_of(TERRAIN_SHADER.code, "lake_color")
	var forest := _default_of(TERRAIN_SHADER.code, "tint_forest")
	if _check(lake.size() == 3 and forest.size() == 3, "lake_color / tint_forest defaults unreadable: %s %s" % [lake, forest]):
		var weights := Vector3(0.2126, 0.7152, 0.0722)
		var lake_v := Vector3(lake[0], lake[1], lake[2])
		var forest_v := Vector3(forest[0], forest[1], forest[2])
		_check(lake_v.dot(weights) > 1.8 * forest_v.dot(weights), "lake %s not clearly lighter than forest %s" % [lake_v, forest_v])
		_check(lake_v.z > lake_v.y and lake_v.y > lake_v.x, "lake %s is not blue-grey" % lake_v)
		print("hc_water_test: lake_color %s (luminance %.3f), tint_forest %s (luminance %.3f)" % [lake_v, lake_v.dot(weights), forest_v, forest_v.dot(weights)])
	var min_px := _default_of(FileAccess.get_file_as_string(LANDCOVER_INCLUDE), "rl_pond_min_px")
	_check(min_px.size() == 1 and min_px[0] >= 6.0, "rl_pond_min_px default %s under 6 px" % [min_px])


## Valeur par défaut d'un uniforme `float` ou `vec3` dans un source de shader (tableau vide sinon).
func _default_of(source: String, uniform_name: String) -> Array[float]:
	var regex := RegEx.new()
	regex.compile("uniform\\s+\\w+\\s+%s\\s*(?::[^=;]*)?=\\s*(?:vec3\\()?([^;)]*)\\)?\\s*;" % uniform_name)
	var found := regex.search(source)
	var values: Array[float] = []
	if found != null:
		for part in found.get_string(1).split(",", false):
			values.append(float(part.strip_edges()))
	return values


## Fenêtre réelle : la Dombes montre des nappes d'eau distinctes à rig 150 et encore à rig 300.
func _check_image() -> void:
	var map: Node3D = await VIEW.open_map(self)
	if not _check(map != null, "campaign map failed to load"):
		return
	for entry: Array in DOMBES_CHECKS:
		await VIEW.frame(self, map, VIEW.DOMBES, entry[0])
		var materials: Array[ShaderMaterial] = VIEW.terrain_materials(map)
		for material in materials:
			material.set_shader_parameter("rl_pond_fill", 0.0)
		var bare: Dictionary = await _stats(map)
		for material in materials:
			material.set_shader_parameter("rl_pond_fill", null)  # valeur par défaut du shader
		var stats: Dictionary = await _stats(map)
		var sheets := int(stats["blobs"]) - int(bare["blobs"])
		var share := float(stats["share"]) - float(bare["share"])
		print("hc_water_test: Dombes rig %d ponds: %d sheets, share %.4f (view %.4f, without ponds %.4f), water %s land %s" % [
			int(entry[0]), sheets, share, stats["share"], bare["share"], stats["core_rgb"], stats["land_rgb"]])
		_check(sheets >= int(entry[1]), "Dombes rig %d: %d pond sheets, %d wanted" % [int(entry[0]), sheets, entry[1]])
		_check(share > float(entry[2]), "Dombes rig %d: pond share %.4f under %.4f" % [int(entry[0]), share, entry[2]])
		var water: Vector3 = stats["core_rgb"]
		var land: Vector3 = stats["land_rgb"]
		_check(water.z > land.z * 1.5 and water.z > water.x * 1.2, "Dombes rig %d: water %s does not stand out from the land %s" % [int(entry[0]), water, land])
	map.queue_free()
	await process_frame


func _stats(map: Node3D) -> Dictionary:
	for i in 4:
		await process_frame
	var image: Image = await VIEW.grab(self)
	var mask: Image = await VIEW.water_mask(self, map)
	image.resize(640, 400, Image.INTERPOLATE_LANCZOS)
	mask.resize(640, 400, Image.INTERPOLATE_NEAREST)
	return VIEW.water_stats(image, mask, POND_BLOB_PX)
