class_name MenuBackdrop3D
extends Node3D

## Lot MM1 — décor 3D vivant du menu principal : Paris (maquette L1 `paris_siege.glb`, ADR 0015)
## au crépuscule sous un ciel HDRI V3 (`AtmosphereLibrary`), plans de caméra lents enchaînés par
## un fondu au noir, ost au premier plan (figurines skinnées V2 en `MultiMesh`, ADR 0014) avec
## bannières au vent. Réglages dans `data/ui/front_end.json` (`backdrop`). Rendu seulement.
## Options (après `--`) : `--menu-shot=<n>` fige le plan n (captures) ; `--menu-shot-t=<0..1>`
## sa progression ; `--menu-camera=x,y,z,tx,ty,tz` impose une caméra (mise au point).

signal shot_changed(index: int, label: String)

const LANDMARK_SHADER := preload("res://shaders/landmark.gdshader")
const GROUND_SHADER := preload("res://shaders/menu_ground.gdshader")
const GRASS_SHADER := preload("res://shaders/menu_grass.gdshader")
const GRASS_TEXTURE_PATH := "res://assets/textures/battle/grass_clump.png"
const BANNER_SHADER := preload("res://shaders/battle_banner.gdshader")
const BANNER_DIR := "res://assets/heraldry/banners/"
const BANNER_HEIGHT := 7.0
const DIP_SECONDS := 1.1
const GROUND_SIZE := 24000.0

var config: Dictionary = {}
var camera: Camera3D
var sun: DirectionalLight3D
var world_env: WorldEnvironment
var shot_index: int = 0
var shot_time: float = 0.0
## Statistiques de construction (captures, test) : triangles, soldats, durée.
var stats: Dictionary = {}

var _shots: Array = []
var _shot_seconds := 26.0
var _anim_time := 0.0
var _soldier_materials: Array[ShaderMaterial] = []
var _host_root: Node3D
var _foreground: Node3D  # ost et herbe : visibles dans les plans « host »
var _fade_layer: CanvasLayer
var _fade: ColorRect
var _frozen_shot := -1
var _frozen_t := -1.0
var _forced_camera: Array = []
var _dipping := false
var _water_albedo := Color(0, 0, 0, 0)


func _ready() -> void:
	var started := Time.get_ticks_usec()
	config = FrontEndData.backdrop()
	_shots = config.get("shots", [])
	_shot_seconds = float(config.get("shot_seconds", 26.0))
	_parse_args()
	_foreground = Node3D.new()
	_foreground.name = "Foreground"
	add_child(_foreground)
	_build_environment()
	_build_city()
	_build_ground()
	if config.has("host"):
		_build_host(config["host"])
	_build_fade()
	camera = Camera3D.new()
	camera.name = "MenuCamera"
	camera.near = 0.5
	camera.far = 12000.0
	add_child(camera)
	camera.current = true
	if _frozen_shot >= 0:
		shot_index = clampi(_frozen_shot, 0, maxi(_shots.size() - 1, 0))
	_enter_shot(shot_index)
	stats["build_ms"] = (Time.get_ticks_usec() - started) / 1000.0
	print("MenuBackdrop3D: built in %.0f ms (%s)" % [stats["build_ms"], JSON.stringify(stats)])


func _parse_args() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--menu-shot="):
			_frozen_shot = int(arg.trim_prefix("--menu-shot="))
		elif arg.begins_with("--menu-shot-t="):
			_frozen_t = clampf(float(arg.trim_prefix("--menu-shot-t=")), 0.0, 1.0)
		elif arg.begins_with("--menu-camera="):
			var parts := arg.trim_prefix("--menu-camera=").split(",")
			if parts.size() == 6:
				for p in parts:
					_forced_camera.append(float(p))


# --- Construction ------------------------------------------------------------------------------


func _build_environment() -> void:
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = _rgb(config.get("fog_color", [0.6, 0.5, 0.45]))
	env.tonemap_mode = Environment.TONE_MAPPER_AGX
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.ambient_light_energy = 0.9
	env.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	env.fog_enabled = true
	env.fog_mode = Environment.FOG_MODE_EXPONENTIAL
	env.fog_light_color = _rgb(config.get("fog_color", [0.6, 0.5, 0.45]))
	env.fog_density = float(config.get("fog_density", 0.0007))
	env.fog_aerial_perspective = 0.55
	env.fog_sky_affect = 0.0
	env.fog_height = 40.0
	env.fog_height_density = 0.004
	env.glow_enabled = true
	env.glow_intensity = 0.55
	env.glow_bloom = 0.08
	env.glow_hdr_threshold = 0.9
	env.ssao_enabled = true
	env.ssao_radius = 1.5
	env.ssao_intensity = 1.2
	var sky_id := str(config.get("sky", "dawn"))
	var resolved := {
		"sky_id": sky_id,
		"sky": AtmosphereLibrary.sky_info(sky_id),
		"look": {
			"sky_luminance": float(config.get("sky_luminance", 0.5)),
			"sky_saturation": float(config.get("sky_saturation", 1.1)),
			"sky_tint": config.get("sky_tint", [1, 1, 1]),
			"sun_disc": 0.0,
			"haze": 0.25,
		},
		"grades": [{"temperature": 0.2, "contrast": 1.06, "saturation": 1.05, "gain": [1.04, 0.99, 0.92], "shadows": [0.95, 0.97, 1.06]}],
		"strength": 1.0,
	}
	AtmosphereLibrary.apply_to_environment(env, resolved, _rgb(config.get("fog_color", [0.6, 0.5, 0.45])), _rgb(config.get("ground_color", [0.24, 0.26, 0.16])))
	world_env = WorldEnvironment.new()
	world_env.environment = env
	add_child(world_env)

	sun = DirectionalLight3D.new()
	sun.name = "Sun"
	sun.light_color = _rgb(config.get("sun_color", [1.0, 0.65, 0.4]))
	sun.light_energy = float(config.get("sun_energy", 1.6))
	sun.shadow_enabled = true
	sun.directional_shadow_max_distance = 1800.0
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	var elevation := deg_to_rad(float(config.get("sun_elevation_deg", 7.0)))
	# Soleil couchant à l'ouest-sud-ouest (la caméra regarde surtout vers le nord, -Z) : lumière
	# rasante de trois quarts, façades dorées, longues ombres.
	var sky: Dictionary = AtmosphereLibrary.sky_info(sky_id)
	var azimuth := deg_to_rad(float(sky.get("sun_azimuth_deg", 250.0)))
	sun.rotation = Vector3(-elevation, azimuth, 0.0)
	add_child(sun)


func _build_city() -> void:
	var path := str(config.get("model", ""))
	if path == "" or not ResourceLoader.exists(path):
		push_warning("MenuBackdrop3D: model %s missing" % path)
		return
	var model := (load(path) as PackedScene).instantiate() as Node3D
	model.name = "City"
	add_child(model)
	var height := Image.create(1, 1, false, Image.FORMAT_RF)
	height.set_pixel(0, 0, Color(0, 0, 0))
	var height_texture := ImageTexture.create_from_image(height)
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
			material.shader = LANDMARK_SHADER
			if source is BaseMaterial3D:
				material.set_shader_parameter("albedo", (source as BaseMaterial3D).albedo_color)
				material.set_shader_parameter("roughness", (source as BaseMaterial3D).roughness)
				material.set_shader_parameter("water", 1.0 if source.resource_name == "Water" else 0.0)
				if source.resource_name == "Water":
					_water_albedo = (source as BaseMaterial3D).albedo_color
			material.set_shader_parameter("tint_strength", 0.0)
			material.set_shader_parameter("height_map", height_texture)
			material.set_shader_parameter("map_origin", Vector2(-1.0e5, -1.0e5))
			material.set_shader_parameter("map_extent", 2.0e5)
			mesh_instance.set_surface_override_material(surface, material)
		mesh_instance.extra_cull_margin = 50.0
		if mesh_instance.name == "houses":
			mesh_instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	stats["city_triangles"] = triangles


## Campagne autour de la maquette : Seine prolongée, champs, bosquets et peupliers des berges.
func _build_ground() -> void:
	var land: Dictionary = config.get("countryside", {})
	var plane := PlaneMesh.new()
	plane.size = Vector2(GROUND_SIZE, GROUND_SIZE)
	var material := ShaderMaterial.new()
	material.shader = GROUND_SHADER
	material.set_shader_parameter("river_z", float(land.get("river_z", 170.0)))
	material.set_shader_parameter("river_half_width", float(land.get("river_half_width", 85.0)))
	var zone: Array = land.get("meadow_zone", [0, 0, 0, 0])
	material.set_shader_parameter("meadow_zone", Vector4(float(zone[0]), float(zone[1]), float(zone[2]), float(zone[3])))
	if _water_albedo.a > 0.0:
		material.set_shader_parameter("water_color", Color(_water_albedo, 1.0))
	plane.material = material
	var ground := MeshInstance3D.new()
	ground.name = "Countryside"
	ground.mesh = plane
	ground.position = Vector3(0.0, -0.4, 0.0)
	ground.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(ground)
	_build_trees(land)
	if land.has("grass"):
		_build_grass(land["grass"])


func _build_trees(land: Dictionary) -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = int(land.get("seed", 1337))
	var river_z := float(land.get("river_z", 170.0))
	var half := float(land.get("river_half_width", 85.0))
	var keep_out: Array = land.get("keep_out", [])
	var groups := {"oak": [], "poplar": [], "bush": []}
	# Bosquets : grappes d'arbres sur la rive gauche et au-delà de la ville.
	for _c in int(land.get("groves", 60)):
		var center := Vector2(rng.randf_range(-3500.0, 3500.0), rng.randf_range(-4200.0, 4200.0))
		if center.y > -1300.0 and center.y < 320.0 and absf(center.x) < 2000.0:
			continue
		for _t in rng.randi_range(6, 26):
			var at := center + Vector2(rng.randf_range(-60.0, 60.0), rng.randf_range(-45.0, 45.0))
			(groups["oak" if rng.randf() < 0.8 else "bush"] as Array).append(at)
	# Peupliers le long de la berge sud, par rangées interrompues.
	var x := -3200.0
	while x < 3200.0:
		x += rng.randf_range(14.0, 30.0)
		if rng.randf() < 0.35:
			x += rng.randf_range(80.0, 260.0)
			continue
		var center_z := river_z + sin(x * 0.0011) * 38.0 + sin(x * 0.0031 + 1.7) * 12.0
		(groups["poplar"] as Array).append(Vector2(x, center_z + half + rng.randf_range(10.0, 22.0)))
	var count := 0
	for kind in groups:
		var points: Array = groups[kind]
		var kept: Array[Vector2] = []
		for point: Vector2 in points:
			var blocked := false
			for box in keep_out:
				var b: Array = box
				if point.x >= float(b[0]) and point.x <= float(b[2]) and point.y >= float(b[1]) and point.y <= float(b[3]):
					blocked = true
			if not blocked:
				kept.append(point)
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.use_colors = true
		mm.mesh = BattleMeshes.tree(kind)
		mm.instance_count = kept.size()
		for i in kept.size():
			var scale := rng.randf_range(0.8, 1.35)
			var basis := Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3.ONE * scale)
			mm.set_instance_transform(i, Transform3D(basis, Vector3(kept[i].x, -0.3, kept[i].y)))
			mm.set_instance_color(i, Color(rng.randf_range(0.8, 1.05), rng.randf_range(0.82, 1.0), rng.randf_range(0.7, 0.9)))
		var instance := MultiMeshInstance3D.new()
		instance.name = "Trees_%s" % kind
		instance.multimesh = mm
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if kind != "bush" else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(instance)
		count += kept.size()
	stats["trees"] = count


## Touffes d'herbe du premier plan (plan de l'ost seulement).
func _build_grass(grass: Dictionary) -> void:
	var rect: Array = grass.get("rect", [0, 0, 0, 0])
	var count := int(grass.get("count", 0))
	if count <= 0 or not ResourceLoader.exists(GRASS_TEXTURE_PATH):
		return
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for k in 2:
		var angle := PI * 0.5 * k + 0.4
		var dir := Vector3(cos(angle), 0.0, sin(angle)) * 0.35
		var normal := Vector3(-dir.z, 0.0, dir.x).normalized()
		var corners := [-dir, dir, dir + Vector3(0, 0.55, 0), -dir + Vector3(0, 0.55, 0)]
		var uvs := [Vector2(0, 1), Vector2(1, 1), Vector2(1, 0), Vector2(0, 0)]
		for idx in [0, 1, 2, 0, 2, 3]:
			st.set_normal(normal)
			st.set_color(Color.WHITE)
			st.set_uv(uvs[idx])
			st.add_vertex(corners[idx])
	var material := ShaderMaterial.new()
	material.shader = GRASS_SHADER
	material.set_shader_parameter("grass_texture", load(GRASS_TEXTURE_PATH))
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.mesh = st.commit()
	mm.instance_count = count
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	for i in count:
		var at := Vector3(rng.randf_range(float(rect[0]), float(rect[2])), -0.4, rng.randf_range(float(rect[1]), float(rect[3])))
		var basis := Basis(Vector3.UP, rng.randf() * TAU).scaled(Vector3.ONE * rng.randf_range(0.45, 1.0))
		mm.set_instance_transform(i, Transform3D(basis, at))
		var shade := rng.randf_range(0.75, 1.1)
		mm.set_instance_color(i, Color(shade * rng.randf_range(0.95, 1.1), shade, shade * rng.randf_range(0.8, 1.0)))
	var instance := MultiMeshInstance3D.new()
	instance.name = "Grass"
	instance.multimesh = mm
	instance.material_override = material
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_foreground.add_child(instance)
	stats["grass"] = count


## Ost au premier plan : régiments en ordre de bataille, bannières.
func _build_host(host: Dictionary) -> void:
	_host_root = Node3D.new()
	_host_root.name = "Host"
	var center: Array = host.get("center", [0, 0, 0])
	_host_root.position = Vector3(float(center[0]), float(center[1]), float(center[2]))
	_host_root.rotation.y = deg_to_rad(float(host.get("facing_deg", 0.0)))
	_foreground.add_child(_host_root)
	var rng := RandomNumberGenerator.new()
	rng.seed = 1337
	var soldiers := 0
	var index := 0
	for entry in host.get("regiments", []):
		var regiment: Dictionary = entry
		soldiers += _build_regiment(regiment, rng, index)
		index += 1
	stats["soldiers"] = soldiers


func _build_regiment(regiment: Dictionary, rng: RandomNumberGenerator, index: int) -> int:
	var kind := str(regiment["kind"])
	var variant := int(regiment["variant"])
	var faction := str(regiment["faction"])
	var files := int(regiment["files"])
	var ranks := int(regiment["ranks"])
	var spacing := float(regiment["spacing"])
	var offset: Array = regiment["offset"]
	var origin := Vector3(float(offset[0]), 0.0, float(offset[1]))
	var facade := get_node_or_null("/root/SimFacade")
	var info: Dictionary = facade.call("faction_info", faction) if facade != null else {}
	var color: Color = info.get("color", Color(0.4, 0.4, 0.6))
	if BattleSkinned.has_figure(kind, variant):
		var material := ShaderMaterial.new()
		material.shader = BattleSkinned.SHADER
		material.set_shader_parameter("livery", color)
		material.set_shader_parameter("trim", Color(0.85, 0.7, 0.25) if color.get_luminance() < 0.55 else Color(0.8, 0.82, 0.86))
		var arms := PortraitLoader.heraldry_texture(faction)
		material.set_shader_parameter("heraldry", arms)
		material.set_shader_parameter("has_heraldry", arms != null)
		BattleSkinned.setup_material(material, kind, variant)
		material.set_shader_parameter("livery_share", 0.9 if variant == 0 else 0.6)
		BattleSkinned.apply_config(material, BattleSkinned.state_config(kind, variant, "idle", false), 0.0)
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = BattleSkinned.mesh(kind, variant, 0)
		mm.instance_count = files * ranks
		var i := 0
		for rank in ranks:
			for file in files:
				var x := (float(file) - float(files - 1) * 0.5) * spacing + rng.randf_range(-0.18, 0.18) * spacing
				var z := float(rank) * spacing * (1.4 if kind == "cavalry" else 1.1) + rng.randf_range(-0.15, 0.15) * spacing
				var basis := Basis(Vector3.UP, rng.randf_range(-0.12, 0.12))
				mm.set_instance_transform(i, Transform3D(basis, origin + Vector3(x, 0.0, z)))
				i += 1
		var instance := MultiMeshInstance3D.new()
		instance.name = "Regiment%d_%s" % [index, kind]
		instance.multimesh = mm
		instance.material_override = material
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON
		_host_root.add_child(instance)
		_soldier_materials.append(material)
	if regiment.has("banner"):
		_build_banner(origin + Vector3(0.0, 0.0, -spacing * 0.8), str(regiment["banner"]), color, float(index) * 1.7)
		if files > 10:
			var wing := float(files) * spacing * 0.4
			_build_banner(origin + Vector3(-wing, 0.0, -spacing * 0.6), "%s_pennon.png" % faction, color, float(index) * 2.3 + 1.0)
			_build_banner(origin + Vector3(wing, 0.0, -spacing * 0.6), "%s_pennon.png" % faction, color, float(index) * 3.1 + 2.0)
	return files * ranks


func _build_banner(at: Vector3, cloth_file: String, livery: Color, phase: float) -> void:
	var texture := PortraitLoader.load_texture(BANNER_DIR + cloth_file)
	var pennon := cloth_file.ends_with("_pennon.png")
	var flag_size := Vector2(3.0, 0.75) if pennon else Vector2(1.3, 2.6)
	var node := Node3D.new()
	node.position = at
	_host_root.add_child(node)
	var pole := MeshInstance3D.new()
	pole.mesh = BattleMeshes.pole()
	pole.scale = Vector3(1, BANNER_HEIGHT, 1)
	node.add_child(pole)
	var flag := MeshInstance3D.new()
	var material := ShaderMaterial.new()
	material.shader = BANNER_SHADER
	material.set_shader_parameter("livery", livery)
	material.set_shader_parameter("phase", phase)
	material.set_shader_parameter("heraldry", texture)
	material.set_shader_parameter("has_heraldry", texture != null)
	material.set_shader_parameter("full_texture", texture != null)
	material.set_shader_parameter("flag_length", flag_size.x)
	flag.mesh = BattleMeshes.flag(flag_size.x, flag_size.y)
	flag.material_override = material
	flag.position = Vector3(0.03, BANNER_HEIGHT - 0.05, 0)
	node.add_child(flag)


## Voile de fondu entre deux plans (sous l'interface : couche -1).
func _build_fade() -> void:
	_fade_layer = CanvasLayer.new()
	_fade_layer.layer = -1
	add_child(_fade_layer)
	_fade = ColorRect.new()
	_fade.color = Color(0, 0, 0, 0)
	_fade.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_fade.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_fade_layer.add_child(_fade)


# --- Plans de caméra ---------------------------------------------------------------------------


func shot_count() -> int:
	return _shots.size()


func _enter_shot(index: int) -> void:
	if _shots.is_empty():
		return
	shot_index = index % _shots.size()
	shot_time = 0.0
	var shot: Dictionary = _shots[shot_index]
	camera.fov = float(shot.get("fov", 38.0))
	_foreground.visible = bool(shot.get("host", false))
	_place_camera(_frozen_t if _frozen_t >= 0.0 else 0.0)
	shot_changed.emit(shot_index, str(shot.get("label", "")))


func _place_camera(t: float) -> void:
	if _forced_camera.size() == 6:
		camera.position = Vector3(_forced_camera[0], _forced_camera[1], _forced_camera[2])
		camera.look_at(Vector3(_forced_camera[3], _forced_camera[4], _forced_camera[5]))
		return
	if _shots.is_empty():
		return
	var shot: Dictionary = _shots[shot_index]
	var eased := t * t * (3.0 - 2.0 * t) * 0.35 + t * 0.65  # départ et arrivée adoucis, jamais figé
	var from: Dictionary = shot["from"]
	var to: Dictionary = shot["to"]
	var position := _vec3(from["position"]).lerp(_vec3(to["position"]), eased)
	var target := _vec3(from["target"]).lerp(_vec3(to["target"]), eased)
	camera.position = position
	camera.look_at(target)


func _process(delta: float) -> void:
	_anim_time += delta
	for material in _soldier_materials:
		material.set_shader_parameter("anim_time", _anim_time)
	if camera == null or _shots.is_empty() or _frozen_t >= 0.0:
		return
	shot_time += delta
	_place_camera(clampf(shot_time / _shot_seconds, 0.0, 1.0))
	if _frozen_shot < 0 and not _dipping and shot_time >= _shot_seconds - DIP_SECONDS * 0.5:
		_dip_to_next()


func _dip_to_next() -> void:
	_dipping = true
	var tween := create_tween()
	tween.tween_property(_fade, "color:a", 1.0, DIP_SECONDS * 0.5)
	tween.tween_callback(func() -> void: _enter_shot(shot_index + 1))
	tween.tween_property(_fade, "color:a", 0.0, DIP_SECONDS)
	tween.tween_callback(func() -> void: _dipping = false)


# --- Outils ------------------------------------------------------------------------------------


static func _rgb(values: Variant) -> Color:
	if values is Array and (values as Array).size() >= 3:
		return Color(float(values[0]), float(values[1]), float(values[2]))
	return Color.WHITE


static func _vec3(values: Variant) -> Vector3:
	if values is Array and (values as Array).size() >= 3:
		return Vector3(float(values[0]), float(values[1]), float(values[2]))
	return Vector3.ZERO
