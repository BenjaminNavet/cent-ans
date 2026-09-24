extends SceneTree

## Lot S1 : captures avant / pendant / après l'effondrement physique d'un pan de courtine, sur
## l'assaut de la Guyenne (`debug_stage_siege`). La bataille est mise en pause : l'effondrement
## est déclenché directement sur `WallCollapseFx` (rendu seulement, la simulation n'est pas
## touchée). Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1600x900 --script res://tests/s1_collapse_shot.gd -- --out=<dossier> [--gate]
## `--gate` : la porte enfoncée (planches) et un palier de dégâts du pan voisin (pierres du
## parapet) au lieu d'un pan effondré.

var _out := ""
var _gate_shot := false


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg == "--gate":
			_gate_shot = true
	if _out == "":
		push_error("s1_collapse_shot: --out=<dossier> required")
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
	scene.autoplay = true
	root.add_child(scene)
	for _i in 30:
		await process_frame
	scene.paused = true
	var siege: Dictionary = scene.battle.call("get_siege")
	var pieces: Array = siege["pieces"]
	var gate := int(siege.get("gate", 0))
	for k in pieces.size():
		if str(pieces[k]["kind"]) == "gate":
			gate = k
	print("s1_collapse_shot: gate piece %d, %.1f m" % [gate, (pieces[gate]["a"] as Vector2).distance_to(pieces[gate]["b"])])
	var piece_index := gate if _gate_shot else (gate + 2) % pieces.size()
	var piece: Dictionary = pieces[piece_index]
	var a: Vector2 = piece["a"]
	var b: Vector2 = piece["b"]
	var mid := (a + b) * 0.5
	var dir := (b - a).normalized()
	var outward := Vector2(dir.y, -dir.x)
	var center: Vector2 = siege.get("center", Vector2(600, 560))
	if outward.dot(mid - center) < 0.0:
		outward = -outward
	# Pan : vu de l'extérieur (les blocs tombent vers la caméra) ; porte : vue de l'intérieur de
	# la ville (les vantaux tombent vers la place).
	var side := -1.0 if _gate_shot else 1.0
	var focus := Vector3(mid.x, 0, mid.y) + Vector3(outward.x, 0, outward.y) * 6.0 * side
	scene.camera_rig.edge_pan_enabled = false
	scene.camera_rig.look_at_point(focus, 40.0 if _gate_shot else 70.0, atan2(outward.x * side, outward.y * side) + (0.3 if _gate_shot else 0.45))
	for _i in 40:
		await process_frame
	var siege_view: Node = scene.siege_view
	var fx: WallCollapseFx = siege_view.get("_fx")
	if _gate_shot:
		var views: Array = siege_view.get("_pieces")
		var planks := fx.collapse_gate(piece_index, views[piece_index])
		var stones := fx.drop_parapet((piece_index + 1) % views.size(), views[(piece_index + 1) % views.size()])
		print("s1_collapse_shot: gate %d, %d planks, %d stones" % [piece_index, planks, stones])
		await _wait(0.8)
		await _shot("s1-porte-chute.png")
		await _wait(4.0)
		await _shot("s1-porte-apres.png")
		quit(0)
		return
	await _shot("s1-avant.png")
	var bodies := fx.collapse_wall(piece_index, siege_view.get("_pieces")[piece_index])
	print("s1_collapse_shot: piece %d, %d bodies" % [piece_index, bodies])
	await _wait(1.4)
	await _shot("s1-chute.png")
	await _wait(7.2)
	await _shot("s1-apres.png")
	quit(0)


func _wait(seconds: float) -> void:
	var end := Time.get_ticks_msec() + int(seconds * 1000.0)
	while Time.get_ticks_msec() < end:
		await process_frame


func _shot(file_name: String) -> void:
	await RenderingServer.frame_post_draw
	var image := root.get_texture().get_image()
	var path := _out.path_join(file_name)
	DirAccess.make_dir_recursive_absolute(_out)
	print("s1_collapse_shot: %s (%s)" % [path, error_string(image.save_png(path))])
