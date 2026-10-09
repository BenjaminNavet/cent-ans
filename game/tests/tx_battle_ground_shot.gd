extends SceneTree

## TX T2c : captures du sol de bataille régional (un biome par lancement, vues au choix).
## Usage (avec affichage ; passer par tools/godot_bg.sh, jamais en direct) :
##   tools/godot_bg.sh --path game --resolution 1280x720 --script res://tests/tx_battle_ground_shot.gd -- \
##       --out=<dossier> --prefix=b02 --battle-biome=2 --terrain=plains \
##       [--views=deploy:70:0.5,micro:5:0.9] [--legacy-textures]
## `--battle-biome=N` force le biome du sol (sinon celui de la province / du terrain).
## Chaque vue : nom:distance(m):inclinaison(rad), sur le centre du champ.

var _out := ""
var _prefix := "tx"
var _views := "deploy:70:0.5"


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--prefix="):
			_prefix = arg.trim_prefix("--prefix=")
		elif arg.begins_with("--views="):
			_views = arg.trim_prefix("--views=")
	if _out == "":
		push_error("tx_battle_ground_shot: --out=<dossier> requis")
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
	for view in _views.split(",", false):
		var parts := view.split(":")
		scene.camera_rig.look_at_point(focus, float(parts[1]), float(parts[2]))
		await _wait(2.5)
		await _shot("%s-%s.png" % [_prefix, parts[0]])
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
	print("tx_battle_ground_shot: %s (%s)" % [path, error_string(image.save_png(path))])
