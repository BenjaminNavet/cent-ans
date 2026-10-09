extends SceneTree

## DN-forets (essai) : compare les forêts actuelles (arbres généralisés HC1) à des massifs générés
## d'un seul tenant (un glb = un bloc de bois, `tools/experiments/dn_forest_patch.py`) posés sur le
## masque forestier. Pour chaque vue et distance : `<vue>_d<D>_trees.png` puis `<vue>_d<D>_patch.png`.
## Usage : tools/godot_bg.sh --path game --resolution 960x600 \
##   --script res://tests/dn_forest_patch_shot.gd -- --out=<dossier> [--patch-dir=<dn/forets>] \
##   [--distances=20,40,80] [--patch-width=2.4] [--only=orleans]
## Les glb sont lus hors dépôt (GLTFDocument, sans import) ; les PNG ne sont jamais commités.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
# nom, lon, lat, massif
const VIEWS := [
	["orleans", 2.10, 47.97, "forest_broadleaf"],
	["vosges", 7.05, 48.25, "forest_conifer"],
	["maures", 6.35, 43.28, "forest_mediterranean"],
]
## Seuil de couverture forestière au cœur des massifs ; les lisières tirent un seuil aléatoire.
const CORE_FOREST := 0.55


func _init() -> void:
	var out := CmdArgs.value("--out", "user://dn_forest_patch")
	var patch_dir := CmdArgs.value("--patch-dir", OS.get_environment("HOME").path_join("dev/cent-ans-raw/dn"))
	var distances := CmdArgs.value("--distances", "20,40,80").split(",")
	var patch_width := float(CmdArgs.value("--patch-width", "2.4"))
	var only := CmdArgs.value("--only", "")
	await process_frame
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
	for _i in 5:
		await process_frame
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var data: MapData = map.get("map_data")
	var vegetation: Vegetation = map.get_node("Vegetation")
	DirAccess.make_dir_recursive_absolute(out)
	for view in VIEWS:
		if only != "" and not String(view[0]).contains(only):
			continue
		var px := FaunaLayer.lonlat_to_px(float(view[1]), float(view[2]), data)
		var focus := Vector3(px.x, data.surface_world_at(px.x, px.y), px.y)
		var patch := _load_patch(patch_dir.path_join(String(view[3])))
		if patch.is_empty():
			print("dn_forest_patch_shot: no glb for ", view[3], " in ", patch_dir)
			continue
		for dist_text in distances:
			var distance := float(dist_text)
			# Semis limité au champ utile de la distance (le GPU décroche au-delà de ~20 M triangles).
			var layer := _scatter(patch, data, vegetation.mask, px, distance * 1.2, patch_width)
			map.add_child(layer)
			rig.look_at_point(focus, distance)
			rig.snap()
			for mode in ["trees", "patch"]:
				var patch_mode: bool = mode == "patch"
				layer.visible = patch_mode
				vegetation.visible = not patch_mode
				if vegetation.forest_detail != null:
					vegetation.forest_detail.visible = not patch_mode
				for _i in 90:
					await process_frame
				await RenderingServer.frame_post_draw
				var path := out.path_join("%s_d%s_%s.png" % [view[0], dist_text, mode])
				root.get_texture().get_image().save_png(path)
				print("dn_forest_patch_shot: ", path, " dist=", rig.distance, " patches=", layer.multimesh.instance_count)
			layer.queue_free()
	map.queue_free()
	await process_frame
	quit(0)


## glb du dossier `<massif>/3d/` (version décimée `lod_*` d'abord, `tools/blender_scripts/dn_forest_patch_lod.py`) :
## {mesh, aabb}.
func _load_patch(dir: String) -> Dictionary:
	var files := Array(DirAccess.get_files_at(dir.path_join("3d")))
	files.sort_custom(func(a: String, b: String) -> bool: return a.begins_with("lod_") and not b.begins_with("lod_"))
	for file: String in files:
		if not file.ends_with(".glb"):
			continue
		var document := GLTFDocument.new()
		var state := GLTFState.new()
		if document.append_from_file(dir.path_join("3d").path_join(file), state) != OK:
			continue
		var scene := document.generate_scene(state)
		var instances := scene.find_children("*", "MeshInstance3D", true, false)
		if instances.is_empty():
			continue
		var instance := instances[0] as MeshInstance3D
		var mesh := instance.mesh
		var aabb := instance.transform * mesh.get_aabb()
		scene.free()
		print("dn_forest_patch_shot: patch ", file, " aabb=", aabb)
		return {"mesh": mesh, "aabb": aabb}
	return {}


## Massifs sur une grille au pas de la largeur du bloc (décalée, tournée, taille variable) là où
## la couverture forestière le veut, dans un rayon `radius` autour de `center`.
func _scatter(patch: Dictionary, data: MapData, mask: VegetationMask, center: Vector2, radius: float, width: float) -> MultiMeshInstance3D:
	var aabb: AABB = patch["aabb"]
	var fit := width / maxf(aabb.size.x, aabb.size.z)
	var noise := VegetationMask.make_noise()
	var rng := RandomNumberGenerator.new()
	rng.seed = 1337
	var step := width * 0.7
	var transforms: Array[Transform3D] = []
	var y := center.y - radius
	while y <= center.y + radius:
		var x := center.x - radius
		while x <= center.x + radius:
			var jitter := Vector2(rng.randf_range(-0.3, 0.3), rng.randf_range(-0.3, 0.3)) * step
			var at := Vector2(x, y) + jitter
			var forest: float = mask.sample(at.x, at.y, noise)["forest"]
			if forest > lerpf(CORE_FOREST - 0.25, CORE_FOREST + 0.2, rng.randf()) and at.distance_to(center) <= radius:
				var size := fit * rng.randf_range(0.8, 1.15)
				var basis := Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3(size, size, size))
				# Socle centré sur le point, enfoncé de 8 % de la hauteur (pentes).
				var offset := basis * Vector3(-aabb.get_center().x, -aabb.position.y - aabb.size.y * 0.08, -aabb.get_center().z)
				transforms.append(Transform3D(basis, Vector3(at.x, data.surface_world_at(at.x, at.y), at.y) + offset))
			x += step
		y += step
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.mesh = patch["mesh"]
	multimesh.instance_count = transforms.size()
	for index in transforms.size():
		multimesh.set_instance_transform(index, transforms[index])
	var node := MultiMeshInstance3D.new()
	node.multimesh = multimesh
	node.visible = false
	return node
