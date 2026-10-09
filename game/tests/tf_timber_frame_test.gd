extends TestCase

## Lot TF : colombage `TimberFrame` câblé et choix régional des maisons.
## - données : `wired: true`, couches 14 (torchis, `high`) et 15 (lattis, `low`) de l'atlas `Building` ; matériau atlas : couches unies
##   11..13 (`first_plain`..`plain_last`), la couche 14 reste texturée ;
## - modèles : au moins un modèle de bataille exporté (`manifest.json` : `framed`) porte des faces
##   de la couche 14 (alpha de la couleur de sommet) ; variantes du Midi présentes ;
## - région : Normandie (`france_nord`) surtout à colombage, Guyenne (`aquitaine`) presque jamais,
##   variantes du Midi seulement au Midi ;
## - château Kenney des cités : matériau atlas `Building`, faces d'ardoise (`RoofSlate`), un seul
##   maillage et une seule surface (pas d'appel de rendu en plus).
## Usage : godot --headless --path game --script res://tests/tf_timber_frame_test.gd


func _init() -> void:
	var layers := BuildingMaterials.atlas_layers()
	var index := layers.find("TimberFrame")
	check(index == 14, "TimberFrame est la couche 14 de l'atlas (trouvé %d)" % index)
	var doc: Variant = JSON.parse_string(FileAccess.get_file_as_string(ProjectSettings.globalize_path("res://").path_join("../data/art/building_materials.json")))
	var wired := false
	for entry in (doc as Dictionary)["textured"]:
		if str(entry["name"]) == "TimberFrame":
			wired = bool(entry.get("wired", true))
	check(wired, "TimberFrame marquée wired: true")
	var atlas := BuildingMaterials.material("Building") as ShaderMaterial
	var first_plain := int(atlas.get_shader_parameter("first_plain"))
	var plain_last := int(atlas.get_shader_parameter("plain_last"))
	check(first_plain == 11 and plain_last == 13, "couches unies 11..13 (trouvé %d..%d)" % [first_plain, plain_last])
	check(index > plain_last, "TimberFrame après le bloc uni : texturée")
	var far_index := layers.find("TimberFrameFar")
	check(far_index == 15, "TimberFrameFar est la couche 15 (trouvé %d)" % far_index)
	check(str(BuildingMaterials.SPECS["TimberFrame"][0]).contains("daub"), "TimberFrame (niveau high) : torchis sans poutres peintes")
	var albedo := atlas.get_shader_parameter("albedo_array") as TextureLayered
	check(albedo != null and albedo.get_layers() == layers.size(), "tableau d'albédos à %d tranches (trouvé %d)" % [layers.size(), albedo.get_layers() if albedo != null else -1])

	# Modèles exportés : faces de la couche 14 dans un modèle marqué `framed`.
	var manifest := BuildingKit.manifest()
	var framed: Array = []
	var southern := 0
	for model_name in manifest:
		var entry: Dictionary = manifest[model_name]
		if bool(entry.get("framed", false)):
			framed.append(model_name)
		if bool(entry.get("southern", false)):
			southern += 1
	check(framed.size() >= 1, "%d modèles de bataille à colombage" % framed.size())
	check(southern >= 3, "%d variantes du Midi" % southern)
	if not framed.is_empty():
		check(_count_layer("res://assets/models/buildings/%s.glb" % framed[0], 14) > 0, "%s : faces TimberFrame (couche 14)" % framed[0])
		check(_count_layer("res://assets/models/buildings/%s.glb" % framed[0], 15) == 0, "%s : pas de lattis peint sous les poutres modelées" % framed[0])
	check(_count_layer("res://assets/models/town_kit/timber_0.glb", 15) > 0, "kit des villes (low) : lattis peint TimberFrameFar")

	# Choix régional.
	var north := BuildingRegions.style_for_province("prov_normandie")
	var south := BuildingRegions.style_for_province("prov_guyenne")
	check(BuildingRegions.region_of("prov_normandie") == "france_nord", "Normandie → france_nord")
	check(float(north.get("framed", 0)) >= 0.7 and not bool(north.get("southern", true)), "Normandie : colombage courant")
	check(float(south.get("framed", 1)) <= 0.1 and bool(south.get("southern", false)), "Guyenne : pierre et enduit (Midi)")
	var north_share := _framed_share(north)
	var south_share := _framed_share(south)
	check(north_share > 0.6, "Normandie : %.0f %% de maisons à colombage" % (north_share * 100.0))
	check(south_share < 0.2, "Guyenne : %.0f %% de maisons à colombage" % (south_share * 100.0))
	check(_southern_picks(north) == 0, "pas de variante du Midi en Normandie")
	check(_southern_picks(south) > 0, "variantes du Midi en Guyenne")
	BuildingKit.region_style = {}

	# Château Kenney : atlas, ardoise, une surface.
	var castle := SettlementGrowth.kenney_castle()
	check(castle != null, "château Kenney construit")
	if castle != null:
		var mesh: Mesh = castle.mesh
		check(mesh.get_surface_count() == 1, "château : une seule surface")
		check(mesh.surface_get_material(0) == BuildingMaterials.material("Building", "far"), "château : matériau atlas partagé")
		check(_mesh_layer_count(mesh, layers.find("RoofSlate")) > 0, "château : toits d'ardoise (RoofSlate)")
		check(_mesh_layer_count(mesh, layers.find("Masonry")) > 0, "château : murs de pierre (Masonry)")
		castle.free()
	finish()


## Part des maisons à colombage tirées pour un style (types régionaux, 300 tirages).
func _framed_share(style: Dictionary) -> float:
	BuildingKit.region_style = style
	var manifest := BuildingKit.manifest()
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	var framed := 0
	var total := 0
	for i in 300:
		var kind: String = BuildingKit.REGIONAL_KINDS[i % 3]
		var model := BuildingKit.pick(kind, 9.0, 6.5, rng)
		if model == "":
			continue
		total += 1
		if bool((manifest[model] as Dictionary).get("framed", false)):
			framed += 1
	return float(framed) / maxf(total, 1.0)


func _southern_picks(style: Dictionary) -> int:
	BuildingKit.region_style = style
	var manifest := BuildingKit.manifest()
	var rng := RandomNumberGenerator.new()
	rng.seed = 11
	var count := 0
	for i in 120:
		var model := BuildingKit.pick(BuildingKit.REGIONAL_KINDS[i % 3], 9.0, 6.5, rng)
		if model != "" and bool((manifest[model] as Dictionary).get("southern", false)):
			count += 1
	return count


func _count_layer(path: String, layer: int) -> int:
	var scene := load(path) as PackedScene
	if scene == null:
		return 0
	var node := scene.instantiate()
	var count := 0
	for child in node.find_children("*", "MeshInstance3D", true, false):
		count += _mesh_layer_count((child as MeshInstance3D).mesh, layer)
	node.free()
	return count


## Sommets dont l'alpha de couleur code la couche `layer` ((indice + 0,5) / 16).
func _mesh_layer_count(mesh: Mesh, layer: int) -> int:
	var count := 0
	for surface in mesh.get_surface_count():
		var arrays := mesh.surface_get_arrays(surface)
		var colors: Variant = arrays[Mesh.ARRAY_COLOR]
		if colors == null:
			continue
		for c: Color in colors as PackedColorArray:
			if clampi(int(c.a * 16.0), 0, 15) == layer:
				count += 1
	return count
