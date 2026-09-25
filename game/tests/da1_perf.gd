extends SceneTree

## Lot DA1 : mesure A/B du coût GPU des armoiries des figurines skinnées, dans un seul processus
## (les deux rendus alternent toutes les `SWITCH` images : la charge extérieure de la machine
## pèse autant sur l'un que sur l'autre). Foule de nobles et de gens du commun en gros plan.
## Usage (avec affichage) :
##   godot --path game --resolution 1600x900 --disable-vsync --script res://tests/da1_perf.gd -- [--cycles=20]
## Sortie : `DA1_PERF {"da1_gpu_ms": …, "base_gpu_ms": …, "delta_pct": …}`.

const SIDES := ["attacker", "defender"]
const FACTIONS := ["fac_france", "fac_england"]
const LIVERIES := [Color(0.16, 0.25, 0.62), Color(0.72, 0.12, 0.12)]
const FIGS := [["infantry_0", 0], ["cavalry_0", 1], ["archer_0", 1], ["infantry_2", 0], ["infantry_0", 1], ["cavalry_0", 0]]
const SWITCH := 30
const WARMUP := 60

var _sets: Array = [[], []]  # 0 : DA1, 1 : sans DA1
var _samples: Array = [[], []]
var _base_shader: Shader = null


func _init() -> void:
	var cycles := 20
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--cycles="):
			cycles = int(arg.trim_prefix("--cycles="))
		elif arg.begins_with("--base-shader="):
			# Shader d'avant le lot (ex. `git show main:game/shaders/battle_soldier_skinned.gdshader`).
			_base_shader = Shader.new()
			_base_shader.code = FileAccess.get_file_as_string(arg.trim_prefix("--base-shader="))
	var world := Node3D.new()
	root.add_child(world)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-50, 30, 0)
	sun.shadow_enabled = true
	world.add_child(sun)
	var env := WorldEnvironment.new()
	env.environment = Environment.new()
	env.environment.background_mode = Environment.BG_COLOR
	env.environment.background_color = Color(0.6, 0.7, 0.85)
	world.add_child(env)
	var camera := Camera3D.new()
	world.add_child(camera)
	camera.look_at_from_position(Vector3(12, 3.0, 14), Vector3(12, 1.1, 0))
	for mode in 2:
		var soldiers := BattleSoldiers.new()
		soldiers.da1_enabled = mode == 0
		world.add_child(soldiers)
		soldiers._side_colors = {"attacker": LIVERIES[0], "defender": LIVERIES[1]}
		var factions := {}
		for i in 2:
			factions[SIDES[i]] = FACTIONS[i]
			soldiers._side_heraldry[SIDES[i]] = PortraitLoader.heraldry_texture(FACTIONS[i])
		soldiers._build_arms_atlas(factions, {"attacker": "Valois", "defender": "Plantagenêt"})
		for f in FIGS.size():
			var parts := str(FIGS[f][0]).rsplit("_", true, 1)
			var inst := _crowd(world, soldiers, parts[0], int(parts[1]), SIDES[int(FIGS[f][1])], f)
			inst.visible = mode == 0
			_sets[mode].append(inst)
	RenderingServer.viewport_set_measure_render_time(root.get_viewport_rid(), true)
	var mode := 0
	var frame := 0
	var total := WARMUP + cycles * 2 * SWITCH
	while frame < total:
		await process_frame
		frame += 1
		if frame > WARMUP and (frame - WARMUP) % SWITCH > 3:  # 3 images de transition ignorées
			_samples[mode].append(RenderingServer.viewport_get_measured_render_time_gpu(root.get_viewport_rid()))
		if frame > WARMUP and (frame - WARMUP) % SWITCH == 0:
			mode = 1 - mode
			for m in 2:
				for inst in _sets[m]:
					(inst as Node3D).visible = m == mode
	var da1 := _median(_samples[0])
	var base := _median(_samples[1])
	print("DA1_PERF %s" % JSON.stringify({"da1_gpu_ms": da1, "base_gpu_ms": base, "delta_pct": (da1 / maxf(base, 0.0001) - 1.0) * 100.0, "samples": _samples[0].size()}))
	quit(0)


func _median(values: Array) -> float:
	if values.is_empty():
		return 0.0
	var sorted := values.duplicate()
	sorted.sort()
	return float(sorted[sorted.size() / 2])


func _crowd(world: Node3D, soldiers: BattleSoldiers, kind: String, variant: int, side: String, index: int) -> MultiMeshInstance3D:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = BattleSkinned.mesh(kind, variant, 0)
	var cols := 12
	var rows := 6
	mm.instance_count = cols * rows
	var spacing := 1.6 if kind == "cavalry" else 1.0
	var origin := Vector3((index % 3) * 13.0, 0, -(index / 3) * 12.0)
	for r in rows:
		for c in cols:
			mm.set_instance_transform(r * cols + c, Transform3D(Basis.IDENTITY, origin + Vector3(c * spacing, 0, -r * spacing * 1.4)))
	var mat := soldiers._make_skinned_material(side, kind, variant, false, 100 + index)
	if not soldiers.da1_enabled and _base_shader != null:
		mat.shader = _base_shader
	BattleSkinned.apply_config(mat, BattleSkinned.state_config(kind, variant, "melee", false), 0.0)
	var inst := MultiMeshInstance3D.new()
	inst.multimesh = mm
	inst.material_override = mat
	world.add_child(inst)
	return inst
