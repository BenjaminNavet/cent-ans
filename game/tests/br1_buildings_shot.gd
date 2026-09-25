extends SceneTree

## Lot BR1 : captures des bâtiments du kit sur un champ de bataille avec village (options B5
## `--village --terrain=…`, lues par `BattleTerrain.apply_site_overrides`). Trois vues du village
## (large, moyenne, rapprochée sur la maison la plus proche du centre).
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1600x900 --script res://tests/br1_buildings_shot.gd -- \
##       --out=<dossier> --village --terrain=plains [--season=winter] [--prefix=village]
## `--siege` : assaut de la Guyenne (`debug_stage_siege`), vues de la ville assiégée. Chaque vue
## imprime appels de dessin et primitives (comparaison avant/après).

var _out := ""
var _prefix := "village"
var _siege := false


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--prefix="):
			_prefix = arg.trim_prefix("--prefix=")
		elif arg == "--siege":
			_siege = true
	if _out == "":
		push_error("br1_buildings_shot: --out=<dossier> required")
		quit(1)
		return
	await process_frame
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		quit(1)
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_siege", armies[0], "prov_guyenne") if _siege else sim.call("debug_stage_battle", armies[0], armies[1])
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.configure(sim, index, 11)
	root.add_child(scene)
	for _i in 30:
		await process_frame
	scene.paused = true
	var battle: Object = scene.battle
	if _siege:
		await _siege_shots(scene, battle)
		quit(0)
		return
	var terrain: Dictionary = battle.call("get_terrain")
	var houses: Array = terrain.get("village", {}).get("houses", [])
	if houses.is_empty():
		push_error("br1_buildings_shot: no village on this field")
		quit(1)
		return
	var center := Vector2.ZERO
	for house in houses:
		center += Vector2(float(house["x"]), float(house["z"]))
	center /= houses.size()
	var nearest: Dictionary = houses[0]
	for house in houses:
		if Vector2(float(house["x"]), float(house["z"])).distance_to(center) < Vector2(float(nearest["x"]), float(nearest["z"])).distance_to(center):
			nearest = house
	print("br1_buildings_shot: %d houses" % houses.size())
	scene.camera_rig.edge_pan_enabled = false
	var focus := Vector3(center.x, 0, center.y)
	scene.camera_rig.look_at_point(focus, 150.0, 0.7)
	await _wait(2.5)
	await _shot(_prefix + "-large.png")
	scene.camera_rig.look_at_point(focus, 70.0, 2.4)
	await _wait(2.0)
	await _shot(_prefix + "-moyen.png")
	var near := Vector3(float(nearest["x"]), 0, float(nearest["z"]))
	scene.camera_rig.look_at_point(near, 30.0, 0.3)
	await _wait(2.0)
	await _shot(_prefix + "-proche.png")
	quit(0)


func _siege_shots(scene: Node, battle: Object) -> void:
	var siege: Dictionary = battle.call("get_siege")
	var center: Vector2 = siege.get("center", Vector2(600, 560))
	var focus := Vector3(center.x, 0, center.y)
	scene.camera_rig.edge_pan_enabled = false
	scene.camera_rig.look_at_point(focus + Vector3(0, 0, 60), 230.0, 0.4)
	await _wait(2.5)
	await _shot(_prefix + "-large.png")
	scene.camera_rig.look_at_point(focus + Vector3(60, 0, 20), 55.0, 2.2)
	await _wait(2.0)
	await _shot(_prefix + "-rue.png")


func _wait(seconds: float) -> void:
	var end := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < end:
		await process_frame


func _shot(file: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_viewport().get_texture().get_image()
	var path := _out.path_join(file)
	DirAccess.make_dir_recursive_absolute(_out)
	var err := image.save_png(path)
	var calls := RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME)
	var prims := RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME)
	print("br1_buildings_shot: %s (%s) %d draw calls, %.2f M primitives" % [path, error_string(err), calls, prims / 1.0e6])
