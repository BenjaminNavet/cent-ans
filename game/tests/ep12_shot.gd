extends SceneTree

## Lot EP12 : gros plans hors simulation des blessés au sol et des fuyards désarmés (mêmes
## matériaux, clips et objets que `BattleSoldiers` / `BattleDroppedArms`).
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1600x900 --script res://tests/ep12_shot.gd -- \
##     --out=<png> [--scene=wounded|rout] [--time=<s>] [--cam=x,y,z,tx,ty,tz]
## `--time` : secondes écoulées depuis la chute (blessés) ou la débandade (fuyards).
## Sans `--out`, contrôle seulement (manifeste, clips, maillages marqués) : sortie `EP12_CHECK`.

const SKY_SHADER := preload("res://shaders/battle_sky.gdshader")
const LIVERY := Color(0.16, 0.25, 0.62)
const FIGURES := [["infantry", 0], ["infantry", 1], ["infantry", 2], ["archer", 0], ["archer", 1]]

var _time := 3.0
var _scene := "wounded"


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
		quit(_check())
		return
	var world := Node3D.new()
	root.add_child(world)
	_environment(world)
	var camera := Camera3D.new()
	camera.fov = 40.0
	world.add_child(camera)
	await process_frame
	var t0 := 10.0
	match _scene:
		"wounded":
			# Rangée 1 : les trois blessés d'infanterie (rampe, assis, à genoux), deux instants.
			# Rangée 2 : archers blessés ; à l'arrière : un mort ordinaire armé pour comparer.
			var rows := []
			for i in 3:
				rows.append([i, t0, BattleSkinned.CODE_UNARMED + 0.4])
			for i in 3:
				rows.append([i, t0 + _time * 0.5, BattleSkinned.CODE_UNARMED + 0.2])
			_rank(world, "infantry", 0, "wounded", rows, Vector3(0, 0, 0), 1.8)
			_rank(world, "archer", 0, "wounded", rows, Vector3(0, 0, -4.0), 1.8)
			_rank(world, "infantry", 2, "dead", [[0, t0, 0.3]], Vector3(-2.5, 0, -2.0), 1.8)
			for i in 3:
				var arms := BattleDroppedArms.new()
				world.add_child(arms)
				arms.setup(BattleGore.settings().get("dropped_arms", {}))
				arms.drop_one("sword", Vector3(i * 1.8 + 0.4, 0, 0.9), 0.3 * i, LIVERY, 11 + i)
				arms.drop_one("bow", Vector3(i * 1.8 + 0.3, 0, -3.1), 0.5 * i, LIVERY, 21 + i)
		"rout":
			# Fuyards désarmés (course de fuite) de cinq figurines, et leurs armes au sol derrière.
			var arms := BattleDroppedArms.new()
			world.add_child(arms)
			arms.setup(BattleGore.settings().get("dropped_arms", {}))
			for f in FIGURES.size():
				var kind: String = FIGURES[f][0]
				var variant: int = FIGURES[f][1]
				var origin := Vector3(f * 1.6, 0, 0)
				_rank(world, kind, variant, "routing", [[0, 0.0, 0.0], [0, 0.0, 0.0]], origin, 0.0, Vector3(0, 0, 1.4))
				var slice := PackedFloat32Array()
				for k in 6:
					var at := origin + Vector3((k % 2) * 0.7 - 0.35, 0, -3.0 - (k / 2) * 1.1)
					slice.append_array([1, 0, 0, at.x, 0, 1, 0, at.y, 0, 0, 1, at.z])
				arms.drop_from_rout(slice, 6, BattleSkinned.style_of(kind, variant), LIVERY, 100 + f)
			# Armés, pour comparer : la même rangée en marche.
			for f in FIGURES.size():
				_rank(world, FIGURES[f][0], FIGURES[f][1], "marching", [[0, 0.0, 0.0]], Vector3(f * 1.6, 0, -8.0), 0.0)
	if cam.size() == 6:
		camera.look_at_from_position(Vector3(cam[0], cam[1], cam[2]), Vector3(cam[3], cam[4], cam[5]))
	elif _scene == "rout":
		camera.look_at_from_position(Vector3(3.2, 3.4, 8.5), Vector3(3.2, 0.5, -2.5))
	else:
		camera.look_at_from_position(Vector3(2.0, 4.6, 6.0), Vector3(2.0, 0.2, -1.5))
	for _i in 8:
		await process_frame
	RenderingServer.force_draw()
	await process_frame
	var img := root.get_viewport().get_texture().get_image()
	img.save_png(out)
	print("EP12_SHOT %s" % out)
	quit(0)


## Contrôle sans rendu : clips du rig, configurations de déroute et de blessés, faces marquées
## `HELD_MASK` (bit 6 du masque de variante, UV2.x) dans les maillages des figurines à pied.
func _check() -> int:
	var ok := true
	for kind_variant in FIGURES:
		var kind: String = kind_variant[0]
		var variant: int = kind_variant[1]
		var wounded := BattleSkinned.wounded_config(kind, variant)
		var routing := BattleSkinned.state_config(kind, variant, "routing", false)
		var held := 0
		var mesh := BattleSkinned.mesh(kind, variant, 0)
		if mesh != null:
			var uv2: PackedVector2Array = mesh.surface_get_arrays(0)[Mesh.ARRAY_TEX_UV2]
			for v in uv2:
				if (int(v.x + 0.5) & 64) != 0:
					held += 1
		var good := not wounded.is_empty() and (routing["names"] as Array).has("flee") and held > 0
		ok = ok and good
		print("EP12_CHECK %s_%d wounded=%s routing=%s held_vertices=%d %s" % [kind, variant, wounded.get("set", []), routing["names"], held, "OK" if good else "FAIL"])
	return 0 if ok else 1


## Rangée de figurines : `rows` = [[clip dans le jeu, instant de chute, code + sang], ...] ;
## `mode` wounded (couche des blessés), dead (cadavres), routing (fuyards désarmés), marching.
func _rank(world: Node3D, kind: String, variant: int, mode: String, rows: Array, origin: Vector3, spacing: float, step := Vector3.ZERO) -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	var custom := mode == "wounded" or mode == "dead"
	mm.use_custom_data = custom
	mm.mesh = BattleSkinned.mesh(kind, variant, 0)
	mm.instance_count = rows.size()
	for k in rows.size():
		var row: Array = rows[k]
		var at := origin + Vector3(k * spacing, 0, 0) + step * k
		mm.set_instance_transform(k, Transform3D(Basis.IDENTITY, at))
		if custom:
			mm.set_instance_custom_data(k, Color(float(row[1]), float(row[0]), 0.0, float(row[2])))
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	BattleSkinned.setup_material(mat, kind, variant)
	var config := BattleSkinned.state_config(kind, variant, "marching", false)
	match mode:
		"wounded":
			config = BattleSkinned.wounded_config(kind, variant)
		"dead":
			config = BattleSkinned.death_config(kind, variant)
		"routing":
			config = BattleSkinned.state_config(kind, variant, "routing", false)
			mat.set_shader_parameter("drop_arms", true)
	BattleSkinned.apply_config(mat, config, 0.0)
	mat.set_shader_parameter("blend_since", -100.0)
	mat.set_shader_parameter("anim_time", 10.0 + _time)
	mat.set_shader_parameter("livery", LIVERY)
	mat.set_shader_parameter("trim", Color(0.83, 0.66, 0.24))
	mat.set_shader_parameter("livery_share", 0.9)
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
