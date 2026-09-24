extends SceneTree

## Lot BV1 : captures des volées, traits fichés, sang et poussière, hors simulation, sur un sol plat.
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1600x900 --script res://tests/bv1_shot.gd -- \
##     --out=<png> --shot=volley|sky|stuck|bolts|fire|blood|dust [--blood=0|1|2] [--ground=dry|muddy|snowy]
## Les « tirs » sont des dictionnaires au format de `BattleSim.get_shots()` ; les régiments, au
## format de `get_units()`.

const SKY_SHADER := preload("res://shaders/battle_sky.gdshader")

var _time := 0.0
var _effects: BattleEffects = null
var _blood: BattleBlood = null
var _camera: Camera3D = null


func _init() -> void:
	var out := ""
	var shot := "volley"
	var blood_level := 2
	var ground := "dry"
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--shot="):
			shot = arg.trim_prefix("--shot=")
		elif arg.begins_with("--blood="):
			blood_level = int(arg.trim_prefix("--blood="))
		elif arg.begins_with("--ground="):
			ground = arg.trim_prefix("--ground=")
	if out == "":
		push_error("bv1_shot: --out=<png> required")
		quit(1)
		return
	var world := Node3D.new()
	root.add_child(world)
	_environment(world, ground)
	_camera = Camera3D.new()
	_camera.fov = 45.0
	_camera.far = 3000.0
	world.add_child(_camera)
	var flat := func(_x: float, _z: float) -> float: return 0.0
	var dry := func(_x: float, _z: float) -> int: return 0
	_effects = BattleEffects.new()
	world.add_child(_effects)
	_effects.setup("clear", flat, dry)
	_effects.configure_ground(ground, "clear")
	_blood = BattleBlood.new()
	_effects.add_child(_blood)
	_blood.setup(flat, blood_level, dry)
	_effects.hit_landed.connect(func(pos: Vector3, time: float) -> void: _blood.add_hit(pos, time, _camera.position))
	var archers := _unit(1, "attacker", "unit_longbowmen", Vector3(0, 0, 0), 0.0, 120, 40.0, 3.0)
	var crossbows := _unit(2, "attacker", "unit_genoese_crossbowmen", Vector3(60, 0, 20), 0.0, 100, 30.0, 3.0)
	var enemy := _unit(3, "defender", "unit_men_at_arms_foot", Vector3(0, 0, 170), PI, 200, 50.0, 4.0)
	var by_id := {1: archers, 2: crossbows, 3: enemy}
	var units := [archers, crossbows, enemy]
	var frames := 20
	match shot:
		"volley", "sky":
			# Trois volées d'archers, vues de derrière les archers (« volley ») ou d'en dessous.
			for v in 3:
				_advance(0.9, units)
				_fire(_shot(1, 3, 120, 2.0, "arrow", false, "none"), by_id)
			_advance(1.6, units)
			if shot == "volley":
				_camera.look_at_from_position(Vector3(-14, 6, -22), Vector3(4, 16, 80))
			else:
				_camera.look_at_from_position(Vector3(10, 1.7, 95), Vector3(-2, 20, 60))
		"stuck":
			# Une minute de tir : le champ se couvre de flèches ; certaines dans les pieux.
			enemy["stakes"] = true
			for v in 12:
				_advance(5.0, units)
				_fire(_shot(1, 3, 120, 3.0, "arrow", false, "stakes" if v > 3 else "none"), by_id)
			_advance(8.0, units)
			_camera.look_at_from_position(Vector3(14, 3.2, 142), Vector3(-2, 0, 168))
		"bolts":
			# Carreaux génois sur des arbalétriers derrière leurs pavois.
			enemy["pavise_cover"] = true
			enemy["type"] = "unit_crossbowmen"
			for v in 6:
				_advance(4.0, units)
				_fire(_shot(2, 3, 100, 1.0, "bolt", false, "pavise"), by_id)
			_advance(0.9, units)
			_camera.look_at_from_position(Vector3(22, 3.5, 150), Vector3(0, 1.0, 166))
		"fire":
			# Flèches enflammées (assiégeants), à la tombée de la courbe.
			for v in 3:
				_advance(1.1, units)
				_fire(_shot(1, 3, 120, 0.0, "arrow", true, "none"), by_id)
			_advance(2.4, units)
			_camera.look_at_from_position(Vector3(22, 4, 120), Vector3(0, 8, 160))
		"blood":
			# Mêlée : pertes répétées au premier rang, puis déroute (traînées).
			enemy["state"] = "melee"
			for step in 40:
				_advance(0.5, units)
				enemy["soldiers"] = int(enemy["soldiers"]) - 2
			enemy["state"] = "routing"
			enemy["facing"] = 0.0
			for step in 20:
				enemy["z"] = float(enemy["z"]) + 1.5
				_advance(0.5, units)
			_advance(3.0, units)
			_camera.look_at_from_position(Vector3(12, 5, 152), Vector3(0, 0, 172))
		"dust":
			# Chevaliers à la charge (poussière selon le sol, mottes projetées).
			var knights := _unit(4, "attacker", "unit_knights", Vector3(0, 0, 40), 0.0, 60, 40.0, 7.0)
			knights["render"] = "cavalry"
			knights["state"] = "charging"
			units.append(knights)
			_camera.look_at_from_position(Vector3(14, 2.2, 58), Vector3(0, 1.0, 70))
			# Les particules vivent en temps réel : la charge avance image par image.
			for step in 100:
				knights["z"] = float(knights["z"]) + 0.3
				_advance(1.0 / 30.0, units)
				await process_frame
			frames = 1
	for i in frames:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(out.get_base_dir())
	var err := image.save_png(out)
	print("bv1_shot: %s (%s), %d arrows, %d stuck, %d blood decals" % [out, error_string(err), _effects.volleys.launched, _effects.volleys.stuck_count, _blood.decal_count])
	quit(0 if err == OK else 1)


func _advance(seconds: float, units: Array) -> void:
	var steps := maxi(int(seconds * 30.0), 1)
	for i in steps:
		_time += seconds / steps
		_effects.update(units, null, _time, seconds / steps, _camera.position, [])
		_blood.tick_time(_time)
		_blood.update(units, _camera.position)


func _fire(shot: Dictionary, by_id: Dictionary) -> void:
	_effects.update(by_id.values(), null, _time, 0.0, _camera.position, [shot])


static func _unit(id: int, side: String, type: String, pos: Vector3, facing: float, soldiers: int, width: float, depth: float) -> Dictionary:
	return {
		"id": id, "side": side, "type": type, "render": "archer", "present": true, "state": "idle",
		"x": pos.x, "y": pos.y, "z": pos.z, "facing": facing, "width": width, "depth": depth,
		"soldiers": soldiers, "figures": soldiers, "ammo": 20, "running": false,
	}


static func _shot(shooter: int, target: int, missiles: int, kills: float, kind: String, fire: bool, cover: String) -> Dictionary:
	return {
		"time": 0.0, "shooter": shooter, "target": target, "from": Vector2.ZERO, "aim": Vector2(0, 170),
		"missiles": missiles, "kills": kills, "kind": kind, "incendiary": fire, "cover": cover,
	}


func _environment(world: Node3D, ground_kind: String) -> void:
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
	sun.rotation = Vector3(deg_to_rad(-48), deg_to_rad(-35), 0)
	sun.light_color = Color(1, 0.94, 0.84)
	sun.light_energy = 1.6
	sun.shadow_enabled = true
	world.add_child(sun)
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(800, 800)
	ground.mesh = plane
	var mat := StandardMaterial3D.new()
	match ground_kind:
		"snowy":
			mat.albedo_color = Color(0.86, 0.88, 0.92)
		"muddy":
			mat.albedo_color = Color(0.3, 0.24, 0.16)
		_:
			mat.albedo_color = Color(0.4, 0.4, 0.22)
	mat.roughness = 1.0
	ground.material_override = mat
	world.add_child(ground)
