extends SceneTree

## Lot CB-M2 : capture de l'aperçu du trajet et du curseur d'attaque sur la démo autonome de
## bataille (`battle.tscn`, France-Angleterre, graine 1337) : une troupe du joueur sélectionnée,
## trajet en pointillés et fantôme d'arrivée vers un point 90 m devant elle (aperçu en direct, comme
## pendant le clic droit maintenu), et le curseur que le cœur donne au survol de l'ennemi le plus
## proche (le curseur système n'apparaît pas dans une capture : son image est posée à l'écran
## sur l'ennemi).
## Usage (avec affichage, pas en headless : le rendu headless ne produit pas d'image) :
##   godot --path game --resolution 1600x900 --script res://tests/cbm2_path_shot.gd -- --out=<png>
## Sans `--out` ni `--probe`, contrôle seulement : sortie `CBM2_PATH_CHECK`.
## `--probe=<dossier>` : mesure sans lecture humaine de l'image. Vue plongeante sur le trajet,
## figurines et drapeaux masqués, trois rendus (trajet masqué / visible / masqué)
## `probe-path-off.png`, `-on.png`, `-off2.png`, et `probe-path.json` avec les points écran du milieu
## des tirets (`dash`) et de points à 6 m de côté (`aside`) ; `tests/cbm2_path_probe.py` compare.


func _init() -> void:
	var out := ""
	var probe := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--probe="):
			probe = arg.trim_prefix("--probe=")
	await process_frame
	if DisplayServer.get_name() == "headless":
		root.size = Vector2i(1600, 900)  # headless : la fenêtre par défaut est minuscule (64x64)
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.autoplay = true
	root.add_child(scene)
	for _i in 10:
		await process_frame
	if scene.battle == null:
		push_error("cbm2_path_shot: battle demo failed to stage (run core/build.sh?)")
		quit(1)
		return
	scene.battle.call("set_ai", scene.player_side, false)
	scene.paused = true
	var units: Array = scene.battle.call("get_units")
	var own := _first(units, scene.player_side, true)
	var foe := _nearest_enemy(units, scene.player_side, Vector2(float(own["x"]), float(own["z"])))
	if own.is_empty() or foe.is_empty():
		push_error("cbm2_path_shot: no regiments")
		quit(1)
		return
	var id := int(own["id"])
	scene.selected.assign([id])
	# Point visé : 90 m devant la troupe, vers l'ennemi, décalé de 30 m sur le côté.
	var own_pos := Vector2(float(own["x"]), float(own["z"]))
	var dir := (Vector2(float(foe["x"]), float(foe["z"])) - own_pos).normalized()
	var aim := own_pos + dir * 90.0 + Vector2(-dir.y, dir.x) * 30.0
	var point := Vector3(aim.x, scene.terrain.height_at(aim.x, aim.y), aim.y)
	var preview: BattlePathPreview = scene.path_preview
	preview.compute([id], scene.units, point, NAN, 0.0)
	# Survol de l'ennemi : contexte du cœur.
	var hover: Dictionary = scene.battle.call("hover_context", float(foe["x"]), float(foe["z"]), PackedInt32Array([id]))
	var context := str(hover.get("context", "none"))
	scene.markers.world_hover = int(foe["id"])
	scene._refresh_view(true)
	var legs: Array = preview.legs
	var path: PackedVector3Array = legs[0]["path"] if not legs.is_empty() else PackedVector3Array()
	var ok: bool = preview.live_mesh().visible and preview.reachable() and path.size() >= 2 and preview.ghost_count() >= 1 and ["melee", "ranged", "ranged_blocked"].has(context)
	print("CBM2_PATH_CHECK legs=%d points=%d ghosts=%d hover=%s %s" % [legs.size(), path.size(), preview.ghost_count(), context, "OK" if ok else "FAIL"])
	if probe != "":
		var probed_ok: bool = await _probe(scene, probe, path)
		quit(0 if ok and probed_ok else 1)
		return
	if out == "":
		quit(0 if ok else 1)
		return
	# Cadrage : derrière la troupe, vers le point visé ; l'ennemi survolé dans le champ.
	var middle := (own_pos + aim) * 0.5
	scene.camera_rig.edge_pan_enabled = false
	scene.camera_rig.look_at_point(Vector3(middle.x, 0, middle.y), 190.0, atan2(-dir.x, -dir.y))
	for _i in 40:
		scene.markers.world_hover = int(foe["id"])
		await process_frame
	# Image du curseur posée sur l'ennemi (le curseur système ne se capture pas).
	var camera: Camera3D = scene.camera_rig.camera
	var foe_world := Vector3(float(foe["x"]), float(foe["y"]) + 1.5, float(foe["z"]))
	var overlay := TextureRect.new()
	overlay.texture = ImageTexture.create_from_image(BattleCursor.image(context))
	overlay.scale = Vector2(1.5, 1.5)
	overlay.position = camera.unproject_position(foe_world) - Vector2(24, 24)
	scene.hud.add_child(overlay)
	await process_frame
	RenderingServer.force_draw()
	await process_frame
	var image := root.get_viewport().get_texture().get_image()
	if image == null or image.is_empty():
		push_error("cbm2_path_shot: empty viewport image (headless?)")
		quit(1)
		return
	var err := image.save_png(out)
	print("CBM2_PATH_SHOT %s (%s)" % [out, error_string(err)])
	quit(0 if err == OK and ok else 1)


func _first(units: Array, side: String, same: bool) -> Dictionary:
	for unit in units:
		if bool(unit["present"]) and (str(unit["side"]) == side) == same:
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


## Mesure : vue plongeante sur le premier tronçon, rendus trajet masqué / visible / masqué, et
## points écran du milieu des tirets et de points à 6 m de côté.
func _probe(scene: Node, dir: String, path: PackedVector3Array) -> bool:
	if path.size() < 2:
		return false
	DirAccess.make_dir_recursive_absolute(dir)
	var preview: BattlePathPreview = scene.path_preview
	for child in scene.get_children():
		if str(child.name).begins_with("Banner"):
			(child as Node3D).visible = false
	if scene.standards != null:
		scene.standards.visible = false
	scene.markers.visible = false
	scene.outlines.visible = false
	scene.hud.root.visible = false
	scene.camera_rig.edge_pan_enabled = false
	scene.camera_rig.process_mode = Node.PROCESS_MODE_DISABLED
	var camera: Camera3D = scene.camera_rig.camera
	var a := path[0]
	var b := path[1]
	var flat := Vector2(b.x - a.x, b.z - a.z)
	var length := flat.length()
	var along := flat / maxf(length, 0.01)
	var mid := a.lerp(b, 0.5)
	var above := length * 0.6 / tan(deg_to_rad(camera.fov * 0.5)) + 10.0
	camera.global_transform = Transform3D(Basis.IDENTITY, mid + Vector3(0, above, 0))
	camera.look_at(mid, Vector3(along.x, 0, along.y))
	var mesh := preview.live_mesh()
	var ghosts: Array = []
	for child in preview.get_children():
		if child is Decal:
			ghosts.append(child)
			(child as Decal).visible = false  # le fantôme se mesure avec la sonde CB-M1
	mesh.visible = false
	for _i in 120:
		scene.soldiers.visible = false
		await process_frame
	var off: Image = await _grab()
	mesh.visible = true
	for _i in 10:
		await process_frame
	var on: Image = await _grab()
	mesh.visible = false
	for _i in 10:
		await process_frame
	var off2: Image = await _grab()
	mesh.visible = true
	if on == null or off == null or off2 == null:
		push_error("cbm2_path_shot: empty viewport image (headless?)")
		return false
	on.save_png(dir.path_join("probe-path-on.png"))
	off.save_png(dir.path_join("probe-path-off.png"))
	off2.save_png(dir.path_join("probe-path-off2.png"))
	var dash := []
	var aside := []
	var side := Vector2(-along.y, along.x)
	var step := BattlePathPreview.DASH + BattlePathPreview.GAP
	var s := BattlePathPreview.DASH * 0.5
	var scale := Vector2(on.get_size()) / camera.get_viewport().get_visible_rect().size
	while s < length - 1.0:
		for offset in [0.0, 6.0]:
			var p: Vector2 = Vector2(a.x, a.z) + along * s + side * offset
			var world := Vector3(p.x, scene.terrain.height_at(p.x, p.y) + BattlePathPreview.LIFT, p.y)
			if camera.is_position_behind(world):
				continue
			var px := camera.unproject_position(world) * scale
			if px.x < 0 or px.y < 0 or px.x >= on.get_width() or px.y >= on.get_height():
				continue
			(dash if offset == 0.0 else aside).append([int(px.x), int(px.y)])
		s += step
	var file := FileAccess.open(dir.path_join("probe-path.json"), FileAccess.WRITE)
	file.store_string(JSON.stringify({"length": length, "dash": dash, "aside": aside}))
	file.close()
	print("CBM2_PATH_PROBE dash=%d aside=%d" % [dash.size(), aside.size()])
	return dash.size() > 5 and aside.size() > 5


func _grab() -> Image:
	RenderingServer.force_draw()
	await process_frame
	var image := root.get_viewport().get_texture().get_image()
	return null if image == null or image.is_empty() else image
