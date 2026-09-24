extends SceneTree

## Lot BV2 : charge de cavalerie filmée au choc (vraie simulation, vrai rendu des soldats).
## Les chevaliers de l'armée française chargent un régiment de `--target=<type>` (IA coupée),
## et des clichés sont pris aux instants `--after=<s,s,...>` après le premier impact.
## Usage (avec affichage) :
##   godot --path game --resolution 1600x900 --script res://tests/bv2_charge_shot.gd -- \
##     --out=<préfixe> [--target=unit_urban_militia] [--after=0.3,0.9,1.8,3.5] [--blood=full]
##     [--cam=dx,dy,dz] (caméra relative au point d'impact, repère de la charge) [--stakes]
## Sans affichage (`--headless`) : vérifie seulement qu'un impact arrive et qu'il renverse des
## soldats (code de retour 0).

var _out := ""
var _target := "unit_urban_militia"
var _after: PackedFloat64Array = PackedFloat64Array([0.4, 1.0, 2.0, 3.5])
var _cam := Vector3(-9.0, 5.0, -14.0)


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--target="):
			_target = arg.trim_prefix("--target=")
		elif arg.begins_with("--after="):
			_after = arg.trim_prefix("--after=").split_floats(",")
		elif arg.begins_with("--cam="):
			var c := arg.trim_prefix("--cam=").split_floats(",")
			_cam = Vector3(c[0], c[1], c[2])
	await process_frame
	var code := await _run()
	quit(code)


func _run() -> int:
	var sim: Object = ClassDB.instantiate("CampaignSim")
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		push_error("bv2_charge_shot: campaign")
		return 1
	var armies := BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var setup: Dictionary = sim.call("get_battle_setup", index)
	var pool: Array = (setup["attacker"]["units"] as Array) + (setup["defender"]["units"] as Array)
	var knights: Dictionary = {}
	var target: Dictionary = {}
	for unit in pool:
		var type := str(unit.get("unit_type", ""))
		if knights.is_empty() and type == "unit_knights":
			knights = unit.duplicate(true)
		if target.is_empty() and type == _target:
			target = unit.duplicate(true)
	if target.is_empty():
		target = _from_data(data_dir, _target)
	if knights.is_empty() or target.is_empty():
		push_error("bv2_charge_shot: knights or %s missing" % _target)
		return 1
	# Sans pieux (sauf `--stakes`) : la charge doit porter.
	if not OS.get_cmdline_user_args().has("--stakes"):
		var kept: Array = []
		for ability in target.get("abilities", []):
			if str(ability) != "stakes":
				kept.append(ability)
		target["abilities"] = kept
	setup["attacker"]["units"] = [knights]
	setup["defender"]["units"] = [target]
	setup["river"] = false
	setup["village"] = false
	var battle: Object = ClassDB.instantiate("BattleSim")
	if not battle.call("setup", setup, 7):
		return 1
	battle.call("set_ai", "attacker", false)
	battle.call("set_ai", "defender", false)
	var world := Node3D.new()
	root.add_child(world)
	var headless := DisplayServer.get_name() == "headless"
	var terrain: BattleTerrain = null
	if not headless:
		_environment(world)
		terrain = BattleTerrain.new()
		world.add_child(terrain)
		terrain.build(battle.call("get_terrain"), "clear")
	var camera := Camera3D.new()
	camera.fov = 45.0
	world.add_child(camera)
	var units: Array = battle.call("get_units")
	var soldiers := BattleSoldiers.new()
	world.add_child(soldiers)
	soldiers.setup(units, {"attacker": Color(0.16, 0.25, 0.62), "defender": Color(0.7, 0.12, 0.12)}, {"attacker": "fac_france", "defender": "fac_england"})
	var ids := {}
	for unit in units:
		ids[str(unit["side"])] = int(unit["id"])
	battle.call("issue_command", {"type": "attack", "units": [ids["attacker"]], "target": ids["defender"], "run": true})
	# Avance jusqu'au premier impact (le rendu consomme les impacts : on suit les renversés).
	var impact_time := -1.0
	var shots := 0
	var start_knocked := soldiers.knocked_count
	for _i in 1200:
		battle.call("tick", 0.1)
		units = battle.call("get_units")
		soldiers.update(battle, units, 0.1, [])
		var elapsed := float(battle.call("get_elapsed"))
		if impact_time < 0.0 and _charge_hit(units):
			impact_time = elapsed
			var focus := Vector3.ZERO
			var heading := 0.0
			for unit in units:
				if str(unit["side"]) == "defender":
					focus = Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"]))
				else:
					heading = atan2(-float(unit["x"]), -float(unit["z"]))
			var fwd := Vector3(sin(float(units[0].get("facing", 0.0))), 0, cos(float(units[0].get("facing", 0.0))))
			var right := fwd.cross(Vector3.UP)
			var eye := focus + right * _cam.x + Vector3.UP * _cam.y + fwd * _cam.z
			camera.look_at_from_position(eye, focus + Vector3.UP * 1.0)
			print("BV2_CHARGE impact at %.1f s, heading %.2f" % [elapsed, heading])
		if impact_time >= 0.0 and shots < _after.size() and elapsed >= impact_time + _after[shots]:
			if not headless and _out != "":
				for _f in 3:
					await process_frame
				RenderingServer.force_draw()
				await process_frame
				var path := "%s_%d.png" % [_out, shots]
				root.get_viewport().get_texture().get_image().save_png(path)
				print("BV2_SHOT %s" % path)
			shots += 1
		if shots >= _after.size():
			break
	print("BV2_CHARGE knocked %d, corpses %d, severed %d, drops %d" % [soldiers.knocked_count - start_knocked, soldiers.corpse_count, soldiers.severed_count, soldiers.gore.drops_emitted])
	# Piques et pieux arrêtent la charge : aucun renversé attendu.
	var expect_knock := _target != "unit_flemish_pikemen" and not OS.get_cmdline_user_args().has("--stakes")
	if impact_time < 0.0 or (expect_knock and soldiers.knocked_count - start_knocked <= 0):
		push_error("bv2_charge_shot: no knock-down")
		return 1
	return 0


## Régiment de `unit_type` construit depuis `data/unit_types/` (absent des deux armées).
func _from_data(data_dir: String, unit_type: String) -> Dictionary:
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(data_dir.path_join("unit_types/%s.json" % unit_type)))
	if not parsed is Dictionary:
		return {}
	var d: Dictionary = parsed
	var stats := {}
	for key in d["stats"]:
		stats[key] = int(d["stats"][key])
	d["stats"] = stats
	return {"unit_type": unit_type, "name": str(d["name"]["display"]), "category": d["category"], "mounted": bool(d.get("mounted", false)), "soldiers": int(d["soldiers"]), "max_soldiers": int(d["soldiers"]), "morale": 70, "experience": 2, "stats": d["stats"], "abilities": d.get("abilities", [])}


## Le régiment des chevaliers vient d'entrer au contact (impact résolu par le cœur).
func _charge_hit(units: Array) -> bool:
	for unit in units:
		if str(unit["side"]) == "attacker" and str(unit["state"]) == "melee":
			return true
	return false


func _environment(world: Node3D) -> void:
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color(0.62, 0.7, 0.8)
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.55, 0.58, 0.62)
	env.ambient_light_energy = 0.8
	env.tonemap_mode = Environment.TONE_MAPPER_AGX
	var we := WorldEnvironment.new()
	we.environment = env
	world.add_child(we)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-48, 30, 0)
	sun.light_energy = 1.6
	sun.shadow_enabled = true
	sun.directional_shadow_max_distance = 120.0
	world.add_child(sun)
