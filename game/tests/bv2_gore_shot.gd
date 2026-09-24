extends SceneTree

## Lot BV2 : gros plans hors simulation des morts, renversés, cavaliers désarçonnés, sang et
## démembrements (mêmes matériaux et couches que `BattleSoldiers` / `BattleGore`).
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1600x900 --script res://tests/bv2_gore_shot.gd -- \
##     --out=<png> [--scene=deaths|sever|knock|horse|spray] [--time=<s>] [--blood=off|moderate|full]
##     [--cam=x,y,z,tx,ty,tz]
## `--time` : secondes écoulées depuis la chute (ou l'impact).

const SKY_SHADER := preload("res://shaders/battle_sky.gdshader")
const LIVERY := Color(0.16, 0.25, 0.62)

var _time := 3.0
var _scene := "deaths"


func _init() -> void:
	var out := ""
	var cam := PackedFloat64Array()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--scene="):
			_scene = arg.trim_prefix("--scene=")
		elif arg.begins_with("--time="):
			_time = float(arg.trim_prefix("--time="))
		elif arg.begins_with("--cam="):
			cam = arg.trim_prefix("--cam=").split_floats(",")
	if out == "":
		push_error("bv2_gore_shot: --out=<png> required")
		quit(1)
		return
	var world := Node3D.new()
	root.add_child(world)
	_environment(world)
	var camera := Camera3D.new()
	camera.fov = 40.0
	world.add_child(camera)
	var gore := BattleGore.new()
	world.add_child(gore)
	await process_frame
	var t0 := 10.0
	match _scene:
		"deaths":
			# Quatre clips de mort à pied (projection croissante), sang croissant.
			var names := ["death", "death_m", "death_knees", "death_back", "death_back"]
			var rows := []
			for i in names.size():
				rows.append([BattleSkinned.death_index("infantry", 0, names[i]), [0.0, 0.3, 0.0, 3.0, 7.0][i], 0.25 * i])
			_rank(world, "infantry", 0, "dead", rows, t0, Vector3(0, 0, 0))
			_rank(world, "archer", 0, "dead", rows, t0 + 0.3, Vector3(0, 0, -3.0))
		"sever":
			var rows := []
			for code in [1, 2, 3, 4, 5]:
				rows.append([BattleSkinned.death_index("infantry", 0, ["death", "death_back", "death_m", "death_knees", "death"][code - 1]), 0.5, float(code) + 0.8])
			_rank(world, "infantry", 0, "dead", rows, t0, Vector3(0, 0, 0))
			_rank(world, "infantry", 2, "dead", rows, t0 + 0.2, Vector3(0, 0, -3.0))
			for i in 5:
				gore.sever(Vector3(i * 1.4, 0, 1.2), Vector3(1, 0, -0.3), ["head", "arm_r", "arm_l", "leg_r", "head"][i], 1.5)
		"knock":
			var rows := []
			for i in 5:
				rows.append([0, 2.0 + i * 0.6, 0.2])
			_rank(world, "infantry", 2, "knock", rows, t0, Vector3(0, 0, 0))
			_rank(world, "archer", 0, "knock", rows, t0 + 0.4, Vector3(0, 0, -3.0))
		"horse":
			var rows := []
			for clip in ["c_fall", "c_fall", "c_death_m", "c_death", "c_fall"]:
				var code := BattleSkinned.CODE_HORSE_FLEES if clip == "c_fall" else 0
				rows.append([BattleSkinned.death_index("cavalry", 0, clip), 0.0, float(code) + 0.7])
			_rank(world, "cavalry", 0, "dead", rows, t0, Vector3(0, 0, 0))
		"spray":
			var rows := []
			for i in 5:
				rows.append([0, 0.0, 0.0])
			_rank(world, "infantry", 0, "living", rows, t0, Vector3(0, 0, 0))
			for i in 5:
				gore.spray(Vector3(i * 1.4, 0, 0.3), Vector3(0, 0, 1), 30, 1.3)
	# Horloge : les événements du gore partent à anim_time = 0 ; on avance à `_time`.
	gore.update(_time, Vector3(3, 2, 6))
	if OS.get_cmdline_user_args().has("--debug-pieces"):
		for child in gore.get_children():
			if str(child.name).begins_with("Pieces"):
				(child as MultiMeshInstance3D).material_override = null
				print(child.name, " ", (child as MultiMeshInstance3D).multimesh.instance_count, " ", (child as MultiMeshInstance3D).multimesh.visible_instance_count, " ", (child as MultiMeshInstance3D).visible)
	if OS.get_cmdline_user_args().has("--debug"):
		print("BV2 pieces %d drops %d" % [gore.pieces_emitted, gore.drops_emitted], " ", (gore._pieces[0]["mm"] as MultiMesh).buffer.slice(0, 20))
	if cam.size() == 6:
		camera.look_at_from_position(Vector3(cam[0], cam[1], cam[2]), Vector3(cam[3], cam[4], cam[5]))
	else:
		camera.look_at_from_position(Vector3(5.5, 3.2, 8.0), Vector3(3.0, 0.4, -1.2))
	for _i in 8:
		await process_frame
	RenderingServer.force_draw()
	await process_frame
	var img := root.get_viewport().get_texture().get_image()
	img.save_png(out)
	print("BV2_SHOT %s" % out)
	quit(0)


## Rang de figurines : `rows` = [[clip dans le jeu, impulsion m/s, code + sang], ...] ;
## `mode` dead (cadavres), knock (renversés, clip `knockdown`), living (garde, sang du régiment).
func _rank(world: Node3D, kind: String, variant: int, mode: String, rows: Array, t0: float, origin: Vector3) -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = mode != "living"
	mm.mesh = BattleSkinned.mesh(kind, variant, 0)
	mm.instance_count = rows.size()
	var spacing := 2.2 if kind == "cavalry" else 1.4
	for k in rows.size():
		var row: Array = rows[k]
		mm.set_instance_transform(k, Transform3D(Basis.IDENTITY, origin + Vector3(k * spacing, 0, 0)))
		if mode != "living":
			mm.set_instance_custom_data(k, Color(t0, float(row[0]), float(row[1]), float(row[2])))
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER if mode == "living" else BattleSkinned.corpse_shader()
	BattleSkinned.setup_material(mat, kind, variant)
	var config := BattleSkinned.state_config(kind, variant, "idle", false)
	if mode == "dead":
		config = BattleSkinned.death_config(kind, variant)
	elif mode == "knock":
		config = BattleSkinned.knockdown_config(kind, variant)
		mat.set_shader_parameter("tumble", true)
	BattleSkinned.apply_config(mat, config, 0.0)
	mat.set_shader_parameter("blend_since", -100.0)
	mat.set_shader_parameter("anim_time", t0 + _time)
	mat.set_shader_parameter("livery", LIVERY)
	mat.set_shader_parameter("trim", Color(0.83, 0.66, 0.24))
	mat.set_shader_parameter("livery_share", 0.9)
	if mode == "living":
		mat.set_shader_parameter("blood", 0.8)
	var inst := MultiMeshInstance3D.new()
	inst.multimesh = mm
	inst.material_override = mat
	world.add_child(inst)


func _environment(world: Node3D) -> void:
	var sky_mat := ShaderMaterial.new()
	sky_mat.shader = SKY_SHADER
	var sky := Sky.new()
	sky.sky_material = sky_mat
	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.tonemap_mode = Environment.TONE_MAPPER_AGX
	var we := WorldEnvironment.new()
	we.environment = env
	world.add_child(we)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-50, 35, 0)
	sun.light_energy = 1.6
	sun.shadow_enabled = true
	world.add_child(sun)
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(200, 200)
	ground.mesh = plane
	var ground_mat := StandardMaterial3D.new()
	ground_mat.albedo_color = Color(0.36, 0.38, 0.2)
	ground_mat.roughness = 1.0
	ground.material_override = ground_mat
	world.add_child(ground)
