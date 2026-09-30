extends SceneTree

## Lot GA3-L1 : décor de bataille généré (`data/art/ga3_decor.json`, `Ga3Kit`). Vérifie pour
## chaque variante branchée du manifeste `assets/models/props_ga/manifest.json` : les trois LOD
## chargent, triangles sous le plafond du catalogue (LOD1 < LOD0, LOD2 < LOD1), pied à y = 0 ;
## puis l'option : un lot du kit (`BuildingKit.Batch`) remplace une part des maisons par la
## variante GA3 (poignées, masquage, tuiles LOD), rien sous la neige ni avec `--no-ga3`.
## Usage : godot --headless --path game --script res://tests/ga3_l1_decor_test.gd [-- --no-ga3]



func _catalog() -> Dictionary:
	var path := ProjectSettings.globalize_path("res://").path_join("../data/art/ga3_decor.json")
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
	return parsed if parsed is Dictionary else {}


func _stats(root: Node) -> Dictionary:
	var tris := 0
	var box := AABB()
	var first := true
	for child in root.find_children("*", "MeshInstance3D", true, false):
		var mi := child as MeshInstance3D
		for s in mi.mesh.get_surface_count():
			var arrays := mi.mesh.surface_get_arrays(s)
			var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
			tris += indices.size() / 3 if indices.size() > 0 else (arrays[Mesh.ARRAY_VERTEX] as PackedVector3Array).size() / 3
		var local := mi.global_transform * mi.get_aabb() if mi.is_inside_tree() else mi.transform * mi.get_aabb()
		box = local if first else box.merge(local)
		first = false
	return {"triangles": tris, "aabb": box}


func _init() -> void:
	var ok := true
	var no_ga3 := OS.get_cmdline_user_args().has("--no-ga3")
	var caps: Dictionary = {}
	for entry in _catalog().get("objects", []):
		caps["ga3_" + str(entry["id"])] = int(entry["lod0"])
	if caps.is_empty():
		push_error("ga3_l1: catalogue data/art/ga3_decor.json introuvable")
		quit(1)
		return
	var manifest := Ga3Kit.manifest()
	var wired := 0
	for model_name in manifest:
		var entry: Dictionary = manifest[model_name]
		if not bool(entry.get("wired", false)):
			continue
		wired += 1
		var previous := 1 << 30
		for lod in 3:
			var path := Ga3Kit.DIR + "%s_lod%d.glb" % [model_name, lod]
			var scene := load(path) as PackedScene
			if scene == null:
				push_error("ga3_l1: %s introuvable" % path)
				ok = false
				continue
			var node := scene.instantiate()
			var stats := _stats(node)
			node.free()
			var box: AABB = stats["aabb"]
			var tris := int(stats["triangles"])
			print("ga3_l1: %s lod%d %d tri, %.2f x %.2f x %.2f m, pied y=%.2f" % [
				model_name, lod, tris, box.size.x, box.size.y, box.size.z, box.position.y])
			if lod == 0 and tris > int(caps.get(model_name, 3000)):
				push_error("ga3_l1: %s LOD0 %d tri > plafond %d" % [model_name, tris, caps.get(model_name, 3000)])
				ok = false
			if tris >= previous:
				push_error("ga3_l1: %s lod%d pas plus léger que le précédent" % [model_name, lod])
				ok = false
			previous = tris
			if absf(box.position.y) > 0.05:
				push_error("ga3_l1: %s lod%d pied à y=%.3f" % [model_name, lod, box.position.y])
				ok = false
		if Ga3Kit.mesh(model_name, 0) == null:
			push_error("ga3_l1: Ga3Kit.mesh(%s) nul" % model_name)
			ok = false
	if wired == 0:
		push_error("ga3_l1: aucune variante branchée dans le manifeste")
		ok = false

	# Option : 200 chaumières du kit sur une grille, variante GA3 pour une part d'entre elles.
	Ga3Kit.active = Ga3Kit.requested()
	var kit_model := ""
	for m in BuildingKit.models_of("cottage"):
		kit_model = m
		break
	var root := Node3D.new()
	get_root().add_child(root)
	for variant in ["", "snow"]:
		var batch := BuildingKit.Batch.new(variant, 1900.0)
		var handles: Array = []
		for i in 200:
			var xform := Transform3D(Basis(Vector3.UP, i * 0.37) * Basis.from_scale(BuildingKit.fit_scale(kit_model, 9.0, 5.5)), Vector3((i % 20) * 37.0, 0.0, (i / 20) * 41.0))
			handles.append(batch.add(kit_model, xform))
		batch.build(root)
		var ga3 := batch.ga3_count()
		print("ga3_l1: variante '%s', %d / %d chaumières GA3 (--no-ga3 %s)" % [variant, ga3, batch.count(), no_ga3])
		if batch.count() != 200:
			push_error("ga3_l1: %d instances au lieu de 200" % batch.count())
			ok = false
		var expected_some: bool = not no_ga3 and variant == "" and not Ga3Kit.variants_of("cottage").is_empty()
		if expected_some and (ga3 < 40 or ga3 > 160):
			push_error("ga3_l1: part GA3 %d hors de [40, 160] (share 0,5)" % ga3)
			ok = false
		if not expected_some and ga3 != 0:
			push_error("ga3_l1: %d variantes GA3 alors qu'elles sont coupées" % ga3)
			ok = false
		# Poignée GA3 : lecture, déplacement, masquage.
		for handle in handles:
			if str(handle[0]).begins_with("ga3_"):
				var before := batch.get_transform(handle)
				batch.hide(handle)
				var after := batch.get_transform(handle)
				if after.origin.y > before.origin.y - 40.0:
					push_error("ga3_l1: masquage d'une poignée GA3 sans effet")
					ok = false
				break
	var lod_nodes := 0
	for child in root.find_children("Ga3_*", "MultiMeshInstance3D", true, false):
		var mmi := child as MultiMeshInstance3D
		lod_nodes += 1
		if mmi.visibility_range_end <= mmi.visibility_range_begin:
			push_error("ga3_l1: portées LOD incohérentes sur %s" % mmi.name)
			ok = false
	print("ga3_l1: %d nœuds MultiMesh GA3 (tuiles × LOD)" % lod_nodes)
	if not no_ga3 and lod_nodes > 0 and lod_nodes % 3 != 0:
		push_error("ga3_l1: tuiles sans leurs trois LOD")
		ok = false
	root.free()
	Ga3Kit.active = false
	print("ga3_l1: %s" % ("OK" if ok else "ÉCHEC"))
	quit(0 if ok else 1)
