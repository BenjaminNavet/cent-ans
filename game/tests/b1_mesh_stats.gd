extends SceneTree

## Lot B1 : nombre de triangles des figurines (maillage complet et allégé), par famille et
## variante. `godot --headless --path game --script res://tests/b1_mesh_stats.gd`


func _init() -> void:
	for kind in ["infantry", "archer", "cavalry"]:
		for variant in 3:
			var full := BattleMeshes.soldier(kind, variant)
			var lod := BattleMeshes.soldier(kind, variant, true)
			print("%s/%d: %d triangles, lod %d" % [kind, variant, _triangles(full), _triangles(lod)])
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
