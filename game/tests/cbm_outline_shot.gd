extends SceneTree

## Lot CB-M1 : capture des contours de formation (décales) sur la démo autonome de bataille
## (`battle.tscn`, France-Angleterre, graine 1337) : une troupe du joueur sélectionnée (trait
## plein), un ennemi ciblé (rouge pulsé) et un autre ennemi survolé (pointillé rouge).
## Usage (avec affichage, pas en headless : le rendu headless ne produit pas d'image) :
##   godot --path game --resolution 1600x900 --script res://tests/cbm_outline_shot.gd -- --out=<png>
## Sans `--out`, contrôle seulement (états des décales) : sortie `CBM_OUTLINE_CHECK`.
## `--probe=<dossier>` : mesure sans lecture humaine de l'image. Pour la troupe sélectionnée et
## l'ennemi ciblé : caméra rapprochée, figurines et drapeaux masqués, deux rendus (décales
## visibles / masquées) `probe-<nom>-on.png` / `-off.png`, et `probe-<nom>.json` avec les points
## écran du cœur (30 % central) et de la bande du bord ; `tests/cbm_outline_probe.py` compare.

const S := BattleFormationOutline.State


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
		push_error("cbm_outline_shot: battle demo failed to stage (run core/build.sh?)")
		quit(1)
		return
	scene.battle.call("set_ai", scene.player_side, false)
	var units: Array = scene.battle.call("get_units")
	var own := _first(units, scene.player_side, true)
	if own.is_empty():
		push_error("cbm_outline_shot: no player regiment")
		quit(1)
		return
	var own_pos := Vector2(float(own["x"]), float(own["z"]))
	var foes := _enemies_by_distance(units, scene.player_side, own_pos)
	if foes.size() < 2:
		push_error("cbm_outline_shot: fewer than two enemy regiments")
		quit(1)
		return
	var target: Dictionary = foes[0]
	var hovered: Dictionary = foes[1]
	scene.selected.assign([int(own["id"])])
	scene.issue({"type": "attack", "units": [int(own["id"])], "target": int(target["id"]), "run": false})
	scene.paused = false
	for _i in 3:
		await process_frame
	scene.paused = true
	scene.markers.world_hover = int(hovered["id"])
	scene._refresh_view(true)
	var outlines: BattleFormationOutline = scene.outlines
	var ok: bool = outlines.state_of(int(own["id"])) == S.SELECTED and outlines.state_of(int(target["id"])) == S.ENEMY_TARGETED and outlines.state_of(int(hovered["id"])) == S.ENEMY_HOVERED
	print("CBM_OUTLINE_CHECK selected=%d targeted=%d hovered=%d decals=%d %s" % [outlines.state_of(int(own["id"])), outlines.state_of(int(target["id"])), outlines.state_of(int(hovered["id"])), outlines.decal_count(), "OK" if ok else "FAIL"])
	if probe != "":
		var probed_ok: bool = await _probe(scene, probe, {"selected": int(own["id"]), "target": int(target["id"])})
		quit(0 if ok and probed_ok else 1)
		return
	if out == "":
		quit(0 if ok else 1)
		return
	# Cadrage : derrière la troupe du joueur, en regardant vers la cible, assez près pour que la
	# sélection et la cible occupent une bonne part du cadre.
	var target_pos := Vector2(float(target["x"]), float(target["z"]))
	var middle := (own_pos + target_pos) * 0.5
	var gap := own_pos.distance_to(target_pos)
	var dir := (target_pos - own_pos).normalized()
	scene.camera_rig.edge_pan_enabled = false
	scene.camera_rig.look_at_point(Vector3(middle.x, 0, middle.y), clampf(gap * 0.6 + 60.0, 110.0, 300.0), atan2(-dir.x, -dir.y))
	for _i in 40:
		scene.markers.world_hover = int(hovered["id"])  # le survol réel suit la souris : on le maintient
		await process_frame
	RenderingServer.force_draw()
	await process_frame
	var image := root.get_viewport().get_texture().get_image()
	if image == null or image.is_empty():
		push_error("cbm_outline_shot: empty viewport image (headless?)")
		quit(1)
		return
	var err := image.save_png(out)
	print("CBM_OUTLINE_SHOT %s (%s)" % [out, error_string(err)])
	quit(0 if err == OK and ok else 1)


func _first(units: Array, side: String, same: bool) -> Dictionary:
	for unit in units:
		if bool(unit["present"]) and (str(unit["side"]) == side) == same:
			return unit
	return {}


func _enemies_by_distance(units: Array, player_side: String, from: Vector2) -> Array:
	var foes := []
	for unit in units:
		if bool(unit["present"]) and str(unit["side"]) != player_side and str(unit["state"]) != "routing":
			foes.append(unit)
	foes.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		return Vector2(float(a["x"]), float(a["z"])).distance_to(from) < Vector2(float(b["x"]), float(b["z"])).distance_to(from))
	return foes


## Mesure : pour chaque décale nommée, rendu décales visibles puis masquées, et points écran.
func _probe(scene: Node, dir: String, ids: Dictionary) -> bool:
	DirAccess.make_dir_recursive_absolute(dir)
	var outlines: BattleFormationOutline = scene.outlines
	outlines.set_process(false)  # pulsé figé : opacité pleine pour la mesure
	for child in scene.get_children():
		if str(child.name).begins_with("Banner"):
			(child as Node3D).visible = false
	if scene.standards != null:
		scene.standards.visible = false
	scene.markers.visible = false
	scene.hud.root.visible = false
	scene.camera_rig.edge_pan_enabled = false
	var camera: Camera3D = scene.camera_rig.camera
	var all_ok := true
	for probe_name in ids:
		var id := int(ids[probe_name])
		var decal := outlines.decal_of(id)
		decal.modulate.a = 1.0
		var size := decal.size
		# Vue plongeante (rig figé) : une vue rasante réduit le trait à 2-3 px et fausse la mesure.
		scene.camera_rig.process_mode = Node.PROCESS_MODE_DISABLED
		var above := maxf(size.x, size.z) * 0.6 / tan(deg_to_rad(camera.fov * 0.5)) + 10.0
		camera.global_transform = Transform3D(Basis.IDENTITY, decal.global_position + Vector3(0, above, 0))
		camera.look_at(decal.global_position, Vector3(-sin(decal.rotation.y), 0, -cos(decal.rotation.y)))
		# Ordre masquée / visible / masquée après stabilisation (exposition, effets temporels) :
		# l'écart entre les deux rendus masqués donne le bruit de fond autour du rendu visible.
		outlines.visible = false
		for _i in 120:
			scene.soldiers.visible = false
			await process_frame
		var off: Image = await _grab()
		outlines.visible = true
		for _i in 10:
			await process_frame
		var on: Image = await _grab()
		outlines.visible = false
		for _i in 10:
			await process_frame
		var off2: Image = await _grab()
		outlines.visible = true
		if off2 != null:
			off2.save_png(dir.path_join("probe-%s-off2.png" % probe_name))
		if on == null or off == null:
			push_error("cbm_outline_shot: empty viewport image (headless?)")
			return false
		on.save_png(dir.path_join("probe-%s-on.png" % probe_name))
		off.save_png(dir.path_join("probe-%s-off.png" % probe_name))
		var core := []
		var edge := []
		var basis := decal.global_transform.basis.orthonormalized()
		var half := Vector2(size.x, size.z) * 0.5
		var inset := 0.25  # m : au milieu d'un trait d'au moins 0,5 m
		for i in 41:
			for j in 41:
				var u := -0.5 + i / 40.0
				var v := -0.5 + j / 40.0
				var local := Vector2(u * size.x, v * size.z)
				var kind := ""
				if absf(u) <= 0.15 and absf(v) <= 0.15:
					kind = "core"
				elif i == 0 or j == 0 or i == 40 or j == 40:
					local = Vector2(clampf(local.x, -half.x + inset, half.x - inset), clampf(local.y, -half.y + inset, half.y - inset))
					kind = "edge"
				if kind == "":
					continue
				var world := decal.global_position + basis * Vector3(local.x, 0, local.y)
				world.y = scene.terrain.height_at(world.x, world.z)
				if camera.is_position_behind(world):
					continue
				# `unproject_position` rend des coordonnées de canevas (étirement du projet) : on
				# les ramène aux pixels de l'image.
				var px := camera.unproject_position(world) * Vector2(on.get_size()) / camera.get_viewport().get_visible_rect().size
				if px.x < 0 or px.y < 0 or px.x >= on.get_width() or px.y >= on.get_height():
					continue
				(core if kind == "core" else edge).append([int(px.x), int(px.y)])
		var info := {"state": outlines.state_of(id), "size": [size.x, size.z], "core": core, "edge": edge}
		var file := FileAccess.open(dir.path_join("probe-%s.json" % probe_name), FileAccess.WRITE)
		file.store_string(JSON.stringify(info))
		file.close()
		print("CBM_OUTLINE_PROBE %s core=%d edge=%d" % [probe_name, core.size(), edge.size()])
		all_ok = all_ok and core.size() > 10 and edge.size() > 10
	return all_ok


func _grab() -> Image:
	RenderingServer.force_draw()
	await process_frame
	var image := root.get_viewport().get_texture().get_image()
	return null if image == null or image.is_empty() else image
