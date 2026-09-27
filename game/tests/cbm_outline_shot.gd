extends SceneTree

## Lot CB-M1 : capture des contours de formation (décales) sur la démo autonome de bataille
## (`battle.tscn`, France-Angleterre, graine 1337) : une troupe du joueur sélectionnée (trait
## plein), un ennemi ciblé (rouge pulsé) et un autre ennemi survolé (pointillé rouge).
## Usage (avec affichage, pas en headless : le rendu headless ne produit pas d'image) :
##   godot --path game --resolution 1600x900 --script res://tests/cbm_outline_shot.gd -- --out=<png>
## Sans `--out`, contrôle seulement (états des décales) : sortie `CBM_OUTLINE_CHECK`.

const S := BattleFormationOutline.State


func _init() -> void:
	var out := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
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
	if out == "":
		quit(0 if ok else 1)
		return
	# Cadrage : du côté du joueur, en regardant vers la cible, les trois contours dans le champ.
	var target_pos := Vector2(float(target["x"]), float(target["z"]))
	var middle := (own_pos + target_pos) * 0.5
	var gap := own_pos.distance_to(target_pos)
	var dir := (target_pos - own_pos).normalized()
	scene.camera_rig.edge_pan_enabled = false
	scene.camera_rig.look_at_point(Vector3(middle.x, 0, middle.y), clampf(gap * 0.9 + 120.0, 180.0, 460.0), atan2(-dir.x, -dir.y))
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
