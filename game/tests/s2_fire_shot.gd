extends SceneTree

## Lot S2 : captures d'un siège en feu sur l'assaut de la Guyenne (`debug_stage_siege`). Quatre
## maisons voisines sont allumées (`BattleSim.debug_ignite`), la simulation avance pour laisser le
## feu se propager, puis trois vues : gros plan, vue large avec la muraille, ruines plus tard.
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1600x900 --script res://tests/s2_fire_shot.gd -- --out=<dossier>

var _out := ""


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
	if _out == "":
		push_error("s2_fire_shot: --out=<dossier> required")
		quit(1)
		return
	await process_frame
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		quit(1)
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.configure(sim, index, 7)
	root.add_child(scene)
	for _i in 30:
		await process_frame
	var battle: Object = scene.battle
	if battle.call("is_deploying"):
		battle.call("start_battle")
	battle.call("set_ai", "attacker", false)
	battle.call("set_ai", "defender", false)
	scene.paused = true
	var siege: Dictionary = battle.call("get_siege")
	var houses: Array = siege["houses"]
	var center: Vector2 = siege.get("center", Vector2(600, 560))
	# Le foyer : la maison intra-muros la plus proche du centre et ses trois voisines.
	var order := range(houses.size())
	order = order.filter(func(i: int) -> bool: return not bool(houses[i]["suburb"]))
	order.sort_custom(func(i: int, j: int) -> bool: return _pos(houses[i]).distance_to(center) < _pos(houses[j]).distance_to(center))
	var seed_pos := _pos(houses[order[0]])
	order.sort_custom(func(i: int, j: int) -> bool: return _pos(houses[i]).distance_to(seed_pos) < _pos(houses[j]).distance_to(seed_pos))
	for k in 4:
		battle.call("debug_ignite", order[k])
	for _i in 400:
		battle.call("tick", 0.1)
	scene.camera_rig.edge_pan_enabled = false
	var focus := Vector3(seed_pos.x, 0, seed_pos.y)
	scene.camera_rig.look_at_point(focus, 60.0, 0.6 + PI)
	await _wait(3.0)
	siege = battle.call("get_siege")
	print("s2_fire_shot: %d burning, status « %s »" % [int(siege.get("houses_burning", 0)), BattleScene.siege_status(siege)])
	await _shot("s2-feu-proche.png")
	scene.camera_rig.look_at_point(Vector3(center.x, 0, center.y), 170.0, 0.6)
	await _wait(2.0)
	await _shot("s2-feu-large.png")
	for _i in 1500:
		battle.call("tick", 0.1)
	scene.camera_rig.look_at_point(focus, 70.0, 0.6 + PI)
	await _wait(3.0)
	siege = battle.call("get_siege")
	print("s2_fire_shot: later %d burning, %d burnt" % [int(siege.get("houses_burning", 0)), int(siege.get("houses_burnt", 0))])
	await _shot("s2-ruines.png")
	quit(0)


func _pos(house: Dictionary) -> Vector2:
	return Vector2(float(house["x"]), float(house["z"]))


func _wait(seconds: float) -> void:
	var end := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < end:
		await process_frame


func _shot(file_name: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := _out.path_join(file_name)
	DirAccess.make_dir_recursive_absolute(_out)
	print("s2_fire_shot: %s (%s)" % [path, error_string(image.save_png(path))])
