class_name LandmarkBackdrop
extends Node3D

## Toile de fond d'une bataille de siège dans une ville emblématique (lot L1, Paris) : la ville
## assiégée reste celle de la simulation (murailles, maisons-obstacles), le plan historique est
## posé derrière elle à l'échelle réelle — la Seine au pied de la muraille du fond, l'île de la
## Cité avec Notre-Dame, la Sainte-Chapelle et le palais, la rive droite et le Louvre.
## Rendu seulement. Modèle : `landmark_city.py --siege` → `assets/models/landmarks/<id>_siege.glb`
## (mètres, origine sur `anchor`, plan tourné de `siege.rotate_deg`, nord vers +Z après pose).
## Option de capture : `--landmark-backdrop=<id>` force la toile de fond quelle que soit la province.

const SHADER := preload("res://shaders/landmark.gdshader")
const HEIGHT_RES := 128
## Marge entre la muraille du fond et la berge (m).
const BANK_MARGIN := 40.0

var landmark: Dictionary = {}
var stats: Dictionary = {}


## Toile de fond pour ce siège, null si la bataille n'a pas lieu dans une ville emblématique.
static func create(setup: Dictionary, siege: Dictionary, height_at: Callable) -> LandmarkBackdrop:
	var forced := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--landmark-backdrop="):
			forced = arg.trim_prefix("--landmark-backdrop=")
	var province := str(setup.get("province", ""))
	for plan in LandmarkLibrary.all():
		var plan_dict: Dictionary = plan
		if not plan_dict.has("siege"):
			continue
		if (forced != "" and str(plan_dict.get("id", "")) == forced) or (forced == "" and province != "" and str(plan_dict.get("province", "")) == province):
			var path := "res://assets/models/landmarks/%s_siege.glb" % str(plan_dict["id"])
			if not ResourceLoader.exists(path):
				return null
			var node := LandmarkBackdrop.new()
			node.name = "LandmarkBackdrop"
			node._setup(plan_dict, (load(path) as PackedScene).instantiate() as Node3D, siege, height_at)
			return node
	return null


func _setup(plan: Dictionary, model: Node3D, siege: Dictionary, height_at: Callable) -> void:
	landmark = plan
	var siege_plan: Dictionary = plan["siege"]
	# Berge au-delà du pan de muraille le plus éloigné (côté +Z).
	var center: Vector2 = siege.get("center", Vector2(600, 560))
	var far_z := center.y
	for piece in siege.get("pieces", []):
		far_z = maxf(far_z, maxf((piece["a"] as Vector2).y, (piece["b"] as Vector2).y))
	var bank_z := far_z + BANK_MARGIN
	# L3 : `center_x_m` recentre la toile de fond (le modèle est tourné de π : x du plan → −x).
	position = Vector3(center.x + float(siege_plan.get("center_x_m", 0.0)), 0.0, bank_z + float(siege_plan.get("bank_offset_m", 100.0)))
	rotation.y = PI
	model.name = "Model"
	add_child(model)
	# Hauteurs du terrain de bataille sous la toile de fond.
	var radius := float(siege_plan.get("radius_m", 1400.0))
	var extent := radius * 2.0
	var origin := Vector2(position.x - radius, position.z - radius)
	var image := Image.create(HEIGHT_RES, HEIGHT_RES, false, Image.FORMAT_RF)
	for j in HEIGHT_RES:
		for i in HEIGHT_RES:
			var x := origin.x + (float(i) + 0.5) / HEIGHT_RES * extent
			var z := origin.y + (float(j) + 0.5) / HEIGHT_RES * extent
			image.set_pixel(i, j, Color(float(height_at.call(x, z)) if height_at.is_valid() else 0.0, 0.0, 0.0))
	var texture := ImageTexture.create_from_image(image)
	var triangles := 0
	for child in model.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := child as MeshInstance3D
		var mesh := mesh_instance.mesh
		if mesh == null:
			continue
		for surface in mesh.get_surface_count():
			triangles += mesh.surface_get_array_len(surface) / 3
			var source := mesh.surface_get_material(surface)
			var material := ShaderMaterial.new()
			material.shader = SHADER
			if source is BaseMaterial3D:
				material.set_shader_parameter("albedo", (source as BaseMaterial3D).albedo_color)
				material.set_shader_parameter("roughness", (source as BaseMaterial3D).roughness)
				material.set_shader_parameter("water", 1.0 if source.resource_name == "Water" else 0.0)
			material.set_shader_parameter("tint_strength", 0.0)
			material.set_shader_parameter("height_map", texture)
			material.set_shader_parameter("map_origin", origin)
			material.set_shader_parameter("map_extent", extent)
			mesh_instance.set_surface_override_material(surface, material)
		mesh_instance.extra_cull_margin = 200.0
		if mesh_instance.name == "houses":
			mesh_instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	stats = {"triangles": triangles, "bank_z": bank_z}
	print("LandmarkBackdrop: %s behind the siege (bank z %.0f, %d triangles)" % [plan.get("id", ""), bank_z, triangles])
