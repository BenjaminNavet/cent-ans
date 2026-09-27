extends SceneTree

## Lot CB-M4 : capture de la portée au sol et de la comparaison au survol sur la démo autonome de
## bataille (`battle.tscn`, France-Angleterre, graine 1337) : un tireur du joueur sélectionné, son
## arc de portée plaqué au relief, et l'ennemi le plus proche survolé, panneau de comparaison ouvert.
## Usage (avec affichage, pas en headless : le rendu headless ne produit pas d'image) :
##   godot --path game --resolution 1600x900 --script res://tests/cbm4_compare_shot.gd -- --out=<png>
## Sans `--out` : contrôle seulement, sortie `CBM4_COMPARE_CHECK`.
## `--probe` : sonde texte, sans image (headless possible) : cadrage de la capture, part des
## sommets de l'arc à l'écran, rectangle du panneau et ses lignes (`CBM4_COMPARE_PROBE`).


func _init() -> void:
	var out := ""
	var probe := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg == "--probe" or arg.begins_with("--probe="):
			probe = true
	await process_frame
	if DisplayServer.get_name() == "headless":
		root.size = Vector2i(1600, 900)  # headless : la fenêtre par défaut est minuscule (64x64)
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.autoplay = true
	root.add_child(scene)
	for _i in 10:
		await process_frame
	if scene.get("battle") == null or scene.get("compare_panel") == null:
		push_error("cbm4_compare_shot: battle demo failed to stage (run core/build.sh?)")
		quit(1)
		return
	scene.battle.call("set_ai", scene.player_side, false)
	scene.paused = true
	var units: Array = scene.battle.call("get_units")
	var own := _first_shooter(units, scene.player_side)
	if own.is_empty():
		push_error("cbm4_compare_shot: no shooter on the player's side")
		quit(1)
		return
	var own_pos := Vector2(float(own["x"]), float(own["z"]))
	var foe := _nearest_enemy(units, scene.player_side, own_pos)
	if foe.is_empty():
		push_error("cbm4_compare_shot: no enemy")
		quit(1)
		return
	var id := int(own["id"])
	scene.selected.assign([id])
	# Cadrage : derrière le tireur, vers l'ennemi ; l'arc (à sa portée) et l'ennemi dans le champ.
	var foe_pos := Vector2(float(foe["x"]), float(foe["z"]))
	var dir := (foe_pos - own_pos).normalized()
	var reach := float(own["effective_range"])
	var middle := own_pos + dir * minf(reach, own_pos.distance_to(foe_pos)) * 0.5
	scene.camera_rig.edge_pan_enabled = false
	scene.camera_rig.look_at_point(Vector3(middle.x, 0, middle.y), clampf(reach * 1.4, 160.0, 420.0), atan2(-dir.x, -dir.y))
	for _i in 40:
		scene.markers.world_hover = int(foe["id"])
		await process_frame
	scene._update_outlines()
	var arc: BattleRangeArc = scene.range_arc
	var panel: BattleComparePanel = scene.compare_panel
	var ok: bool = arc.shown.has(id) and panel.visible and panel.enemy == int(foe["id"]) and panel.shown.size() == 9
	print("CBM4_COMPARE_CHECK shooter=%d range=%.0f arcs=%s enemy=%d panel=%s %s" % [id, reach, arc.shown, panel.enemy, panel.visible, "OK" if ok else "FAIL"])
	if probe:
		quit(0 if ok and _probe(scene, arc, panel) else 1)
		return
	if out == "":
		quit(0 if ok else 1)
		return
	RenderingServer.force_draw()
	await process_frame
	var image := root.get_viewport().get_texture().get_image()
	if image == null or image.is_empty():
		push_error("cbm4_compare_shot: empty viewport image (headless?)")
		quit(1)
		return
	var err := image.save_png(out)
	print("CBM4_COMPARE_SHOT %s (%s)" % [out, error_string(err)])
	quit(0 if err == OK and ok else 1)


## Sonde texte : part des sommets de l'arc visibles à l'écran, panneau dans la fenêtre et au-dessus
## du bandeau du bas, lignes affichées.
func _probe(scene: Node, arc: BattleRangeArc, panel: BattleComparePanel) -> bool:
	var camera: Camera3D = scene.camera_rig.camera
	var view := camera.get_viewport().get_visible_rect()
	var total := 0
	var inside := 0
	var heights := []
	for child in arc.get_children():
		var node := child as MeshInstance3D
		if node == null or not node.visible or node.mesh == null or node.mesh.get_surface_count() == 0:
			continue
		var vertices: PackedVector3Array = node.mesh.surface_get_arrays(0)[Mesh.ARRAY_VERTEX]
		for v in vertices:
			total += 1
			if not camera.is_position_behind(v) and view.has_point(camera.unproject_position(v)):
				inside += 1
		if not vertices.is_empty():
			var low := INF
			var high := -INF
			for v in vertices:
				low = minf(low, v.y)
				high = maxf(high, v.y)
			heights.append("%.1f..%.1f" % [low, high])
	var rect := panel.get_global_rect()
	var band_top := view.size.y - BattleHud.BAND_HEIGHT - 6.0
	var in_view := view.encloses(rect)
	var above_band := rect.end.y <= band_top + 1.0
	print("CBM4_COMPARE_PROBE arc_vertices=%d on_screen=%.0f%% heights=%s panel=%s in_view=%s above_band=%s" % [total, 100.0 * inside / maxf(total, 1), heights, rect, in_view, above_band])
	for line in BattleComparePanel.LINES:
		var shown: Array = panel.shown.get(line[0], [])
		print("CBM4_COMPARE_LINE %s : %s" % [line[1], " | ".join(shown.map(func(x: Variant) -> String: return str(x)))])
	return total > 0 and inside * 2 >= total and in_view and above_band


func _first_shooter(units: Array, side: String) -> Dictionary:
	for unit in units:
		if bool(unit["present"]) and str(unit["side"]) == side and float(unit.get("effective_range", 0.0)) > 0.0:
			return unit
	return {}


func _nearest_enemy(units: Array, player_side: String, from: Vector2) -> Dictionary:
	var best := {}
	var best_d := INF
	for unit in units:
		if bool(unit["present"]) and str(unit["side"]) != player_side:
			var d := Vector2(float(unit["x"]), float(unit["z"])).distance_to(from)
			if d < best_d:
				best_d = d
				best = unit
	return best
