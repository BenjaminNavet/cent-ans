extends SceneTree

## Lot V2 : gros plans des figurines skinnées, hors simulation.
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1600x900 --script res://tests/v2_figures_shot.gd -- \
##     --out=<png> [--fig=infantry_0,archer_0] [--state=idle|marching|running|charging|melee|
##     shooting|routing|dead] [--time=<s>] [--cols=5] [--rows=2] [--cam=x,y,z,tx,ty,tz] [--rigid]
## `--rigid` : mêmes rangs avec les figurines à membres rigides (B1/B4), pour les « avant ».

const SKY_SHADER := preload("res://shaders/battle_sky.gdshader")
const RIGID_SHADER := preload("res://shaders/battle_soldier.gdshader")
const LIVERIES := [Color(0.16, 0.25, 0.62), Color(0.72, 0.12, 0.12)]
const FACTIONS := ["fac_france", "fac_england"]

var _time := 3.37
var _since := 4.4  # secondes depuis la dernière volée
var _state := "idle"
var _cols := 5
var _rows := 2
var _rigid := false
var _hide_pavise := false  # BV3 : `--hide-pavise` (pavois du dos masqué, rangée plantée)


func _init() -> void:
	var out := ""
	var figs := PackedStringArray(["infantry_0"])
	var cam := PackedFloat64Array()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--fig="):
			figs = arg.trim_prefix("--fig=").split(",")
		elif arg.begins_with("--state="):
			_state = arg.trim_prefix("--state=")
		elif arg.begins_with("--time="):
			_time = float(arg.trim_prefix("--time="))
		elif arg.begins_with("--since="):
			_since = float(arg.trim_prefix("--since="))
		elif arg.begins_with("--cols="):
			_cols = int(arg.trim_prefix("--cols="))
		elif arg.begins_with("--rows="):
			_rows = int(arg.trim_prefix("--rows="))
		elif arg.begins_with("--cam="):
			cam = arg.trim_prefix("--cam=").split_floats(",")
		elif arg == "--rigid":
			_rigid = true
		elif arg == "--hide-pavise":
			_hide_pavise = true
	if out == "":
		push_error("v2_figures_shot: --out=<png> required")
		quit(1)
		return
	var world := Node3D.new()
	root.add_child(world)
	_environment(world)
	var camera := Camera3D.new()
	camera.fov = 40.0
	world.add_child(camera)
	var x := 0.0
	var width := 0.0
	for i in figs.size():
		var parts := figs[i].rsplit("_", true, 1)
		var kind := parts[0]
		var variant := int(parts[1])
		var spacing := 1.6 if kind == "cavalry" else 1.1
		_rank(world, kind, variant, i % 2, Vector3(x, 0, 0), spacing)
		x += _cols * spacing + 1.5
	width = x - 1.5
	if cam.size() == 6:
		camera.look_at_from_position(Vector3(cam[0], cam[1], cam[2]), Vector3(cam[3], cam[4], cam[5]))
	else:
		var cx := width * 0.5 - 0.5
		var d := maxf(width * 1.05, 6.0)
		camera.look_at_from_position(Vector3(cx + d * 0.35, 2.2, d * 0.75 + 2.0), Vector3(cx, 0.95, -1.0))
	await process_frame
	for _i in 8:
		await process_frame
	RenderingServer.force_draw()
	await process_frame
	var img := root.get_viewport().get_texture().get_image()
	img.save_png(out)
	print("V2_SHOT %s" % out)
	quit(0)


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
	plane.size = Vector2(400, 400)
	ground.mesh = plane
	var ground_mat := StandardMaterial3D.new()
	ground_mat.albedo_color = Color(0.36, 0.38, 0.2)
	ground_mat.roughness = 1.0
	ground.material_override = ground_mat
	world.add_child(ground)


## Rang de `_cols` × `_rows` figurines, dans l'état `_state` à l'instant `_time`.
func _rank(world: Node3D, kind: String, variant: int, side: int, origin: Vector3, spacing: float) -> void:
	var skinned := not _rigid and BattleSkinned.has_figure(kind, variant)
	var corpse := _state == "dead"
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = corpse
	mm.mesh = BattleSkinned.mesh(kind, variant, 0) if skinned else BattleMeshes.soldier(kind, variant)
	mm.instance_count = _cols * _rows
	for r in _rows:
		for c in _cols:
			var jitter := Vector3(sin(c * 3.1 + r) * 0.12, 0, cos(c * 1.7 + r * 2.3) * 0.15)
			var pos := origin + Vector3(c * spacing, 0, -r * spacing * (1.6 if kind == "cavalry" else 1.25)) + jitter
			var k := r * _cols + c
			mm.set_instance_transform(k, Transform3D(Basis.IDENTITY, pos))
			if corpse:
				mm.set_instance_custom_data(k, Color(_time - 2.0 - 0.1 * k, float(k % 4), 1.0, 0.0))
	var mat := ShaderMaterial.new()
	var arms: Texture2D = PortraitLoader.heraldry_texture(FACTIONS[side])
	if skinned:
		mat.shader = BattleSkinned.SHADER
		BattleSkinned.setup_material(mat, kind, variant)
		var config := BattleSkinned.death_config(kind, variant) if corpse else BattleSkinned.state_config(kind, variant, _state, _state == "running")
		BattleSkinned.apply_config(mat, config, 0.0)
		mat.set_shader_parameter("blend_since", -100.0)
		mat.set_shader_parameter("volley_time", _time - _since)
		mat.set_shader_parameter("livery_share", 0.9)
		mat.set_shader_parameter("hide_pavise", _hide_pavise)
	else:
		mat.shader = RIGID_SHADER
		mat.set_shader_parameter("weapon_mode", BattleMeshes.weapon_mode(kind, variant))
		var mounted := kind == "cavalry"
		mat.set_shader_parameter("mounted", mounted)
		mat.set_shader_parameter("hip", Vector2(1.68, -0.05) if mounted else Vector2(0.93, 0.0))
		mat.set_shader_parameter("shoulder", Vector2(2.18, -0.05) if mounted else Vector2(1.4, 0.0))
		mat.set_shader_parameter("elbow", Vector2(1.91, -0.07) if mounted else Vector2(1.13, -0.02))
		mat.set_shader_parameter("torso_y", 0.78 if mounted else 0.0)
		mat.set_shader_parameter("torso_z", -0.05 if mounted else 0.0)
		mat.set_shader_parameter("anim_state", BattleSoldiers.anim_state({"state": _state}))
		mat.set_shader_parameter("volley_time", _time - _since)
		mat.set_shader_parameter("state_time", 3.0)
	mat.set_shader_parameter("anim_time", _time)
	mat.set_shader_parameter("livery", LIVERIES[side])
	mat.set_shader_parameter("trim", Color(0.83, 0.66, 0.24))
	mat.set_shader_parameter("heraldry", arms)
	mat.set_shader_parameter("has_heraldry", arms != null)
	var inst := MultiMeshInstance3D.new()
	inst.multimesh = mm
	inst.material_override = mat
	world.add_child(inst)
