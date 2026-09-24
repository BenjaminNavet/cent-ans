extends SceneTree

## Lot B1 : gros plans des figurines de bataille (avant/après), hors simulation.
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1600x900 --script res://tests/b1_figures_shot.gd -- \
##     --out=<png> [--shot=knights|archers|infantry|gallop] [--legacy-figures]
## `--legacy-figures` rend les figurines procédurales du lot V4 (capture « avant »).

const SOLDIER_SHADER := preload("res://shaders/battle_soldier.gdshader")
const SKY_SHADER := preload("res://shaders/battle_sky.gdshader")
const FRANCE := Color(0.16, 0.25, 0.62)
const ENGLAND := Color(0.72, 0.12, 0.12)


func _init() -> void:
	var out := ""
	var shot := "knights"
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--shot="):
			shot = arg.trim_prefix("--shot=")
	if out == "":
		push_error("b1_figures_shot: --out=<png> required")
		quit(1)
		return
	var world := Node3D.new()
	root.add_child(world)
	_environment(world)
	var camera := Camera3D.new()
	camera.fov = 40.0
	world.add_child(camera)
	match shot:
		"archers":
			# Archers anglais au tir, arbalétriers génois en face, de trois quarts.
			_rank(world, "archer", 0, ENGLAND, "fac_england", 4, Vector3(-1.5, 0, 0), 5, 2, 1.3)
			_rank(world, "archer", 2, FRANCE, "fac_france", 4, Vector3(5.5, 0, 4.0), 4, 2, 1.3, PI)
			camera.look_at_from_position(Vector3(-4.5, 2.1, 4.0), Vector3(2.0, 1.2, 0.3))
		"infantry":
			_rank(world, "infantry", 0, FRANCE, "fac_france", 0, Vector3(-2.0, 0, 0), 5, 2, 1.1)
			_rank(world, "infantry", 1, FRANCE, "fac_flanders", 0, Vector3(4.5, 0, 0), 4, 2, 1.1)
			camera.look_at_from_position(Vector3(2.0, 2.2, 7.0), Vector3(1.8, 1.1, 0.0))
		"gallop":
			_rank(world, "cavalry", 0, FRANCE, "fac_france", 2, Vector3(-3.0, 0, 0), 4, 2, 2.2)
			camera.look_at_from_position(Vector3(9.0, 2.4, 3.0), Vector3(0.0, 1.3, 1.0))
		_:
			# Chevaliers français au pas (caparaçons armoriés) et archers montés anglais.
			_rank(world, "cavalry", 0, FRANCE, "fac_france", 1, Vector3(-3.0, 0, 0), 3, 2, 2.2)
			_rank(world, "cavalry", 2, ENGLAND, "fac_england", 0, Vector3(4.0, 0, -6.0), 3, 1, 2.4)
			camera.look_at_from_position(Vector3(5.0, 2.3, 5.2), Vector3(0.6, 1.3, -1.0))
	for _i in 30:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(out.get_base_dir())
	var err := image.save_png(out)
	print("b1_figures_shot: %s (%s)" % [out, error_string(err)])
	quit(0 if err == OK else 1)


func _environment(world: Node3D) -> void:
	var sky_mat := ShaderMaterial.new()
	sky_mat.shader = SKY_SHADER
	var sky := Sky.new()
	sky.sky_material = sky_mat
	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	env.tonemap_mode = Environment.TONE_MAPPER_AGX
	env.ssao_enabled = true
	env.ssao_radius = 1.6
	env.ssao_intensity = 1.8
	env.adjustment_enabled = true
	env.adjustment_contrast = 1.06
	env.adjustment_saturation = 1.08
	var we := WorldEnvironment.new()
	we.environment = env
	world.add_child(we)
	var sun := DirectionalLight3D.new()
	sun.transform = Transform3D(Basis(Vector3(-0.788011, 0, 0.615661), Vector3(0.345139, 0.828038, 0.441764), Vector3(-0.509862, 0.560526, -0.652597)), Vector3(0, 50, 0))
	sun.light_color = Color(1, 0.94, 0.84)
	sun.light_energy = 1.75
	sun.shadow_enabled = true
	sun.shadow_bias = 0.03
	sun.shadow_normal_bias = 1.4
	sun.directional_shadow_max_distance = 60.0
	world.add_child(sun)
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(200, 200)
	ground.mesh = plane
	var ground_mat := StandardMaterial3D.new()
	ground_mat.albedo_color = Color(0.3, 0.38, 0.17)
	ground_mat.roughness = 1.0
	ground.material_override = ground_mat
	world.add_child(ground)


## Rang de `cols` × `rows` figurines de `kind`/`variant` à l'état d'animation `state`.
func _rank(world: Node3D, kind: String, variant: int, livery: Color, faction: String, state: int, origin: Vector3, cols: int, rows: int, spacing: float, yaw: float = 0.0) -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = BattleMeshes.soldier(kind, variant)
	mm.instance_count = cols * rows
	var basis := Basis(Vector3.UP, yaw)
	for r in rows:
		for c in cols:
			var jitter := Vector3(sin(c * 3.1 + r) * 0.15, 0, cos(c * 1.7 + r * 2.3) * 0.2)
			var pos := origin + basis * (Vector3(c * spacing, 0, -r * spacing * (1.6 if kind == "cavalry" else 1.2)) + jitter)
			mm.set_instance_transform(r * cols + c, Transform3D(basis, pos))
	var mat := ShaderMaterial.new()
	mat.shader = SOLDIER_SHADER
	mat.set_shader_parameter("livery", livery)
	mat.set_shader_parameter("trim", Color(0.83, 0.66, 0.24))
	var arms: Texture2D = PortraitLoader.heraldry_texture(faction)
	mat.set_shader_parameter("heraldry", arms)
	mat.set_shader_parameter("has_heraldry", arms != null)
	mat.set_shader_parameter("weapon_mode", BattleMeshes.weapon_mode(kind, variant))
	var mounted := kind == "cavalry"
	mat.set_shader_parameter("mounted", mounted)
	mat.set_shader_parameter("hip", Vector2(1.68, -0.05) if mounted else Vector2(0.93, 0.0))
	mat.set_shader_parameter("shoulder", Vector2(2.18, -0.05) if mounted else Vector2(1.4, 0.0))
	mat.set_shader_parameter("torso_y", 0.78 if mounted else 0.0)
	mat.set_shader_parameter("torso_z", -0.05 if mounted else 0.0)
	mat.set_shader_parameter("livery_share", 0.7 if variant == 0 and kind != "archer" else 0.4)
	mat.set_shader_parameter("anim_state", state)
	mat.set_shader_parameter("anim_time", 3.37)
	var instance := MultiMeshInstance3D.new()
	instance.multimesh = mm
	instance.material_override = mat
	world.add_child(instance)
