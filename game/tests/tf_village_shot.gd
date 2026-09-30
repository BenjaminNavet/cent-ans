extends SceneTree

## Lot TF : une capture d'un village de bataille (colombage régional), vue mi-rapprochée sur la
## maison la plus proche du centre. Options du champ lues par `BattleTerrain.apply_site_overrides`
## (`--village --terrain=… --province=…`). Usage (avec affichage) :
##   godot --path game --resolution 1280x720 --script res://tests/tf_village_shot.gd -- \
##       --out=<png absolu> --village --terrain=plains --province=prov_normandie [--dist=40] [--yaw=0.6]
## Ajouter `--no-speech --no-cinematic` : le discours d'ouverture reprend la caméra.

var _out := ""
var _dist := 40.0
var _yaw := 0.6


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--dist="):
			_dist = float(arg.trim_prefix("--dist="))
		elif arg.begins_with("--yaw="):
			_yaw = float(arg.trim_prefix("--yaw="))
	if _out == "":
		push_error("tf_village_shot: --out=<png> required")
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
	# Pas de `scene.paused` : le rendu 3D passe par une vue qui se fige quand la scène est en pause.
	var terrain: Dictionary = scene.battle.call("get_terrain")
	var houses: Array = terrain.get("village", {}).get("houses", [])
	if houses.is_empty():
		push_error("tf_village_shot: no village on this field")
		quit(1)
		return
	var center := Vector2.ZERO
	for house in houses:
		center += Vector2(float(house["x"]), float(house["z"]))
	center /= houses.size()
	print("tf_village_shot: %d houses, province %s, centre %s" % [houses.size(), scene.terrain.province_id, center])
	scene.camera_rig.edge_pan_enabled = false
	# Laisse passer le cadrage d'ouverture (glissement vers le déploiement) avant de viser.
	for pass_index in 2:
		var end := Time.get_ticks_msec() + 2500
		while Time.get_ticks_msec() < end:
			await process_frame
		scene.camera_rig.look_at_point(Vector3(center.x, 0, center.y), _dist, _yaw)
	for layer in scene.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_viewport().get_texture().get_image()
	print("tf_village_shot: %s %s" % [_out, error_string(image.save_png(_out))])
	quit(0)
