extends SceneTree

## Lot BR1 : captures des bâtiments du kit sur un champ de bataille avec village (options B5
## `--village --terrain=…`, lues par `BattleTerrain.apply_site_overrides`). Trois vues du village
## (large, moyenne, rapprochée sur la maison la plus proche du centre).
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1600x900 --script res://tests/br1_buildings_shot.gd -- \
##       --out=<dossier> --village --terrain=plains [--season=winter] [--prefix=village]

var _out := ""
var _prefix := "village"


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--prefix="):
			_prefix = arg.trim_prefix("--prefix=")
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
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.configure(sim, index, 11)
	root.add_child(scene)
	for _i in 30:
		await process_frame
	scene.paused = true
	var battle: Object = scene.battle
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
	print("br1_buildings_shot: %s (%s)" % [path, error_string(err)])
