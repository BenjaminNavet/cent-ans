extends SceneTree

## Lot CB-M3 : capture des ordres en file sur la démo autonome de bataille (`battle.tscn`,
## France-Angleterre, graine 1337) : une troupe du joueur sélectionnée, un ordre de marche puis
## trois ordres en file (Maj + clic droit) ; trajet segment par segment, points de passage
## numérotés au sol (1 à 4) et fantôme d'arrivée au dernier point.
## Usage (avec affichage, pas en headless : le rendu headless ne produit pas d'image) :
##   godot --path game --resolution 1600x900 --script res://tests/cbm3_queue_shot.gd -- --out=<png>
## `--probe` : contrôle texte seulement, sans image (headless possible) : sortie
## `CBM3_QUEUE_PROBE` (ordres en file dans le cœur, numéros visibles, points écran des numéros),
## code de sortie 0 si tout est en place.


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
	if scene.battle == null or not scene.battle.has_method("preview_path_from"):
		push_error("cbm3_queue_shot: battle demo failed to stage (run core/build.sh?)")
		quit(1)
		return
	scene.battle.call("set_ai", scene.player_side, false)
	scene.paused = true
	var units: Array = scene.battle.call("get_units")
	var own := _first(units, scene.player_side)
	if own.is_empty():
		push_error("cbm3_queue_shot: no regiments")
		quit(1)
		return
	var id := int(own["id"])
	scene.selected.assign([id])
	# Un ordre de marche puis trois ordres en file, en zigzag devant la troupe (vers l'ennemi).
	var own_pos := Vector2(float(own["x"]), float(own["z"]))
	var forward := Vector2(0, 1) if str(own["side"]) == "attacker" else Vector2(0, -1)
	var side := Vector2(-forward.y, forward.x)
	var points: Array[Vector2] = [
		own_pos + forward * 45.0 + side * 25.0,
		own_pos + forward * 85.0 - side * 20.0,
		own_pos + forward * 120.0 + side * 35.0,
		own_pos + forward * 150.0 - side * 10.0,
	]
	for k in points.size():
		var command := {"type": "move", "units": [id], "x": points[k].x, "z": points[k].y, "run": false}
		if k > 0:
			command["queue"] = true
		var result: Dictionary = scene.issue(command)
		if not bool(result.get("ok", false)):
			push_error("cbm3_queue_shot: order %d refused: %s" % [k, result])
	scene._refresh_view(true)
	var preview: BattlePathPreview = scene.path_preview
	preview._orders_key = ""
	preview.update_orders(scene.units, scene.selected, 0.0)
	var queue: Array = scene.battle.call("get_units")[id].get("queue", [])
	var numbers := preview.waypoint_numbers()
	var ok: bool = queue.size() == 3 and numbers == ["1", "2", "3", "4"] and preview.orders_mesh().visible and preview.ghost_count() >= 1
	# Cadrage : derrière la troupe, le zigzag entier dans le champ.
	var middle := own_pos + forward * 80.0
	scene.camera_rig.edge_pan_enabled = false
	scene.camera_rig.look_at_point(Vector3(middle.x, 0, middle.y), 260.0, atan2(-forward.x, -forward.y))
	for _i in 30:
		await process_frame
	preview._orders_key = ""
	preview.update_orders(scene.units, scene.selected, 100.0)
	var camera: Camera3D = scene.camera_rig.camera
	var screens: Array = []
	var inside := 0
	for child in preview.get_children():
		if child is Label3D and (child as Label3D).visible:
			var label := child as Label3D
			var at := camera.unproject_position(label.global_position)
			screens.append([label.text, int(at.x), int(at.y)])
			if Rect2(Vector2.ZERO, camera.get_viewport().get_visible_rect().size).has_point(at) and not camera.is_position_behind(label.global_position):
				inside += 1
	ok = ok and inside == numbers.size()
	print("CBM3_QUEUE_PROBE queue=%d numbers=%s on_screen=%d ghosts=%d labels=%s %s" % [queue.size(), numbers, inside, preview.ghost_count(), screens, "OK" if ok else "FAIL"])
	if probe or out == "":
		quit(0 if ok else 1)
		return
	await process_frame
	RenderingServer.force_draw()
	await process_frame
	var image := root.get_viewport().get_texture().get_image()
	if image == null or image.is_empty():
		push_error("cbm3_queue_shot: empty viewport image (headless?)")
		quit(1)
		return
	if image.get_width() > 1600:
		image.resize(1600, int(image.get_height() * 1600.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var err := image.save_png(out)
	print("CBM3_QUEUE_SHOT %s (%s)" % [out, error_string(err)])
	quit(0 if err == OK and ok else 1)


func _first(units: Array, side: String) -> Dictionary:
	for unit in units:
		if bool(unit["present"]) and str(unit["side"]) == side:
			return unit
	return {}
