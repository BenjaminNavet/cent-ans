extends SceneTree

## Lot DA1 : gros plans des armoiries portées par les figurines skinnées, hors simulation.
## Matériaux construits par `BattleSoldiers` (atlas d'armoiries, seigneur et bannerets, croix du
## commun) exactement comme en bataille. Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1600x900 --script res://tests/da1_arms_shot.gd -- \
##     --out=<png> [--fig=infantry_0,cavalry_0] [--sides=0,1] [--houses=Valois,Plantagenêt]
##     [--cols=5] [--rows=2] [--state=idle] [--cam=x,y,z,tx,ty,tz] [--back] [--no-da1]
## `--no-da1` : rendu d'avant le lot (armes de faction sur écus et caparaçons seulement).
## `--back` : caméra derrière les rangs (armes du dos).

const SKY_SHADER := preload("res://shaders/battle_sky.gdshader")
const SIDES := ["attacker", "defender"]
const FACTIONS := ["fac_france", "fac_england"]
const LIVERIES := [Color(0.16, 0.25, 0.62), Color(0.72, 0.12, 0.12)]

var _time := 3.37
var _state := "idle"
var _cols := 5
var _rows := 2


func _init() -> void:
	var out := ""
	var figs := PackedStringArray(["infantry_0", "infantry_0"])
	var sides := PackedInt32Array([0, 1])
	var houses := PackedStringArray(["Valois", "Plantagenêt"])
	var cam := PackedFloat64Array()
	var back := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--fig="):
			figs = arg.trim_prefix("--fig=").split(",")
		elif arg.begins_with("--sides="):
			sides = PackedInt32Array(Array(arg.trim_prefix("--sides=").split(",")).map(func(s: String) -> int: return int(s)))
		elif arg.begins_with("--houses="):
			houses = arg.trim_prefix("--houses=").split(",")
		elif arg.begins_with("--cols="):
			_cols = int(arg.trim_prefix("--cols="))
		elif arg.begins_with("--rows="):
			_rows = int(arg.trim_prefix("--rows="))
		elif arg.begins_with("--state="):
			_state = arg.trim_prefix("--state=")
		elif arg.begins_with("--cam="):
			cam = arg.trim_prefix("--cam=").split_floats(",")
		elif arg == "--back":
			back = true
	if out == "":
		push_error("da1_arms_shot: --out=<png> required")
		quit(1)
		return
	var world := Node3D.new()
	root.add_child(world)
	_environment(world)
	var soldiers := BattleSoldiers.new()
	world.add_child(soldiers)
	soldiers._side_colors = {"attacker": LIVERIES[0], "defender": LIVERIES[1]}
	var factions := {}
	var side_houses := {}
	for i in 2:
		factions[SIDES[i]] = FACTIONS[i]
		side_houses[SIDES[i]] = houses[i] if i < houses.size() else ""
		soldiers._side_heraldry[SIDES[i]] = PortraitLoader.heraldry_texture(FACTIONS[i])
	soldiers._build_arms_atlas(factions, side_houses)
	print("DA1_ATLAS layers=%s" % [soldiers.arms_layer_ids])
	var camera := Camera3D.new()
	camera.fov = 36.0
	world.add_child(camera)
	var x := 0.0
	for i in figs.size():
		var parts := figs[i].rsplit("_", true, 1)
		var kind := parts[0]
		var variant := int(parts[1])
		var spacing := 1.6 if kind == "cavalry" else 1.0
		var side := sides[i] if i < sides.size() else i % 2
		_rank(world, soldiers, kind, variant, SIDES[side], Vector3(x, 0, 0), spacing, 100 + i)
		x += _cols * spacing + 1.2
	var width := x - 1.2
	var cx := width * 0.5 - 0.5
	if cam.size() == 6:
		camera.look_at_from_position(Vector3(cam[0], cam[1], cam[2]), Vector3(cam[3], cam[4], cam[5]))
	else:
		var d := maxf(width * 0.95, 5.0)
		var z := -(d * 0.8 + 4.0) if back else d * 0.8 + 2.0
		camera.look_at_from_position(Vector3(cx + d * 0.2, 1.9, z), Vector3(cx, 1.1, -1.0))
	for _i in 10:
		await process_frame
	RenderingServer.force_draw()
	await process_frame
	root.get_viewport().get_texture().get_image().save_png(out)
	print("DA1_SHOT %s" % out)
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
	var we := WorldEnvironment.new()
	we.environment = env
	world.add_child(we)
	var sun := DirectionalLight3D.new()
	sun.transform = Transform3D(Basis(Vector3(-0.788011, 0, 0.615661), Vector3(0.345139, 0.828038, 0.441764), Vector3(-0.509862, 0.560526, -0.652597)), Vector3(0, 50, 0))
	sun.light_color = Color(1, 0.94, 0.84)
	sun.light_energy = 1.75
	sun.shadow_enabled = true
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


func _rank(world: Node3D, soldiers: BattleSoldiers, kind: String, variant: int, side: String, origin: Vector3, spacing: float, unit_id: int) -> void:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = BattleSkinned.mesh(kind, variant, 0)
	mm.instance_count = _cols * _rows
	for r in _rows:
		for c in _cols:
			var jitter := Vector3(sin(c * 3.1 + r) * 0.1, 0, cos(c * 1.7 + r * 2.3) * 0.12)
			var pos := origin + Vector3(c * spacing, 0, -r * spacing * (1.6 if kind == "cavalry" else 1.3)) + jitter
			mm.set_instance_transform(r * _cols + c, Transform3D(Basis.IDENTITY, pos))
	var mat := soldiers._make_skinned_material(side, kind, variant, false, unit_id)
	BattleSkinned.apply_config(mat, BattleSkinned.state_config(kind, variant, _state, false), 0.0)
	mat.set_shader_parameter("anim_time", _time)
	mat.set_shader_parameter("blend_since", -100.0)
	var inst := MultiMeshInstance3D.new()
	inst.multimesh = mm
	inst.material_override = mat
	world.add_child(inst)
