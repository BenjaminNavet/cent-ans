extends SceneTree

## Lot OMR R5 : unités propres à l'Est en bataille (vraie simulation, vrai rendu des soldats).
## Deux lignes de cinq régiments orientaux (`data/unit_types/`), IA coupée, figées au repos ;
## vérifie que chaque régiment prend sa variante de rendu (`data/fx/unit_looks.json`).
## Usage (avec affichage, pour un cliché) :
##   godot --path game --resolution 1280x720 --script res://tests/omr_r5_units_shot.gd -- \
##     --out=<fichier.png> [--side=attacker|defender]
## Sans affichage (`--headless`) : vérifications seules (code de retour 0).

const ATTACKERS := ["unit_mamluk_cavalry", "unit_steppe_horse_archers", "unit_druzhina", "unit_teutonic_knights", "unit_lithuanian_light_cavalry"]
const DEFENDERS := ["unit_akinci", "unit_yaya", "unit_serbian_heavy_cavalry", "unit_pronoiars", "unit_almogavars"]

var _out := ""
var _side := "attacker"


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--side="):
			_side = arg.trim_prefix("--side=")
	await process_frame
	var code := await _run()
	quit(code)


func _run() -> int:
	var sim: Object = ClassDB.instantiate("CampaignSim")
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		push_error("omr_r5_units_shot: campaign")
		return 1
	var armies := BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var setup: Dictionary = sim.call("get_battle_setup", index)
	setup["attacker"]["units"] = ATTACKERS.map(func(t: String) -> Dictionary: return _from_data(data_dir, t))
	setup["defender"]["units"] = DEFENDERS.map(func(t: String) -> Dictionary: return _from_data(data_dir, t))
	setup["river"] = false
	setup["village"] = false
	var battle: Object = ClassDB.instantiate("BattleSim")
	if not battle.call("setup", setup, 11):
		push_error("omr_r5_units_shot: battle setup")
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
	var units: Array = battle.call("get_units")
	var soldiers := BattleSoldiers.new()
	world.add_child(soldiers)
	soldiers.setup(units, {"attacker": Color(0.85, 0.75, 0.2), "defender": Color(0.62, 0.1, 0.1)}, {"attacker": "fac_mamluks", "defender": "fac_ottoman"})
	var failures := 0
	for unit in units:
		var type := str(unit.get("type", ""))
		var mat := soldiers.get_node_or_null("Unit%d_%s" % [int(unit["id"]), str(unit["render"])]) as MultiMeshInstance3D
		if mat == null:
			push_error("omr_r5_units_shot: no layer for %s" % type)
			failures += 1
			continue
		var look := BattleUnitLooks.look(type)
		var material := mat.material_override as ShaderMaterial
		var count := int(material.get_shader_parameter("plain_count"))
		if look.has("cloth") and count != (look["cloth"] as Array).size():
			push_error("omr_r5_units_shot: %s cloth %d" % [type, count])
			failures += 1
		print("OMR_R5 %s render=%s variant=%d plain=%d coats=%s" % [type, unit["render"], BattleMeshes.variant_of(type), count, material.get_shader_parameter("coat_count")])
	for _i in 10:
		battle.call("tick", 0.1)
		units = battle.call("get_units")
		soldiers.update(battle, units, 0.1, [])
	if not headless and _out != "":
		var camera := Camera3D.new()
		camera.fov = 40.0
		world.add_child(camera)
		var line := Vector3.ZERO
		var other := Vector3.ZERO
		var n := 0
		var m := 0
		for unit in units:
			var p := Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"]))
			if str(unit["side"]) == _side:
				line += p
				n += 1
			else:
				other += p
				m += 1
		line /= maxf(n, 1)
		other /= maxf(m, 1)
		var toward := (other - line).normalized()
		var eye := line + toward * 55.0 + Vector3.UP * 14.0 + toward.cross(Vector3.UP) * 12.0
		camera.look_at_from_position(eye, line + Vector3.UP * 1.5)
		for _f in 4:
			await process_frame
		RenderingServer.force_draw()
		await process_frame
		root.get_viewport().get_texture().get_image().save_png(_out)
		print("OMR_R5_SHOT %s" % _out)
	return 1 if failures > 0 else 0


## Régiment de `unit_type` construit depuis `data/unit_types/`.
func _from_data(data_dir: String, unit_type: String) -> Dictionary:
	var d: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(data_dir.path_join("unit_types/%s.json" % unit_type)))
	return {"unit_type": unit_type, "name": str(d["name"]["display"]), "category": d["category"], "mounted": bool(d.get("mounted", false)), "soldiers": int(d["soldiers"]), "max_soldiers": int(d["soldiers"]), "morale": 70, "experience": 2, "stats": d["stats"], "abilities": d.get("abilities", [])}


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
	sun.directional_shadow_max_distance = 160.0
	world.add_child(sun)
