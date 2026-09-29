extends SceneTree

## NT1 (ADR 0126) : une capture de chaque type de place assiégée (château, bourg fortifié, cité),
## vue plongeante du côté de l'assaillant. Chaque siège est monté par
## `CampaignSim.debug_stage_place_siege` sur la première localité tracée dans ce type.
## Le script écrit les images sans les lire ; il affiche aussi le type, le nombre de bâtiments et
## de donjons reçus du cœur.
## Usage (avec affichage) :
##   godot --path game --resolution 1280x720 --script res://tests/nt1_siege_shot.gd -- \
##     [--out=<dossier>] [--kinds=castle,borough,city]

var _out := ""
var _kinds: PackedStringArray = ["castle", "borough", "city"]


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--kinds="):
			_kinds = arg.trim_prefix("--kinds=").split(",", false)
	if _out == "":
		_out = ProjectSettings.globalize_path("res://").path_join("../docs/audit/captures/nt").simplify_path()
	DirAccess.make_dir_recursive_absolute(_out)
	await process_frame
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var failures := 0
	for kind in _kinds:
		if not await _capture(data_dir, kind):
			failures += 1
	quit(1 if failures > 0 else 0)


func _capture(data_dir: String, kind: String) -> bool:
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		push_error("nt1_siege_shot: no campaign")
		return false
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_place_siege", armies[0], kind)
	if index < 0:
		push_error("nt1_siege_shot: cannot stage a %s siege" % kind)
		return false
	BattleScene.demo_args = PackedStringArray(["--no-speech"])
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.configure(sim, index, 7)
	root.add_child(scene)
	for _i in 30:
		await process_frame
	var battle: Object = scene.battle
	if scene.deployment != null and scene.deployment.active:
		scene.deployment.finish()
	scene.camera_rig.edge_pan_enabled = false
	var siege: Dictionary = battle.call("get_siege")
	var houses: Array = siege.get("houses", [])
	var keeps := houses.filter(func(h: Dictionary) -> bool: return bool(h.get("keep", false))).size()
	print("nt1_siege_shot: %s -> place=%s houses=%d keeps=%d towers=%d" % [
		kind, siege.get("place", "?"), houses.size(), keeps, (siege.get("towers", []) as Array).size()])
	var center: Vector2 = siege.get("center", Vector2(600, 560))
	scene.camera_rig.look_at_point(Vector3(center.x, 0.0, center.y), 330.0 if kind != "castle" else 240.0, 0.0)
	await _wait(1.5)
	if DisplayServer.get_name() == "headless":
		# Sans affichage, aucune image : le script ne vérifie que la mise en place du siège.
		print("nt1_siege_shot: headless, no capture for %s" % kind)
	else:
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		var path := _out.path_join("nt1_%s.png" % kind)
		print("nt1_siege_shot: %s (%s)" % [path, error_string(image.save_png(path))])
	scene.queue_free()
	await process_frame
	return siege.get("place", "") == kind


func _wait(seconds: float) -> void:
	var end := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < end:
		await process_frame
