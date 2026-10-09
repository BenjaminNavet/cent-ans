class_name DnWaterBirds
extends RefCounted

## Lot DN-FLEUVE : oiseaux d'eau en glb générés (mouettes, oies, cigognes, grues, cygnes,
## pélicans). Le maillage est cuit à plat, orientation d'origine du glb (envergure sur X, bec vers
## -Z comme le V de `MapBirdFlocks`), grande dimension horizontale ramenée à `length` du registre
## `data/art/dn_water_models.json`, centré en X/Z, pied à y = 0. Le battement d'ailes et la
## trajectoire restent ceux du shader `life_birds.gdshader` (qui échantillonne la texture d'albédo).

static var _cache: Dictionary = {}


static func clear_cache() -> void:
	_cache = {}


## {mesh: ArrayMesh, texture: Texture2D ou null}, {} si l'identifiant ou le glb manque.
static func baked(id: String, lod: int) -> Dictionary:
	var key := "%s|%d" % [id, lod]
	if _cache.has(key):
		return _cache[key]
	var result := _bake(id, lod)
	_cache[key] = result
	return result


static func _bake(id: String, lod: int) -> Dictionary:
	var entry := DnWaterModels.model_entry(id)
	if entry.is_empty():
		return {}
	var scene := ModelLibrary.get_scene("%s_lod%d" % [entry["path"], lod])
	if scene == null:
		return {}
	var root := scene.instantiate() as Node3D
	var found := root.find_children("*", "MeshInstance3D", true, false)
	var result := {}
	if not found.is_empty():
		var mesh_instance := found[0] as MeshInstance3D
		var local := Transform3D.IDENTITY
		var node: Node3D = mesh_instance
		while node != null and node != root:
			local = node.transform * local
			node = node.get_parent() as Node3D
		var box := local * mesh_instance.mesh.get_aabb()
		var fit := float(entry.get("length", 1.0)) / maxf(maxf(box.size.x, box.size.z), 0.001)
		var center := box.get_center()
		var place := Transform3D(Basis.from_scale(Vector3.ONE * fit), Vector3.ZERO) * Transform3D(Basis.IDENTITY, Vector3(-center.x, -box.position.y, -center.z)) * local
		var baked_mesh := ArrayMesh.new()
		var texture: Texture2D = null
		for surface in mesh_instance.mesh.get_surface_count():
			var arrays := mesh_instance.mesh.surface_get_arrays(surface)
			var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
			for i in vertices.size():
				vertices[i] = place * vertices[i]
			arrays[Mesh.ARRAY_VERTEX] = vertices
			arrays[Mesh.ARRAY_NORMAL] = null
			arrays[Mesh.ARRAY_TANGENT] = null
			baked_mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
			var material := mesh_instance.mesh.surface_get_material(surface)
			if texture == null and material is BaseMaterial3D:
				texture = (material as BaseMaterial3D).albedo_texture
		result = {"mesh": baked_mesh, "texture": texture}
	root.free()
	return result
