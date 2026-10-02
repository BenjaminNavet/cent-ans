class_name Sea
extends MeshInstance3D

## Plan d'eau à Y = 0 couvrant la carte (avec marge) et shader d'eau animé (profondeur,
## vagues, écume). La heightmap et la distance à la côte sont reprises du nœud frère
## `Terrain` (TerrainBuilder, construit avant `setup`) pour ne pas les charger deux fois.

const WATER_SHADER := preload("res://shaders/water.gdshader")

@export var margin_factor: float = 0.5
## Couleur = fond marin du shader terrain (deep_sea mélangé au parchemin).
@export var abyss_color: Color = Color(0.03, 0.10, 0.15)
@export var abyss_depth: float = -4.5


func setup(map_size: Vector2i) -> void:
	var plane := PlaneMesh.new()
	var extent := Vector2(map_size) * (1.0 + margin_factor * 2.0)
	plane.size = extent
	mesh = plane
	position = Vector3(map_size.x * 0.5, 0.0, map_size.y * 0.5)
	var material := ShaderMaterial.new()
	material.shader = WATER_SHADER
	material.set_shader_parameter("map_size", Vector2(map_size))
	var terrain := get_parent().get_node_or_null("Terrain") as TerrainBuilder if get_parent() != null else null
	if terrain != null and terrain.map_data != null and terrain.height_texture() != null:
		material.set_shader_parameter("heightmap", terrain.height_texture())
		material.set_shader_parameter("has_heightmap", true)
		material.set_shader_parameter("height_bpp", terrain.height_texture_mode())
		material.set_shader_parameter("height_little_endian", terrain.map_data.height_little_endian)
		material.set_shader_parameter("height_min_m", terrain.map_data.height_min_m)
		material.set_shader_parameter("height_max_m", terrain.map_data.height_max_m)
		if terrain.coast_texture() != null:
			material.set_shader_parameter("coast_dist", terrain.coast_texture())
			material.set_shader_parameter("has_coast_dist", true)
	CampaignTextures.apply_water(material)  # GA4 : normales animées, couleur de profondeur
	WaterDetail.apply(material, "sea")  # RC5 : détail Nano Banana 2 (repli : rendu inchangé)
	WaterDetail.apply(material, "ocean", "ocean_detail")
	material_override = material
	for key: String in SeasonLook.storm():  # TB1 : mer sous la tempête
		material.set_shader_parameter("storm_" + key, SeasonLook.storm()[key])
	SeaBasins.apply(material, map_size)  # TB5 : mers par bassin
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


## Lot TB1 : mer de la saison (`weights` : poids printemps, été, automne, hiver de `SeasonVisuals`),
## valeurs de `data/ui/campaign_seasons.json`.
func apply_season(weights: Vector4) -> void:
	var material := material_override as ShaderMaterial
	if material == null:
		return
	var look := SeasonLook.sea(weights)
	material.set_shader_parameter("season_tint", look["tint"])
	material.set_shader_parameter("season_grey", look["grey"])
	material.set_shader_parameter("season_grey_amount", look["grey_amount"])
	material.set_shader_parameter("season_foam", look["foam"])


## Lot TB1 : reprend du terrain le masque météo par province (CM2) et la carte des provinces,
## pour l'écume de tempête.
func sync_weather(terrain_material: ShaderMaterial) -> void:
	var material := material_override as ShaderMaterial
	if material == null or terrain_material == null:
		return
	for key in ["province_ids", "weather_mask", "weather_enabled"]:
		var value: Variant = terrain_material.get_shader_parameter(key)
		if value != material.get_shader_parameter(key):
			material.set_shader_parameter(key, value)
