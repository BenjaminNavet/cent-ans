extends SceneTree

## Lot NT7 : captures de contrôle (écrites, jamais lues par l'agent ; la session principale juge
## le rendu). Hors simulation, sur une prairie plate, comme `bv3_shot.gd`.
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1280x720 --script res://tests/nt7_anim_shot.gd -- \
##     [--out-dir=<dossier>] [--no-nt7]
## Écrit dans `docs/audit/captures/nt/` (défaut) :
## - `nt7_melee_0..3.png` : mêlée rapprochée, quatre images à 0,07 s d'écart (fondus de cycle) ;
## - `nt7_charge.png` : porte-étendard et musiciens à la charge (`std_charge`, `drum_run`,
##   `horn_run`) ;
## - `nt7_victory.png` : camp vainqueur (`std_victory`, `drum_victory`, `horn_victory`) ;
## - `nt7_crew.png` : servants (`load_heavy`, `push`/`push_shoulder`, `crank`).

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
var _out_dir := "../docs/audit/captures/nt"


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out-dir="):
			_out_dir = arg.trim_prefix("--out-dir=")
	_out_dir = ProjectSettings.globalize_path("res://").path_join(_out_dir).simplify_path() if _out_dir.is_relative_path() else _out_dir
	DirAccess.make_dir_recursive_absolute(_out_dir)
	await process_frame
	var failures := 0
	for shot in ["melee", "charge", "victory", "crew"]:
		failures += await _shot(shot)
	print("nt7_anim_shot: %s (%s)" % ["OK" if failures == 0 else "FAIL", _out_dir])
	quit(0 if failures == 0 else 1)


func _shot(shot: String) -> int:
	var world := Node3D.new()
	root.add_child(world)
	_environment(world)
	var terrain := BattleTerrain.new()
	world.add_child(terrain)
	var heights := PackedFloat32Array()
	heights.resize(121 * 81)
	terrain.build({"nx": 121, "nz": 81, "resolution": 10.0, "heights": heights, "terrain": "plains", "season": "summer", "ground": "dry", "woodland": 0.0}, "clear")
	var camera := Camera3D.new()
	camera.fov = 45.0
	camera.far = 3000.0
	world.add_child(camera)
	var soldiers := BattleSoldiers.new()
	world.add_child(soldiers)
	var failures := 0
	match shot:
		"melee":
			failures = await _melee(world, terrain, soldiers, camera)
		"charge", "victory":
			failures = await _standards(world, terrain, soldiers, camera, shot)
		"crew":
			failures = await _crew(world, soldiers, camera)
	world.queue_free()
	await process_frame
	return failures


func _save(name: String) -> int:
	await RenderingServer.frame_post_draw
	var path := _out_dir.path_join(name)
	var image := root.get_texture().get_image()
	if image.get_width() > 1280:
		# Écran HiDPI : ramené à 1280 px de large (poids des fichiers versionnés).
		image.resize(1280, int(image.get_height() * 1280.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var err := image.save_png(path)
	print("nt7_anim_shot: %s (%s)" % [path, error_string(err)])
	return 0 if err == OK else 1


func _stage_unit(id: int, side: String, type: String, render: String, pos: Vector3, facing: float, count: int, general: bool) -> Dictionary:
	var width := 24.0 if render != "cavalry" else 30.0
	return {"id": id, "side": side, "type": type, "render": render, "soldiers": count, "figures": count, "initial_soldiers": count, "present": true, "x": pos.x, "y": pos.y, "z": pos.z, "facing": facing, "width": width, "depth": ceil(float(count) / width) * (2.8 if render == "cavalry" else 1.1), "state": "idle", "running": false, "ammo": 0, "is_general": general}


func _colors() -> Dictionary:
	return {"attacker": Color(0.16, 0.25, 0.62), "defender": Color(0.72, 0.12, 0.12)}


func _factions() -> Dictionary:
	return {"attacker": "fac_france", "defender": "fac_england"}


## Mêlée rapprochée : quatre images à 0,07 s d'écart (un fondu de 0,2 s en couvre trois).
func _melee(_world: Node3D, terrain: BattleTerrain, soldiers: BattleSoldiers, camera: Camera3D) -> int:
	var fake := FakeBattle.new()
	var c := _center
	fake.units = [
		_stage_unit(1, "attacker", "unit_men_at_arms_foot", "infantry", Vector3(c.x, 0, c.z - 3.2), 0.0, 48, false),
		_stage_unit(2, "defender", "unit_men_at_arms_foot", "infantry", Vector3(c.x, 0, c.z + 3.2), PI, 48, false),
	]
	for unit in fake.units:
		unit["y"] = terrain.world_height(float(unit["x"]), float(unit["z"]))
		unit["state"] = "melee"
		unit["depth"] = 4.4
	fake.units[0]["target"] = 2
	fake.units[1]["target"] = 1
	soldiers.setup(fake.units, _colors(), _factions())
	camera.look_at_from_position(c + Vector3(6.5, 2.2, -1.0), c + Vector3(0, 1.0, 0))
	var dt := 1.0 / 30.0
	for i in 45:
		soldiers.update(fake, fake.units, dt, [])
		await process_frame
	var failures := 0
	for k in 4:
		failures += await _save("nt7_melee_%d.png" % k)
		for j in 2:
			soldiers.update(fake, fake.units, 0.035, [])
			await process_frame
	return failures


## Porte-étendards et musiciens : charge ou victoire du camp attaquant.
func _standards(world: Node3D, terrain: BattleTerrain, soldiers: BattleSoldiers, camera: Camera3D, shot: String) -> int:
	var fake := FakeBattle.new()
	var c := _center
	fake.units = [
		_stage_unit(1, "attacker", "unit_men_at_arms_foot", "infantry", Vector3(c.x - 8, 0, c.z), 0.0, 72, true),
		_stage_unit(2, "attacker", "unit_knights", "cavalry", Vector3(c.x + 24, 0, c.z - 4), 0.0, 30, false),
	]
	for unit in fake.units:
		unit["y"] = terrain.world_height(float(unit["x"]), float(unit["z"]))
		if shot == "charge":
			unit["state"] = "charging"
	soldiers.setup(fake.units, _colors(), _factions())
	if shot == "victory":
		soldiers.victor_side = "attacker"
	var standards := BattleStandards.new()
	world.add_child(standards)
	var wind := BattleStandards.wind_for("clear", 7)
	var faction := "fac_france"
	standards.setup(fake.units, _colors(), func(_unit: Dictionary) -> Dictionary: return {"texture": PortraitLoader.heraldry_texture(faction), "size": Vector2(2.6, 1.7), "full": false}, wind)
	camera.look_at_from_position(Vector3(c.x - 4, c.y + 4.5, c.z - 22), Vector3(c.x + 2, c.y + 2.0, c.z + 2))
	for i in 40:
		soldiers.update(fake, fake.units, 1.0 / 30.0, [])
		standards.update(fake.units, soldiers, camera.position)
		await process_frame
	return await _save("nt7_%s.png" % shot)


## Servants : chargeurs de trébuchet (pierre lourde), pousseurs de bélier alternés, treuil.
func _crew(world: Node3D, soldiers: BattleSoldiers, camera: Camera3D) -> int:
	if not SiegeCrewFx.enabled():
		print("nt7_anim_shot: crew figure absent, skipped")
		return 0
	soldiers.setup([], _colors(), _factions())
	var crew := SiegeCrewFx.new()
	world.add_child(crew)
	crew.setup(soldiers)
	var c := _center
	var gestures := [["loader", "trebuchet"], ["loader", "trebuchet"], ["pusher", "ram"], ["pusher", "ram"], ["pusher", "ram"], ["winch", "trebuchet"]]
	camera.look_at_from_position(c + Vector3(0, 2.0, -7.5), c + Vector3(0, 0.9, 0))
	for i in 40:
		crew.begin(float(i) / 30.0)
		for k in gestures.size():
			var g: Array = gestures[k]
			var xform := Transform3D(Basis(Vector3.UP, PI), c + Vector3((float(k) - 2.5) * 1.3, 0, 0))
			crew.add("s%d" % k, xform, crew.clip_of(str(g[0]), str(g[1]), k), 0, "attacker")
		crew.finish(camera.position)
		await process_frame
	return await _save("nt7_crew.png")


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
	sun.rotation = Vector3(deg_to_rad(-48), deg_to_rad(-35), 0)
	sun.light_energy = 1.6
	sun.shadow_enabled = true
	world.add_child(sun)
