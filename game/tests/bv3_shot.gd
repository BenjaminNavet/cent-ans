extends SceneTree

## Lot BV3 : captures hors simulation sur un vrai terrain de bataille (prairie plate, herbe V4).
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1600x900 --script res://tests/bv3_shot.gd -- \
##     --out=<png> --shot=grass [--blood=0|1|2] [--no-bv3] [--cam=x,y,z,tx,ty,tz]
## `grass` : un champ de mêlée (herbe foulée), des morts sur l'herbe et leur sang.
## `standards` : étendards portés dans le vent ; `duel` : duel apparié (4 captures) ;
## `atlas` / `impostor` : imposteurs lointains (`--fig=infantry_0`).
## `--no-bv3` : même scène sans herbe couchée (captures « avant »).

const SKY_SHADER := preload("res://shaders/battle_sky.gdshader")


## Simulation factice : `get_soldier_buffer` rend les figurines rangées des régiments du décor.
class FakeBattle:
	extends RefCounted
	var units: Array = []

	func get_soldier_buffer(side: String, kind: String) -> PackedFloat32Array:
		var out := PackedFloat32Array()
		for unit in units:
			if str(unit["side"]) != side or str(unit["render"]) != kind or not bool(unit["present"]):
				continue
			var facing := float(unit["facing"])
			var c := cos(facing)
			var s := sin(facing)
			var n := int(unit["figures"])
			var files := int(ceil(float(unit["width"]) / (2.6 if kind == "cavalry" else 1.0)))
			for i in n:
				var fx := (float(i % files) - (files - 1) * 0.5) * float(unit["width"]) / files
				var fz := (float(i / files) + 0.5) * (2.8 if kind == "cavalry" else 1.1) - float(unit["depth"]) * 0.5
				var x := float(unit["x"]) + c * fx - s * fz
				var z := float(unit["z"]) - s * fx - c * fz
				out.append_array(PackedFloat32Array([c, 0, s, x, 0, 1, 0, float(unit["y"]), -s, 0, c, z]))
		return out


var _center := Vector3(600, 0, 400)

var _no_bv3 := false


func _init() -> void:
	var out := ""
	var shot := "grass"
	var blood_level := 2
	var cam := PackedFloat64Array()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--shot="):
			shot = arg.trim_prefix("--shot=")
		elif arg.begins_with("--blood="):
			blood_level = int(arg.trim_prefix("--blood="))
		elif arg.begins_with("--cam="):
			cam = arg.trim_prefix("--cam=").split_floats(",")
		elif arg.begins_with("--center="):
			var c := arg.trim_prefix("--center=").split_floats(",")
			_center = Vector3(c[0], 0, c[1])
		elif arg == "--no-bv3":
			_no_bv3 = true
	if out == "":
		push_error("bv3_shot: --out=<png> required")
		quit(1)
		return
	# Les autoloads (Settings…) ne sont enregistrés qu'après _init.
	await process_frame
	var world := Node3D.new()
	root.add_child(world)
	_environment(world)
	var terrain := BattleTerrain.new()
	world.add_child(terrain)
	var nx := 121
	var nz := 81
	var heights := PackedFloat32Array()
	heights.resize(nx * nz)
	terrain.build({"nx": nx, "nz": nz, "resolution": 10.0, "heights": heights, "terrain": "plains", "season": "summer", "ground": "dry", "woodland": 0.0}, "clear")
	var camera := Camera3D.new()
	camera.fov = 45.0
	camera.far = 3000.0
	world.add_child(camera)
	var eye := _center + Vector3(-9, 5.5, -13)
	var target := _center + Vector3(0, 0.2, 2)
	if cam.size() >= 6:
		eye = _center + Vector3(cam[0], cam[1], cam[2])
		target = _center + Vector3(cam[3], cam[4], cam[5])
	camera.look_at_from_position(eye, target)
	var height := func(x: float, z: float) -> float: return terrain.world_height(x, z)
	var dry := func(_x: float, _z: float) -> int: return 0
	var blood := BattleBlood.new()
	world.add_child(blood)
	blood.setup(height, blood_level, dry)
	blood.corpse_driven = true
	var flatten: BattleGrassFlatten = null
	if not _no_bv3:
		flatten = BattleGrassFlatten.new()
		flatten.setup()
		terrain.vegetation.set_flatten(flatten)
	var soldiers := BattleSoldiers.new()
	world.add_child(soldiers)
	soldiers.setup([], {"attacker": Color(0.16, 0.25, 0.62), "defender": Color(0.72, 0.12, 0.12)}, {"attacker": "fac_france", "defender": "fac_england"})
	var blood_amount: float = [0.0, 0.6, 1.0][clampi(blood_level, 0, 2)]
	soldiers.corpse_fallen.connect(func(pos: Vector3, side: String, kind: String, cause: String) -> void:
		blood.on_corpse(pos, side, kind, cause, camera.position)
		if flatten != null:
			flatten.on_corpse(pos, kind, blood_amount))
	match shot:
		"standards":
			await _standards_shot(world, terrain, soldiers, camera, out)
			return
		"duel":
			await _duel_shot(world, terrain, soldiers, camera, out)
			return
		"atlas", "impostor":
			await _impostor_shot(world, soldiers, camera, out, shot)
			return
		"grass":
			# Deux régiments au contact : l'herbe foulée par la mêlée (front + emprise).
			var a := _unit(1, "attacker", Vector3(_center.x, 0, _center.z - 6), 0.0)
			var d := _unit(2, "defender", Vector3(_center.x, 0, _center.z + 6), PI)
			if flatten != null:
				for step in 60:
					flatten.update([a, d], 0.5)
			# Morts sur la ligne de contact.
			var rng := RandomNumberGenerator.new()
			rng.seed = 77
			for side in ["attacker", "defender"]:
				var prev := PackedFloat32Array()
				for k in 14:
					var p := _center + Vector3(rng.randf_range(-14, 14), 0, rng.randf_range(-3, 3) + (-1.5 if side == "attacker" else 1.5))
					p.y = terrain.world_height(p.x, p.z)
					var ang := rng.randf() * TAU
					prev.append_array(PackedFloat32Array([cos(ang), 0, sin(ang), p.x, 0, 1, 0, p.y, -sin(ang), 0, cos(ang), p.z]))
				var unit := {"id": 1 if side == "attacker" else 2, "side": side, "type": "unit_men_at_arms_foot", "loss_cause": "melee", "loss_by": -1}
				soldiers.call("_spawn_corpses", unit, side, "infantry", 0, prev, 14)
			if flatten != null:
				flatten.flush()
	soldiers.set("_camera_pos", camera.position)
	soldiers.call("_update_corpse_lods")
	# Les morts jouent leur clip de chute puis se figent ; les flaques s'étalent (20 s).
	soldiers.anim_time += 30.0
	blood.tick_time(30.0)
	for i in 20:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(out.get_base_dir())
	var err := image.save_png(out)
	print("bv3_shot: %s (%s), %d corpses, %d blood decals, flatten %s" % [out, error_string(err), soldiers.corpse_count, blood.decal_count, "off" if flatten == null else "%.2f / blood %.2f at center" % [flatten.flatten_at(_center.x, _center.z), flatten.blood_at(_center.x, _center.z)]])
	quit(0 if err == OK else 1)


func _unit(id: int, side: String, pos: Vector3, facing: float) -> Dictionary:
	return {"id": id, "side": side, "present": true, "state": "melee", "x": pos.x, "z": pos.z, "facing": facing, "width": 30.0, "depth": 6.0, "soldiers": 100}


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
	sun.rotation = Vector3(deg_to_rad(-48), deg_to_rad(-35), 0)
	sun.light_color = Color(1, 0.94, 0.84)
	sun.light_energy = 1.6
	sun.shadow_enabled = true
	world.add_child(sun)


## `atlas` : l'atlas d'imposteurs cuit (image brute) ; `impostor` : 8 figurines (caps 0-315°)
## en maillage (rang du fond) et en imposteurs (rang de devant), vues de près pour comparer.
## `--fig=archer_0` choisit la figurine.
func _impostor_shot(world: Node3D, soldiers: BattleSoldiers, camera: Camera3D, out: String, shot: String) -> void:
	var fig := "infantry_0"
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--fig="):
			fig = arg.trim_prefix("--fig=")
	var parts := fig.rsplit("_", true, 1)
	var kind := parts[0]
	var variant := int(parts[1])
	var impostors := BattleImpostors.new()
	world.add_child(impostors)
	var mat: ShaderMaterial = soldiers.call("_make_skinned_material", "attacker", kind, variant, false)
	var key := BattleImpostors.key_of("attacker", kind, variant)
	impostors.request(key, kind, variant, mat)
	for i in 30:
		await process_frame
		if impostors.is_ready(key):
			break
	if not impostors.is_ready(key):
		push_error("bv3_shot: atlas not baked")
		quit(1)
		return
	if shot == "atlas":
		var err := impostors.atlas_image(key).save_png(out)
		print("bv3_shot: atlas %s (%s)" % [out, error_string(err)])
		quit(0 if err == OK else 1)
		return
	var spacing := 3.5 if kind == "cavalry" else 1.6
	var ground := _center.y
	var real := MultiMesh.new()
	real.transform_format = MultiMesh.TRANSFORM_3D
	real.mesh = BattleSkinned.mesh(kind, variant, 0)
	real.instance_count = 8
	var quads := MultiMesh.new()
	quads.transform_format = MultiMesh.TRANSFORM_3D
	quads.mesh = BattleImpostors.quad_mesh()
	quads.instance_count = 8
	for k in 8:
		var basis := Basis(Vector3.UP, float(k) * TAU / 8.0)
		var x := _center.x + (float(k) - 3.5) * spacing
		real.set_instance_transform(k, Transform3D(basis, Vector3(x, ground, _center.z - spacing * 1.2)))
		quads.set_instance_transform(k, Transform3D(basis, Vector3(x, ground, _center.z)))
	var real_inst := MultiMeshInstance3D.new()
	real_inst.multimesh = real
	real_inst.material_override = mat
	world.add_child(real_inst)
	var imp_inst := MultiMeshInstance3D.new()
	imp_inst.multimesh = quads
	imp_inst.material_override = impostors.make_material(key)
	world.add_child(imp_inst)
	camera.look_at_from_position(Vector3(_center.x, ground + spacing * 2.2, _center.z + spacing * 5.5), Vector3(_center.x, ground + 0.9, _center.z - spacing * 0.6))
	for i in 10:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var err := image.save_png(out)
	print("bv3_shot: impostor %s (%s)" % [out, error_string(err)])
	quit(0 if err == OK else 1)


## Régiment de décor au format de `get_units()`.
func _stage_unit(id: int, side: String, type: String, render: String, pos: Vector3, facing: float, soldiers: int, general: bool) -> Dictionary:
	var width := 24.0 if render != "cavalry" else 30.0
	return {"id": id, "side": side, "type": type, "render": render, "soldiers": soldiers, "figures": soldiers, "initial_soldiers": soldiers, "present": true, "x": pos.x, "y": pos.y, "z": pos.z, "facing": facing, "width": width, "depth": ceil(float(soldiers) / width) * (2.8 if render == "cavalry" else 1.1), "state": "idle", "running": false, "ammo": 0, "is_general": general}


## Décor : hommes d'armes et chevaliers du général (France) face à des hommes d'armes anglais.
func _stage(terrain: BattleTerrain, soldiers: BattleSoldiers) -> FakeBattle:
	var fake := FakeBattle.new()
	var c := _center
	fake.units = [
		_stage_unit(1, "attacker", "unit_men_at_arms_foot", "infantry", Vector3(c.x - 16, 0, c.z), 0.0, 72, false),
		_stage_unit(2, "attacker", "unit_knights", "cavalry", Vector3(c.x + 20, 0, c.z - 4), 0.0, 30, true),
		_stage_unit(3, "defender", "unit_men_at_arms_foot", "infantry", Vector3(c.x, 0, c.z + 40), PI, 72, false),
	]
	for unit in fake.units:
		unit["y"] = terrain.world_height(float(unit["x"]), float(unit["z"]))
	soldiers.setup(fake.units, _colors(), _factions())
	return fake


func _colors() -> Dictionary:
	return {"attacker": Color(0.16, 0.25, 0.62), "defender": Color(0.72, 0.12, 0.12)}


func _factions() -> Dictionary:
	return {"attacker": "fac_france", "defender": "fac_england"}


## `standards` : étendards portés dans le vent (et herbe dans le même vent).
func _standards_shot(world: Node3D, terrain: BattleTerrain, soldiers: BattleSoldiers, camera: Camera3D, out: String) -> void:
	var fake := _stage(terrain, soldiers)
	var c := _center
	var standards := BattleStandards.new()
	world.add_child(standards)
	var wind := BattleStandards.wind_for("clear", 7)
	var factions := _factions()
	standards.setup(fake.units, _colors(), func(unit: Dictionary) -> Dictionary: return _cloth(unit, str(factions[str(unit["side"])])), wind)
	terrain.vegetation.set_wind(wind["dir"], float(wind["strength"]) * float(wind["grass_scale"]))
	camera.look_at_from_position(Vector3(c.x - 6, c.y + 7, c.z - 26), Vector3(c.x + 2, c.y + 2.5, c.z + 4))
	for i in 30:
		soldiers.update(fake, fake.units, 1.0 / 30.0, [])
		standards.update(fake.units, soldiers, camera.position)
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var err := image.save_png(out)
	print("bv3_shot: standards %s (%s), %d shown, wind %s strength %.2f" % [out, error_string(err), standards.shown_count, wind["dir"], float(wind["strength"])])
	quit(0 if err == OK else 1)


## Étoffe (même choix que `BattleScene._banner_cloth`, sans le « pas de quartier »).
func _cloth(unit: Dictionary, faction: String) -> Dictionary:
	var dir := "res://assets/heraldry/banners/"
	var candidates: Array = []
	if bool(unit.get("is_general", false)) and faction == "fac_england":
		candidates.append([dir + "st_george.png", Vector2(1.3, 2.6)])
	if str(unit.get("type", "")) in ["unit_knights", "unit_men_at_arms_foot"]:
		candidates.append([dir + "%s_banner.png" % faction, Vector2(1.3, 2.6)])
	candidates.append([dir + "%s_pennon.png" % faction, Vector2(3.0, 0.75)])
	for candidate in candidates:
		var texture := PortraitLoader.load_texture(candidate[0])
		if texture != null:
			return {"texture": texture, "size": candidate[1], "full": true}
	return {"texture": PortraitLoader.heraldry_texture(faction), "size": Vector2(2.6, 1.7), "full": false}


## `duel` : deux régiments d'hommes d'armes au contact ; un duel s'engage entre deux figurines ;
## captures `<out>_0..3.png` à 0,4 / 1,3 / 2,2 / 3,1 s du début (une par passe).
func _duel_shot(world: Node3D, terrain: BattleTerrain, soldiers: BattleSoldiers, camera: Camera3D, out: String) -> void:
	var fake := FakeBattle.new()
	var c := _center
	fake.units = [
		_stage_unit(1, "attacker", "unit_men_at_arms_foot", "infantry", Vector3(c.x, 0, c.z - 3.4), 0.0, 48, true),
		_stage_unit(3, "defender", "unit_men_at_arms_foot", "infantry", Vector3(c.x, 0, c.z + 3.4), PI, 48, false),
	]
	for unit in fake.units:
		unit["y"] = terrain.world_height(float(unit["x"]), float(unit["z"]))
		unit["state"] = "melee"
		unit["depth"] = 4.4
	fake.units[0]["target"] = 3
	fake.units[1]["target"] = 1
	soldiers.setup(fake.units, _colors(), _factions())
	var duels := BattleDuels.new()
	world.add_child(duels)
	duels.setup()
	var dt := 1.0 / 30.0
	var started_at := -1.0
	var shots := [0.4, 1.3, 2.2, 3.1]
	var shot_index := 0
	for i in 400:
		soldiers.update(fake, fake.units, dt, [])
		duels.update(fake.units, soldiers, soldiers.anim_time, camera.position)
		if started_at < 0.0 and duels.started_count > 0:
			started_at = soldiers.anim_time
			var m := duels.last_center
			camera.look_at_from_position(m + Vector3(5.5, 2.4, -1.5), m + Vector3(0, 1.0, 0))
		await process_frame
		if started_at >= 0.0 and soldiers.anim_time - started_at >= float(shots[shot_index]):
			await RenderingServer.frame_post_draw
			var path := "%s_%d.png" % [out.trim_suffix(".png"), shot_index]
			root.get_texture().get_image().save_png(path)
			print("bv3_shot: duel %s at %.1f s" % [path, soldiers.anim_time - started_at])
			shot_index += 1
			if shot_index >= shots.size():
				break
	print("bv3_shot: %d duels started" % duels.started_count)
	quit(0 if duels.started_count > 0 else 1)
