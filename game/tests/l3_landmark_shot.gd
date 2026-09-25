extends SceneTree

## Lot L3 — gros plans des maquettes de villes emblématiques (matériaux), fenêtré :
##   godot --path game --script res://tests/l3_landmark_shot.gd -- --model=paris_siege
##     --cam=x,y,z --target=x,y,z --out=<png> [--fov=40] [--mpu=1] [--flat] [--sun=az,el]
## `--model` : nom d'un GLB de `assets/models/landmarks/` ; les matériaux importés sont remplacés
## par `landmark.gdshader` comme en jeu (`--flat` : couleurs unies, rendu d'avant L3).

const SHADER := preload("res://shaders/landmark.gdshader")

var _args := {}


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--") and arg.contains("="):
			var pair := arg.trim_prefix("--").split("=", true, 1)
			_args[pair[0]] = pair[1]
		elif arg.begins_with("--"):
			_args[arg.trim_prefix("--")] = "1"
	_run.call_deferred()
	create_timer(90.0).timeout.connect(func() -> void: quit(1))


func _vec(key: String, fallback: Vector3) -> Vector3:
	if not _args.has(key):
		return fallback
	var parts: PackedStringArray = str(_args[key]).split(",")
	return Vector3(float(parts[0]), float(parts[1]), float(parts[2]))


func _run() -> void:
	var world := Node3D.new()
	root.add_child(world)
	current_scene = world
	var path := "res://assets/models/landmarks/%s.glb" % str(_args.get("model", "paris_siege"))
	var model := (load(path) as PackedScene).instantiate() as Node3D
	world.add_child(model)
	var height := Image.create(4, 4, false, Image.FORMAT_RF)
	var height_texture := ImageTexture.create_from_image(height)
	for child in model.find_children("*", "MeshInstance3D", true, false):
		var mesh_instance := child as MeshInstance3D
		for surface in mesh_instance.mesh.get_surface_count():
			var source := mesh_instance.mesh.surface_get_material(surface)
			var material := ShaderMaterial.new()
			material.shader = SHADER
			if source is BaseMaterial3D:
				material.set_shader_parameter("albedo", (source as BaseMaterial3D).albedo_color)
				material.set_shader_parameter("roughness", (source as BaseMaterial3D).roughness)
				material.set_shader_parameter("water", 1.0 if source.resource_name == "Water" else 0.0)
			material.set_shader_parameter("tint_strength", float(_args.get("tint", "0")))
			material.set_shader_parameter("height_map", height_texture)
			material.set_shader_parameter("map_origin", Vector2(-1.0e5, -1.0e5))
			material.set_shader_parameter("map_extent", 2.0e5)
			material.set_shader_parameter("meters_per_unit", float(_args.get("mpu", "1")))
			material.set_shader_parameter("material_detail", 0.0 if _args.has("flat") else 1.0)
			material.set_shader_parameter("debug_view", int(_args.get("debug", "0")))
			mesh_instance.set_surface_override_material(surface, material)
	var env := WorldEnvironment.new()
	var environment := Environment.new()
	var sky := Sky.new()
	sky.sky_material = ProceduralSkyMaterial.new()
	environment.background_mode = Environment.BG_SKY
	environment.sky = sky
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	environment.tonemap_mode = Environment.TONE_MAPPER_AGX
	environment.ssao_enabled = true
	env.environment = environment
	world.add_child(env)
	var sun := DirectionalLight3D.new()
	var sun_dir := _vec("sun", Vector3(-35.0, 40.0, 0.0))
	sun.rotation_degrees = Vector3(-sun_dir.y, sun_dir.x, 0.0)
	sun.shadow_enabled = true
	sun.directional_shadow_max_distance = 800.0
	world.add_child(sun)
	var camera := Camera3D.new()
	camera.fov = float(_args.get("fov", "40"))
	camera.far = 20000.0
	world.add_child(camera)
	camera.position = _vec("cam", Vector3(-120, 60, 160))
	camera.look_at(_vec("target", Vector3(0, 20, 0)))
	camera.make_current()
	for i in 20:
		await process_frame
	await RenderingServer.frame_post_draw
	var out := str(_args.get("out", "user://l3_shot.png"))
	var image := root.get_texture().get_image()
	print("l3_landmark_shot: %s (%s)" % [out, error_string(image.save_png(out))])
	quit()
