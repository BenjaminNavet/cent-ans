class_name Sea
extends MeshInstance3D

## Plan d'eau à Y = 0 couvrant la carte (avec marge) et shader d'eau animé.

const WATER_SHADER := preload("res://shaders/water.gdshader")

@export var margin_factor: float = 0.5
## Couleur = fond marin du shader terrain (deep_sea mélangé au parchemin).
@export var abyss_color: Color = Color(0.166, 0.288, 0.321)
@export var abyss_depth: float = -4.5


func setup(map_size: Vector2i) -> void:
	var plane := PlaneMesh.new()
	var extent := Vector2(map_size) * (1.0 + margin_factor * 2.0)
	plane.size = extent
	mesh = plane
	position = Vector3(map_size.x * 0.5, 0.0, map_size.y * 0.5)
	var material := ShaderMaterial.new()
	material.shader = WATER_SHADER
	material_override = material
	# Fond opaque sous l'eau transparente : masque le bord de la carte et l'arrière-plan.
	var floor_instance := MeshInstance3D.new()
	floor_instance.name = "Abyss"
	var floor_mesh := PlaneMesh.new()
	floor_mesh.size = extent
	floor_instance.mesh = floor_mesh
	floor_instance.position = Vector3(0.0, abyss_depth, 0.0)
	var floor_material := StandardMaterial3D.new()
	floor_material.albedo_color = abyss_color
	floor_material.roughness = 0.92
	floor_material.metallic_specular = 0.15
	floor_instance.material_override = floor_material
	add_child(floor_instance)
