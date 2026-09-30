extends SceneTree

## Test headless du lot VT-C (ADR 0138) : maillage lointain des villes.
##  1. `town_far.gdshader` compile (le rendu factice de --headless compile le code et liste les
##     uniformes ; un shader en erreur n'en liste aucun) et expose les uniformes attendus ;
##  2. `roofscape.gdshaderinc` est inclus par `town_building` et `town_far` (teinte partagée) ;
##  3. matériau posé sur un petit ArrayMesh conforme au contrat de sommets, dessiné quelques images ;
##  4. `TownFarMask` : texel (i % 64, i / 64), bornes, envoi seulement si le masque a changé.
## Pas d'image : le rendu factice ne dessine rien.
## Usage : godot --headless --path game --script res://tests/tf_far_shader_test.gd

const FAR_UNIFORMS := ["albedo_array", "layer_tint", "roof_first", "roof_last", "snow", "meters_per_unit", "roof_mix", "roofscape", "roofscape_near", "roofscape_far", "roofscape_cell_m", "roofscape_gain", "built_mask", "sink_distance", "sink_band", "sink_depth_m", "sink_camera"]

var _failures := 0


func _init() -> void:
	await process_frame
	_test_shader()
	_test_include()
	_test_mask()
	await _test_mesh()
	print("tf_far_shader_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("tf_far_shader_test: " + message)
	return condition


func _uniform_names(shader: Shader) -> Array:
	return shader.get_shader_uniform_list().map(func(u: Dictionary) -> String: return str(u["name"]))


func _test_shader() -> void:
	var far: Shader = load("res://shaders/town_far.gdshader")
	var names := _uniform_names(far)
	for u in FAR_UNIFORMS:
		_check(names.has(u), "town_far uniform %s (got %s)" % [u, names])
	var building: Shader = load("res://shaders/town_building.gdshader")
	var b_names := _uniform_names(building)
	for u in ["roofscape", "roofscape_near", "roofscape_far", "roofscape_cell_m", "roofscape_gain", "lod_mode", "base_source"]:
		_check(b_names.has(u), "town_building uniform %s still exposed" % u)


func _test_include() -> void:
	var inc := "#include \"res://shaders/roofscape.gdshaderinc\""
	var building := FileAccess.get_file_as_string("res://shaders/town_building.gdshader")
	var far := FileAccess.get_file_as_string("res://shaders/town_far.gdshader")
	_check(building.contains(inc) and far.contains(inc), "roofscape include shared")
	_check(not building.contains("float roof_hash("), "roof_hash lives in the include only")
	_check(far.contains("campaign_display_height("), "far ground posed by campaign_display_height (ZG8)")


func _test_mask() -> void:
	var mask := TownFarMask.new()
	var tex := mask.texture()
	_check(tex != null and tex.get_width() == 64 and tex.get_height() == 64, "mask 64 x 64")
	_check(mask.image().get_format() == Image.FORMAT_R8, "mask R8")
	_check(mask.uploads == 1, "one upload at creation")
	_check(mask.texture() == tex and mask.uploads == 1, "no upload when unchanged")
	mask.set_built(0, true)
	mask.set_built(65, true)
	mask.set_built(4095, true)
	mask.set_built(4096, true)
	mask.set_built(-1, true)
	_check(mask.is_built(65) and not mask.is_built(64) and not mask.is_built(4096), "is_built")
	_check(mask.texture() == tex and mask.uploads == 2, "one upload after changes")
	# Le rendu factice ne relit pas les textures : on lit l'image source du masque.
	var img := mask.image()
	_check(img.get_pixel(0, 0).r > 0.99 and img.get_pixel(1, 1).r > 0.99 and img.get_pixel(63, 63).r > 0.99, "texels (i % 64, i / 64) set")
	_check(img.get_pixel(1, 0).r < 0.01 and img.get_pixel(0, 1).r < 0.01, "neighbours unset")
	mask.set_built(65, true)
	mask.texture()
	_check(mask.uploads == 2, "setting the same value does not upload")
	mask.set_built(65, false)
	_check(mask.texture() == tex and mask.image().get_pixel(1, 1).r < 0.01 and mask.uploads == 3, "unset uploads once")


## Nappe de toit (4 sommets à 12 m au-dessus d'un sol à 80 m) et un pan de jupe (−30 m), ville 65.
func _contract_mesh() -> ArrayMesh:
	var ground := 80.0
	var verts := PackedVector3Array()
	var uv2 := PackedVector2Array()
	var colors := PackedColorArray()
	var normals := PackedVector3Array()
	var corners := [Vector2(100.0, 100.0), Vector2(100.1, 100.0), Vector2(100.1, 100.1), Vector2(100.0, 100.1)]
	for c: Vector2 in corners:
		verts.append(Vector3(c.x, ground + 12.0, c.y))
		uv2.append(Vector2(65.0, 12.0))
		colors.append(Color(0.6, 0.45, 0.35))
		normals.append(Vector3.UP)
	for c: Vector2 in [corners[0], corners[1]]:
		verts.append(Vector3(c.x, ground - 30.0, c.y))
		uv2.append(Vector2(65.0, -30.0))
		colors.append(Color(0.5, 0.5, 0.5))
		normals.append(Vector3.BACK)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_TEX_UV2] = uv2
	arrays[Mesh.ARRAY_INDEX] = PackedInt32Array([0, 1, 2, 0, 2, 3, 0, 4, 5, 0, 5, 1])
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh


func _test_mesh() -> void:
	var mesh := _contract_mesh()
	_check(mesh.get_surface_count() == 1 and mesh.surface_get_format(0) & Mesh.ARRAY_FORMAT_TEX_UV2 != 0, "contract mesh has UV2")
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/town_far.gdshader")
	var mask := TownFarMask.new()
	mask.set_built(65, true)
	mat.set_shader_parameter("built_mask", mask.texture())
	mat.set_shader_parameter("sink_distance", 1.5)
	mat.set_shader_parameter("roofscape", 1.0)
	_check(mat.get_shader_parameter("built_mask") == mask.texture(), "mask bound to material")
	var node := MeshInstance3D.new()
	node.mesh = mesh
	node.material_override = mat
	var cam := Camera3D.new()
	root.add_child(node)
	root.add_child(cam)
	cam.position = Vector3(100.05, 1.0, 101.0)
	for i in 3:
		await process_frame
	_check(is_instance_valid(node) and node.is_inside_tree(), "far mesh drawn without error")
	node.queue_free()
	cam.queue_free()
	await process_frame
