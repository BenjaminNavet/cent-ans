extends RefCounted

## Lot FA2 : A/B des planches de feu et de fumée dans les scripts de capture. Charge
## `flame_flipbook.png` et `smoke_flipbook.png` depuis un dossier quelconque (hors dépôt) et les
## pose sur tous les matériaux de la scène qui utilisent `fire_flame.gdshader` ou
## `fire_smoke.gdshader`, sans toucher au code du jeu. À rappeler avant chaque capture : les
## émetteurs créés entre-temps (nouveaux foyers, colonnes de fumée) ont des matériaux neufs.

const SHADERS := {
	"res://shaders/fire_flame.gdshader": "flame_flipbook.png",
	"res://shaders/fire_smoke.gdshader": "smoke_flipbook.png",
}


## Textures (avec mipmaps) par chemin de shader ; vide si le dossier est vide ou illisible.
static func load_textures(folder: String) -> Dictionary:
	var textures := {}
	if folder == "":
		return textures
	for shader_path in SHADERS:
		var path := folder.path_join(SHADERS[shader_path])
		var image := Image.load_from_file(path)
		if image == null:
			push_error("fx_flipbook_override: cannot read %s" % path)
			continue
		image.generate_mipmaps()
		textures[shader_path] = ImageTexture.create_from_image(image)
	return textures


## Remplace la planche de chaque matériau de feu ou de fumée sous `node` ; renvoie leur nombre.
static func apply(node: Node, textures: Dictionary) -> int:
	if textures.is_empty():
		return 0
	var seen := {}
	_walk(node, textures, seen)
	return seen.size()


static func _walk(node: Node, textures: Dictionary, seen: Dictionary) -> void:
	if node is GeometryInstance3D:
		var geometry := node as GeometryInstance3D
		_swap(geometry.material_override, textures, seen)
		var mesh: Mesh = null
		if node is GPUParticles3D:
			mesh = (node as GPUParticles3D).draw_pass_1
		elif node is MeshInstance3D:
			mesh = (node as MeshInstance3D).mesh
		elif node is MultiMeshInstance3D and (node as MultiMeshInstance3D).multimesh != null:
			mesh = (node as MultiMeshInstance3D).multimesh.mesh
		if mesh != null:
			for surface in mesh.get_surface_count():
				_swap(mesh.surface_get_material(surface), textures, seen)
	for child in node.get_children():
		_walk(child, textures, seen)


static func _swap(material: Material, textures: Dictionary, seen: Dictionary) -> void:
	var shader_material := material as ShaderMaterial
	if shader_material == null or shader_material.shader == null or seen.has(shader_material):
		return
	var texture: Texture2D = textures.get(shader_material.shader.resource_path, null)
	if texture == null:
		return
	shader_material.set_shader_parameter("flipbook", texture)
	seen[shader_material] = true
