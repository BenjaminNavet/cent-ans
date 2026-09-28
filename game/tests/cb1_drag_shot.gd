extends SceneTree

## Lot CB1 : capture de la formation au glisser sur la démo autonome de bataille (`battle.tscn`,
## France-Angleterre, graine 1337) : deux régiments du joueur sélectionnés, aperçu en direct d'un
## glisser-droit large (fantômes d'arrivée à la largeur répartie par le cœur, plus larges et moins
## profonds que les régiments), puis la sélection verrouillée (Ctrl+G) : cadenas sur leurs cartes.
## Usage (avec affichage, pas en headless : le rendu headless ne produit pas d'image) :
##   godot --path game --resolution 1600x900 --script res://tests/cb1_drag_shot.gd -- --out=<png>
## `--probe` : contrôle texte seulement, sans image (headless possible) : sortie `CB1_DRAG_PROBE`
## (tailles des fantômes, trajets accessibles, verrou et cadenas), code 0 si tout est en place.


func _init() -> void:
	var out := ""
	var probe := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg == "--probe":
			probe = true
	await process_frame
	if DisplayServer.get_name() == "headless":
		root.size = Vector2i(1600, 900)  # headless : la fenêtre par défaut est minuscule (64x64)
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.autoplay = true
	root.add_child(scene)
	for _i in 10:
		await process_frame
	if scene.battle == null or not scene.battle.has_method("formation_extent"):
		push_error("cb1_drag_shot: battle demo failed to stage (run core/build.sh?)")
		quit(1)
		return
	scene.battle.call("set_ai", scene.player_side, false)
	scene.paused = true
	var units: Array = scene.battle.call("get_units")
	var own: Array = []
	for unit in units:
		if bool(unit["present"]) and str(unit["side"]) == scene.player_side and str(unit["category"]) == "infantry" and own.size() < 2:
			own.append(unit)
	if own.size() < 2:
		push_error("cb1_drag_shot: fewer than two infantry regiments")
		quit(1)
		return
	var ids: Array[int] = [int(own[0]["id"]), int(own[1]["id"])]
	scene.selected.assign(ids)
	# Glisser de 240 m devant les deux régiments, en deçà de la rivière de la démo (terre ferme).
	var center := Vector2((float(own[0]["x"]) + float(own[1]["x"])) * 0.5, (float(own[0]["z"]) + float(own[1]["z"])) * 0.5)
	var forward := Vector2(0, 1) if scene.player_side == "attacker" else Vector2(0, -1)
	var side := Vector2(-forward.y, forward.x)
	var mid := center + forward * 45.0
	var p0 := mid - side * 120.0
	var p1 := mid + side * 120.0
	var cam_at := Vector3(center.x, 0, center.y) - Vector3(forward.x, 0, forward.y) * 400.0
	var line := FormationDrag.drag_line(Vector3(p0.x, 0, p0.y), Vector3(p1.x, 0, p1.y), cam_at)
	var preview: BattlePathPreview = scene.path_preview
	scene.camera_rig.edge_pan_enabled = false
	scene.camera_rig.look_at_point(Vector3(mid.x, 0, mid.y), 300.0, atan2(-forward.x, -forward.y))
	for _i in 20:
		await process_frame
	var ok := bool(line["ok"])
	var mid3: Vector3 = line["mid"]
	preview.compute(ids, scene.units, mid3, float(line["facing"]), 0.0, false, float(line["width"]))
	var sizes := preview.live_ghost_sizes()
	var widths: Array = []
	for unit in own:
		widths.append([snappedf(float(unit["width"]), 0.1), snappedf(float(unit["depth"]), 0.1)])
	var reachable := preview.reachable() and preview.refusal() == ""
	var wider := sizes.size() == 2
	for k in sizes.size():
		wider = wider and sizes[k].x > float(own[k]["width"])
	# Verrouillage de la sélection : cadenas sur les cartes.
	var tag: int = scene.hud.groups.toggle_lock(ids, scene.units)
	scene.hud.update_cards(scene.units, scene.player_side, scene.selected)
	var padlocks := 0
	for id in ids:
		var card: UnitCard = scene.hud._cards.get(id)
		if card != null and card.locked:
			padlocks += 1
	ok = ok and reachable and wider and tag > 0 and padlocks == 2
	print("CB1_DRAG_PROBE width=%.1f ghosts=%s units=%s reachable=%s lock=%d padlocks=%d %s" % [float(line["width"]), sizes, widths, reachable, tag, padlocks, "OK" if ok else "FAIL"])
	if probe or out == "":
		quit(0 if ok else 1)
		return
	for _i in 10:
		await process_frame
	RenderingServer.force_draw()
	await process_frame
	var image := root.get_viewport().get_texture().get_image()
	if image == null or image.is_empty():
		push_error("cb1_drag_shot: empty viewport image (headless?)")
		quit(1)
		return
	if image.get_width() > 1600:
		image.resize(1600, int(image.get_height() * 1600.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var err := image.save_png(out)
	print("CB1_DRAG_SHOT %s (%s)" % [out, error_string(err)])
	quit(0 if err == OK and ok else 1)
