class_name PolylineMesh
extends RefCounted

## Construit un ArrayMesh de rubans (quads) le long de polylignes en coordonnées
## carte, posés sur le relief échantillonné dans la heightmap. Un seul surface
## pour toutes les lignes = un seul appel de rendu.


static func build(lines: Array, widths: Array, map_data: MapData, lift: float) -> ArrayMesh:
	var vertices := PackedVector3Array()
	var indices := PackedInt32Array()
	for li in lines.size():
		var points: PackedVector2Array = lines[li]
		var width: float = widths[li]
		if points.size() < 2:
			continue
		var base := vertices.size()
		var count := points.size()
		for i in count:
			var p := points[i]
			var prev := points[maxi(i - 1, 0)]
			var next := points[mini(i + 1, count - 1)]
			var dir := (next - prev).normalized()
			if dir == Vector2.ZERO:
				dir = Vector2.RIGHT
			var perp := Vector2(-dir.y, dir.x) * (width * 0.5)
			var y := map_data.surface_world_at(p.x, p.y) + lift
			vertices.append(Vector3(p.x + perp.x, y, p.y + perp.y))
			vertices.append(Vector3(p.x - perp.x, y, p.y - perp.y))
		for i in count - 1:
			var a := base + i * 2
			indices.append_array([a, a + 1, a + 3, a, a + 3, a + 2])
	var mesh := ArrayMesh.new()
	if vertices.is_empty():
		return mesh
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_INDEX] = indices
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


static func flat_material(color: Color) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = color
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.cull_mode = BaseMaterial3D.CULL_DISABLED
	material.no_depth_test = true
	material.render_priority = 1
	return material
