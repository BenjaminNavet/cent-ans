extends SceneTree

## Lot B1 : nombre de triangles des figurines (maillage complet et allégé), par famille et
## variante. `godot --headless --path game --script res://tests/b1_mesh_stats.gd`


func _init() -> void:
	for kind in ["infantry", "archer", "cavalry"]:
		for variant in 3:
			var counts := []
			for level in 3:
				counts.append(_triangles(BattleMeshes.soldier_level(kind, variant, level)))
			print("%s/%d: %d / %d / %d triangles (complet / moyen / lointain)" % [kind, variant, counts[0], counts[1], counts[2]])
	# Contrôle du format : teint (SKIN, linéaire ≈ 0.604, 0.319, 0.187), membres et pivots.
	var arrays := BattleMeshes.soldier_level("archer", 0, 0).surface_get_arrays(0)
	var colors: PackedColorArray = arrays[Mesh.ARRAY_COLOR]
	var custom: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
	var seen := {}
	for i in colors.size():
		var key := "%d/%d" % [int(custom[i * 4]), roundi(colors[i].a * 5.0)]
		if not seen.has(key):
			seen[key] = true
			print("part %d code %d color %s pivot (%.2f, %.2f) bend %.2f" % [int(custom[i * 4]), roundi(colors[i].a * 5.0), colors[i], custom[i * 4 + 1], custom[i * 4 + 2], custom[i * 4 + 3]])
	quit(0)


static func _triangles(mesh: Mesh) -> int:
	var total := 0
	for s in mesh.get_surface_count():
		var arrays := mesh.surface_get_arrays(s)
		var indices = arrays[Mesh.ARRAY_INDEX]
		if indices != null and (indices as PackedInt32Array).size() > 0:
			total += (indices as PackedInt32Array).size() / 3
		else:
			total += (arrays[Mesh.ARRAY_VERTEX] as PackedVector3Array).size() / 3
	return total
