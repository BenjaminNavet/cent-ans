extends SceneTree

## Lot A6-L14 : sol de bataille de campagne, deux vues (déploiement, vue haute).
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1600x900 --script res://tests/a6_battle_ground_shot.gd -- \
##       --out=<dossier> [--prefix=ground] [--terrain=plains] [--season=summer]
## Les options terrain/saison sont lues par `BattleTerrain` (comme br1_buildings_shot).

var _out := ""
var _prefix := "ground"


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--prefix="):
			_prefix = arg.trim_prefix("--prefix=")
	if _out == "":
		push_error("a6_battle_ground_shot: --out=<dossier> required")
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
	scene.camera_rig.edge_pan_enabled = false
	var focus := Vector3(600, 0, 400)
	scene.camera_rig.look_at_point(focus, 110.0, 0.55)
	await _wait(2.5)
	await _shot(_prefix + "-deploiement.png")
	scene.camera_rig.look_at_point(focus, 520.0, 1.1)
	await _wait(2.5)
	await _shot(_prefix + "-haute.png")
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
	print("a6_battle_ground_shot: %s (%s)" % [path, error_string(image.save_png(path))])
