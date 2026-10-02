extends SceneTree

## Lot FA3 : captures côte à côte des mêmes scènes, clips du jeu (gauche) et clips CC0 reciblés
## (droite, `--fa-anim`). Hors simulation, sur une prairie plate, comme `nt12_mocap_shot.gd`.
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1280x720 --script res://tests/fa3_anim_shot.gd -- [--out-dir=<dossier>]
## Écrit dans `docs/audit/captures/fa/` (défaut, ignoré par git) :
## - `fa3_<scène>_0..3.png` : 4 instants à 0,25 s d'écart, gauche jeu / droite FA3 (1280 px) ;
##   scènes `melee` (épée et bouclier), `pike` (piquiers contre fantassins), `bow` (archers qui
##   tirent : volée déclenchée, instants autour de la décoche), `victory` (acclamation).

const SKY_SHADER := preload("res://shaders/battle_sky.gdshader")
const STEPS := 4

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
var _out_dir := "../docs/audit/captures/fa"


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out-dir="):
			_out_dir = arg.trim_prefix("--out-dir=")
	_out_dir = ProjectSettings.globalize_path("res://").path_join(_out_dir).simplify_path() if _out_dir.is_relative_path() else _out_dir
	DirAccess.make_dir_recursive_absolute(_out_dir)
	await process_frame
	var failures := 0
	for scene in ["melee", "pike", "bow", "victory"]:
		var images := {}
		for layer in [0, 1]:
			BattleSkinned.fa_anim_forced = layer
			BattleSkinned.reload_caches()
			images[layer] = await _run(scene)
		var left: Array = images[0]
		var right: Array = images[1]
		for k in left.size():
			failures += _save_pair(left[k], right[k], "fa3_%s_%d.png" % [scene, k])
	BattleSkinned.fa_anim_forced = -1
	BattleSkinned.reload_caches()
	print("fa3_anim_shot: %s (%s)" % ["OK" if failures == 0 else "FAIL", _out_dir])
	quit(0 if failures == 0 else 1)


## Même mise en scène et mêmes pas de temps pour les deux passes : STEPS vues rapprochées.
func _run(scene: String) -> Array:
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
	var fake := FakeBattle.new()
	var c := _center
	var foe_type := "unit_men_at_arms_foot"
	var own_type := "unit_men_at_arms_foot"
	var own_render := "infantry"
	var state := "melee"
	match scene:
		"pike":
			own_type = "unit_flemish_pikemen"
		"bow":
			own_type = "unit_longbowmen"
			own_render = "archer"
			state = "shooting"
		"victory":
			state = "idle"
	fake.units = [_stage_unit(1, "attacker", own_type, own_render, Vector3(c.x, 0, c.z - 3.2), 0.0, 48, false)]
	if scene == "melee" or scene == "pike":
		fake.units.append(_stage_unit(2, "defender", foe_type, "infantry", Vector3(c.x, 0, c.z + 3.2), PI, 48, false))
		fake.units[0]["target"] = 2
		fake.units[1]["target"] = 1
	for unit in fake.units:
		unit["y"] = terrain.world_height(float(unit["x"]), float(unit["z"]))
		unit["state"] = state
		unit["depth"] = 4.4
		unit["ammo"] = 24
	soldiers.setup(fake.units, _colors(), _factions())
	if scene == "victory":
		soldiers.victor_side = "attacker"
	camera.look_at_from_position(c + Vector3(6.5, 2.2, -4.0 if scene != "melee" and scene != "pike" else -1.0), c + Vector3(0, 1.0, -3.2 if scene != "melee" and scene != "pike" else 0.0))
	var dt := 1.0 / 30.0
	for i in 45:
		soldiers.update(fake, fake.units, dt, [])
		await process_frame
	if scene == "bow":
		# Une volée part (instant du clip = 1,55 s), le clip reprend à 0 au rechargement (6 s) :
		# 5,4 s plus tard l'archer bande (0,95 s), les 4 vues encadrent la décoche suivante.
		fake.units[0]["ammo"] = 23
		for i in 54:
			soldiers.update(fake, fake.units, 0.1, [])
			await process_frame
	var out: Array = []
	for k in STEPS:
		out.append(await _grab())
		for j in 5:
			soldiers.update(fake, fake.units, 0.05, [])
			await process_frame
	world.queue_free()
	await process_frame
	return out


func _grab() -> Image:
	await RenderingServer.frame_post_draw
	return root.get_texture().get_image()


## Assemble gauche (jeu) | droite (FA3), ramené à 1280 px de large.
func _save_pair(left: Image, right: Image, name: String) -> int:
	var w := left.get_width()
	var h := left.get_height()
	var pair := Image.create(w * 2, h, false, left.get_format())
	pair.blit_rect(left, Rect2i(0, 0, w, h), Vector2i(0, 0))
	pair.blit_rect(right, Rect2i(0, 0, w, h), Vector2i(w, 0))
	pair.resize(1280, int(h * 1280.0 / (w * 2)), Image.INTERPOLATE_LANCZOS)
	var path := _out_dir.path_join(name)
	var err := pair.save_png(path)
	print("fa3_anim_shot: %s (%s)" % [path, error_string(err)])
	return 0 if err == OK else 1

func _stage_unit(id: int, side: String, type: String, render: String, pos: Vector3, facing: float, count: int, general: bool) -> Dictionary:
	var width := 24.0 if render != "cavalry" else 30.0
	return {"id": id, "side": side, "type": type, "render": render, "soldiers": count, "figures": count, "initial_soldiers": count, "present": true, "x": pos.x, "y": pos.y, "z": pos.z, "facing": facing, "width": width, "depth": ceil(float(count) / width) * (2.8 if render == "cavalry" else 1.1), "state": "idle", "running": false, "ammo": 0, "is_general": general}


func _colors() -> Dictionary:
	return {"attacker": Color(0.16, 0.25, 0.62), "defender": Color(0.72, 0.12, 0.12)}


func _factions() -> Dictionary:
	return {"attacker": "fac_france", "defender": "fac_england"}


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
