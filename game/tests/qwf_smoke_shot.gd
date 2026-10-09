extends SceneTree

## QW-F : A/B de la fumée de foyer sans le noyau Rust. Deux colonnes de fumée (gauche : réglages
## d'avant, droite : `data/fx/siege_fire.json`) au-dessus d'une flamme orange simulée (sphère
## émissive + lumière ponctuelle), sous un soleil. Usage (avec affichage, via tools/godot_bg.sh) :
##   --script res://tests/qwf_smoke_shot.gd -- --out=<fichier.png>
const SMOKE_SHADER := preload("res://shaders/fire_smoke.gdshader")
const FLIPBOOK := "res://assets/textures/fx/smoke_flipbook.png"


func _init() -> void:
	var out := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
	var spec: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://../data/fx/siege_fire.json"))
	var smoke: Dictionary = spec["smoke"]
	var old := smoke.duplicate()
	old["color"] = [0.26, 0.25, 0.235, 0.9]
	for key in ["age_darken", "young_density", "point_light_share", "ember_glow_energy"]:
		old.erase(key)
	var window := Window.new()
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color(0.45, 0.55, 0.68)
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.5, 0.55, 0.62)
	env.ambient_light_energy = 0.8
	var we := WorldEnvironment.new()
	we.environment = env
	root.add_child(we)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-50, 30, 0)
	root.add_child(sun)
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(200, 100)
	ground.mesh = plane
	var gm := StandardMaterial3D.new()
	gm.albedo_color = Color(0.35, 0.4, 0.25)
	ground.material_override = gm
	root.add_child(ground)
	for side in [-1.0, 1.0]:
		var base := Node3D.new()
		base.position = Vector3(side * 22.0, 0, 0)
		root.add_child(base)
		var glow := MeshInstance3D.new()
		var sphere := SphereMesh.new()
		sphere.radius = 2.5
		sphere.height = 5.0
		glow.mesh = sphere
		var fm := StandardMaterial3D.new()
		fm.albedo_color = Color(1.0, 0.5, 0.1)
		fm.emission_enabled = true
		fm.emission = Color(1.0, 0.5, 0.1)
		fm.emission_energy_multiplier = 3.0
		glow.material_override = fm
		glow.position.y = 3.0
		base.add_child(glow)
		var light := OmniLight3D.new()
		light.light_color = Color(1.0, 0.52, 0.18)
		light.light_energy = 5.0
		light.omni_range = 30.0
		light.position.y = 7.0
		base.add_child(light)
		base.add_child(_smoke(old if side < 0 else smoke))
	var camera := Camera3D.new()
	camera.position = Vector3(0, 22, 75)
	camera.look_at_from_position(camera.position, Vector3(0, 18, 0))
	camera.fov = 55
	root.add_child(camera)
	await process_frame
	await create_timer(3.0).timeout
	var image := root.get_texture().get_image()
	image.resize(640, int(640.0 * image.get_height() / image.get_width()))
	print("qwf_smoke_shot: %s (%s)" % [out, error_string(image.save_png(out))])
	quit(0)


func _smoke(spec: Dictionary) -> GPUParticles3D:
	var particles := GPUParticles3D.new()
	particles.amount = int(spec["amount"])
	particles.lifetime = float(spec["lifetime_s"])
	particles.preprocess = particles.lifetime * 0.5
	particles.visibility_aabb = AABB(Vector3(-40, -5, -40), Vector3(80, 60, 80))
	particles.randomness = 1.0
	particles.seed = 7
	particles.use_fixed_seed = true
	var process := ParticleProcessMaterial.new()
	process.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
	process.emission_sphere_radius = float(spec["spread_m"])
	process.direction = Vector3.UP
	process.spread = 15.0
	process.initial_velocity_min = float(spec["velocity_m_s"][0])
	process.initial_velocity_max = float(spec["velocity_m_s"][1])
	process.gravity = Vector3(0, 0.5, 0)
	process.scale_min = float(spec["size_m"][0])
	process.scale_max = float(spec["size_m"][1])
	process.angle_min = -180.0
	process.angle_max = 180.0
	var c: Array = spec["color"]
	var ramp := Gradient.new()
	ramp.set_color(0, Color(c[0], c[1], c[2], c[3]))
	ramp.set_color(1, Color(c[0], c[1], c[2], c[3] * 0.6))
	var ramp_texture := GradientTexture1D.new()
	ramp_texture.gradient = ramp
	process.color_ramp = ramp_texture
	particles.process_material = process
	var quad := QuadMesh.new()
	quad.size = Vector2.ONE
	var material := ShaderMaterial.new()
	material.shader = SMOKE_SHADER
	material.set_shader_parameter("flipbook", load(FLIPBOOK))
	material.set_shader_parameter("smoke_color", Color(c[0], c[1], c[2], 1.0))
	for key in ["age_darken", "young_density", "point_light_share", "ember_glow_energy"]:
		if spec.has(key):
			material.set_shader_parameter(key, float(spec[key]))
	quad.material = material
	particles.draw_pass_1 = quad
	particles.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	return particles
