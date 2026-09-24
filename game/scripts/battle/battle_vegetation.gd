class_name BattleVegetation
extends Node3D

## Herbe animée des batailles (lot V4) : deux grilles de touffes (proche dense, lointaine plus
## clairsemée et plus grande) en `MultiMesh`, déplacées par pas entiers de leur maille pour suivre
## le point regardé par la caméra. Tout le placement est dans `battle_grass.gdshader`.
## Masquée quand la caméra est très haute (l'herbe n'y serait qu'un bruit de pixels).
## Lot B5 : teinte et neige selon la saison et le sol du site (`BattleTerrain.snowy()`).

const GRASS_SHADER := preload("res://shaders/battle_grass.gdshader")
const GRASS_TEXTURE := preload("res://assets/textures/battle/grass_clump.png")
const MAX_CAMERA_HEIGHT := 170.0

## [maille (m), rayon (m), échelle des touffes, rayon intérieur (m)]
const LAYERS := [[0.5, 40.0, 1.0, 0.0], [1.1, 95.0, 1.35, 34.0]]

var _layers: Array[MultiMeshInstance3D] = []
var _materials: Array[ShaderMaterial] = []
var _ground_y: float = 0.0


func build(terrain: BattleTerrain, weather: String) -> void:
	_ground_y = terrain.height_at(600.0, 400.0)
	var mesh := _clump_mesh()
	for layer in LAYERS:
		var spacing: float = layer[0]
		var radius: float = layer[1]
		var mat := ShaderMaterial.new()
		mat.shader = GRASS_SHADER
		mat.set_shader_parameter("grass_texture", GRASS_TEXTURE)
		mat.set_shader_parameter("height_map", terrain.height_texture)
		var hr := BattleTerrain.SPLAT_RECT
		var t := BattleTerrain.HEIGHT_TEXEL
		var hw := float(int(hr.size.x / t) + 1) * t
		var hh := float(int(hr.size.y / t) + 1) * t
		mat.set_shader_parameter("height_rect", Vector4(hr.position.x - t * 0.5, hr.position.y - t * 0.5, hw, hh))
		mat.set_shader_parameter("albedo_array", BattleTerrain.ALBEDO_ARRAY)
		mat.set_shader_parameter("macro_noise", terrain.macro_noise)
		mat.set_shader_parameter("splat_a", terrain.splat_a)
		mat.set_shader_parameter("splat_b", terrain.splat_b)
		mat.set_shader_parameter("splat_rect", Vector4(hr.position.x, hr.position.y, hr.size.x, hr.size.y))
		mat.set_shader_parameter("spacing", spacing)
		mat.set_shader_parameter("radius", radius)
		mat.set_shader_parameter("blade_scale", float(layer[2]))
		mat.set_shader_parameter("inner_radius", float(layer[3]))
		# B5 : sol de saison (neige au sol sans chute de neige), herbe d'hiver et d'automne
		# plus sèche, joncs plus sombres du marais.
		var ground_weather := "snow" if terrain.snowy() else weather
		if terrain.site_render and ground_weather == "clear":
			match terrain.season_key:
				"winter":
					mat.set_shader_parameter("base_tint", Color(0.95, 0.88, 0.7))
				"autumn":
					mat.set_shader_parameter("base_tint", Color(1.0, 0.92, 0.72))
			if terrain.terrain_key == "marsh":
				mat.set_shader_parameter("base_tint", Color(0.82, 0.9, 0.7))
		match ground_weather:
			"snow":
				mat.set_shader_parameter("snow", 1.0 if weather == "snow" else 0.8)
				mat.set_shader_parameter("base_tint", Color(0.85, 0.85, 0.75))
			"rain":
				mat.set_shader_parameter("base_tint", Color(0.8, 0.88, 0.75))
				mat.set_shader_parameter("wind_strength", 1.6)
			"fog":
				mat.set_shader_parameter("wind_strength", 0.4)
		var n := int(ceil(radius * 2.0 / spacing))
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = mesh
		mm.instance_count = n * n
		var buffer := PackedFloat32Array()
		buffer.resize(n * n * 12)
		var half := float(n) * spacing * 0.5
		var i := 0
		for iz in n:
			for ix in n:
				var o := i * 12
				buffer[o] = 1.0
				buffer[o + 3] = float(ix) * spacing - half
				buffer[o + 5] = 1.0
				buffer[o + 10] = 1.0
				buffer[o + 11] = float(iz) * spacing - half
				i += 1
		mm.buffer = buffer
		# Les sommets sont déplacés dans le shader : boîte englobante généreuse.
		mm.mesh.custom_aabb = AABB(Vector3(-4, -200, -4), Vector3(8, 600, 8))
		var instance := MultiMeshInstance3D.new()
		instance.name = "Grass%d" % _layers.size()
		instance.multimesh = mm
		instance.material_override = mat
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		instance.custom_aabb = AABB(Vector3(-radius, -300, -radius), Vector3(radius * 2.0, 900, radius * 2.0))
		add_child(instance)
		instance.set_meta("spacing", spacing)
		_layers.append(instance)
		_materials.append(mat)


func _process(_delta: float) -> void:
	var camera := get_viewport().get_camera_3d()
	if camera == null or _layers.is_empty():
		return
	var focus := _focus_point(camera, _ground_y)
	var high := camera.global_position.y - focus.y > MAX_CAMERA_HEIGHT
	for k in _layers.size():
		var instance := _layers[k]
		instance.visible = not high
		if high:
			continue
		var spacing: float = instance.get_meta("spacing")
		instance.global_position = Vector3(snappedf(focus.x, spacing), 0.0, snappedf(focus.z, spacing))
		_materials[k].set_shader_parameter("focus", focus)


## Point du sol regardé (intersection du rayon central avec le plan moyen), rapproché de la
## caméra pour que l'herbe couvre surtout le premier plan.
static func _focus_point(camera: Camera3D, ground_y: float) -> Vector3:
	var origin := camera.global_position
	var forward := -camera.global_transform.basis.z
	var t := 200.0
	if forward.y < -0.05:
		t = (ground_y - origin.y) / forward.y
	var hit := origin + forward * clampf(t, 0.0, 600.0)
	var cam_ground := Vector3(origin.x, hit.y, origin.z)
	return cam_ground.lerp(hit, 0.72)


## Touffe : trois cartes croisées à 60°, 0,6 m de large, 0,42 m de haut, pied à l'origine.
static func _clump_mesh() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for k in 3:
		var a := PI * float(k) / 3.0
		var d := Vector3(cos(a), 0.0, sin(a)) * 0.3
		var n := Vector3(-sin(a), 0.0, cos(a))
		var corners := [-d, d, d + Vector3(0, 0.42, 0), -d + Vector3(0, 0.42, 0)]
		var uvs := [Vector2(0, 1), Vector2(1, 1), Vector2(1, 0), Vector2(0, 0)]
		for idx in [0, 1, 2, 0, 2, 3]:
			st.set_normal(n)
			st.set_uv(uvs[idx])
			st.add_vertex(corners[idx])
	st.index()
	return st.commit()
