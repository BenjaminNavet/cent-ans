extends SceneTree

## Lot EP11 : poussée continue des lignes en mêlée filmée (vraie simulation, vrai rendu des soldats).
## Des hommes d'armes à pied (France) attaquent une milice urbaine (Angleterre), IA coupée ; des
## clichés sont pris aux instants `--after=<s,s,...>` après le contact, caméra haute derrière
## l'attaquant pour voir la forme du front.
## Scènes (`--scene=`) :
##   line   : ligne lourde contre ligne légère — le front de l'attaquant se bombe, la milice recule ;
##   wrap   : la milice en colonne étroite — les files débordantes s'enroulent autour d'elle ;
##   backed : une seconde milice juste derrière la première — elle ne peut reculer, se comprime.
## Usage (avec affichage) :
##   godot --path game --resolution 1600x900 --script res://tests/ep11_push_shot.gd -- \
##     --out=<préfixe> [--scene=line|wrap|backed] [--after=1,10,25] [--cam=dx,dy,dz]
## Sans affichage (`--headless`) : vérifie seulement que le contact a lieu et, en scène `line`, que
## la milice cède du terrain (code de retour 0).

var _out := ""
var _scene := "line"
var _after: PackedFloat64Array = PackedFloat64Array([1.0, 10.0, 25.0])
var _cam := Vector3(-14.0, 22.0, -26.0)


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--scene="):
			_scene = arg.trim_prefix("--scene=")
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
		push_error("ep11_push_shot: campaign")
		return 1
	var armies := BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var setup: Dictionary = sim.call("get_battle_setup", index)
	var heavy := _from_data(data_dir, "unit_men_at_arms_foot")
	var light := _from_data(data_dir, "unit_urban_militia")
	if heavy.is_empty() or light.is_empty():
		push_error("ep11_push_shot: unit types missing")
		return 1
	setup["attacker"]["units"] = [heavy]
	setup["defender"]["units"] = [light, light.duplicate(true)] if _scene == "backed" else [light]
	setup["river"] = false
	setup["village"] = false
	# Laboratoire : les deux camps obéissent aux ordres du script.
	setup.erase("player_side")
	var battle: Object = ClassDB.instantiate("BattleSim")
	if not battle.call("setup", setup, 7):
		return 1
	battle.call("set_ai", "attacker", false)
	battle.call("set_ai", "defender", false)
	var world := Node3D.new()
	root.add_child(world)
	var headless := DisplayServer.get_name() == "headless"
	if not headless:
		_environment(world)
		var terrain := BattleTerrain.new()
		world.add_child(terrain)
		terrain.build(battle.call("get_terrain"), "clear")
	var camera := Camera3D.new()
	camera.fov = 45.0
	world.add_child(camera)
	var units: Array = battle.call("get_units")
	var soldiers := BattleSoldiers.new()
	world.add_child(soldiers)
	soldiers.setup(units, {"attacker": Color(0.16, 0.25, 0.62), "defender": Color(0.7, 0.12, 0.12)}, {"attacker": "fac_france", "defender": "fac_england"})
	var attacker := -1
	var defenders: Array = []
	for unit in units:
		if str(unit["side"]) == "attacker":
			attacker = int(unit["id"])
		else:
			defenders.append(int(unit["id"]))
	var front: Dictionary = _unit(units, defenders[0])
	if _scene == "wrap":
		var result: Dictionary = battle.call("issue_command", {"type": "formation", "units": [defenders[0]], "kind": "column"})
		if not bool(result.get("ok", false)):
			push_error("ep11_push_shot: column refused: %s" % str(result.get("error", "")))
			return 1
	elif _scene == "backed":
		# La seconde milice se range juste derrière la première (côté défenseur : z croissant).
		var depth := float(front["depth"])
		battle.call("issue_command", {"type": "move", "units": [defenders[1]], "x": float(front["x"]), "z": float(front["z"]) + depth + 1.2, "facing": float(front["facing"])})
		for _i in 900:
			battle.call("tick", 0.1)
		units = battle.call("get_units")
		var back: Dictionary = _unit(units, defenders[1])
		print("EP11_SHOT backed: rear militia at (%.1f, %.1f)" % [float(back["x"]), float(back["z"])])
	battle.call("issue_command", {"type": "attack", "units": [attacker], "target": defenders[0], "run": false})
	var contact_time := -1.0
	var start_z := 0.0
	var shots := 0
	for _i in 6000:
		battle.call("tick", 0.1)
		units = battle.call("get_units")
		soldiers.update(battle, units, 0.1, [])
		var elapsed := float(battle.call("get_elapsed"))
		var a: Dictionary = _unit(units, attacker)
		var d: Dictionary = _unit(units, defenders[0])
		if contact_time < 0.0 and str(a["state"]) == "melee":
			contact_time = elapsed
			start_z = float(d["z"])
			print("EP11_SHOT contact at %.1f s" % elapsed)
		if contact_time < 0.0:
			continue
		# Caméra haute derrière l'attaquant, tournée vers le point de contact.
		var fwd := Vector3(sin(float(a["facing"])), 0, cos(float(a["facing"])))
		var right := fwd.cross(Vector3.UP)
		var focus := Vector3(float(a["x"]), float(a.get("y", 0.0)), float(a["z"])) + fwd * (float(a["depth"]) * 0.5 + 1.0)
		var eye := focus + right * _cam.x + Vector3.UP * _cam.y + fwd * _cam.z
		camera.look_at_from_position(eye, focus)
		if shots < _after.size() and elapsed >= contact_time + _after[shots]:
			print("EP11_SHOT t+%.0f s: militia gave %.1f m" % [_after[shots], float(d["z"]) - start_z])
			print("DEBUG a %s | d %s" % [str(a), str(d)])
			if not headless and _out != "":
				for _f in 3:
					await process_frame
				RenderingServer.force_draw()
				await process_frame
				var path := "%s_%s_%d.png" % [_out, _scene, shots]
				root.get_viewport().get_texture().get_image().save_png(path)
				print("EP11_SHOT %s" % path)
			shots += 1
		if shots >= _after.size():
			break
	if contact_time < 0.0:
		push_error("ep11_push_shot: no contact")
		return 1
	var last: Dictionary = _unit(units, defenders[0])
	var gave := float(last["z"]) - start_z
	if _scene == "line" and gave < 1.0:
		push_error("ep11_push_shot: the militia did not give ground (%.2f m)" % gave)
		return 1
	return 0


func _unit(units: Array, id: int) -> Dictionary:
	for unit in units:
		if int(unit["id"]) == id:
			return unit
	return {}


## Régiment de `unit_type` construit depuis `data/unit_types/`.
func _from_data(data_dir: String, unit_type: String) -> Dictionary:
	var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(data_dir.path_join("unit_types/%s.json" % unit_type)))
	if not parsed is Dictionary:
		return {}
	var d: Dictionary = parsed
	var stats := {}
	for key in d["stats"]:
		stats[key] = int(d["stats"][key])
	d["stats"] = stats
	return {"unit_type": unit_type, "name": str(d["name"]["display"]), "category": d["category"], "mounted": bool(d.get("mounted", false)), "soldiers": int(d["soldiers"]), "max_soldiers": int(d["soldiers"]), "morale": 90, "experience": 2, "stats": d["stats"], "abilities": d.get("abilities", [])}


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
