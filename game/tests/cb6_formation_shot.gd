extends SceneTree

## Lot CB6 : capture des formations de groupe sur la démo autonome de bataille (`battle.tscn`,
## France-Angleterre, graine 1337) : phase de déploiement ouverte, préréglage « La herse » actif
## (Alt+Maj+2), « Placer en formation » appuyé : toute l'armée du joueur en fantômes à ses places
## (hommes d'armes au centre, archers en avant sur les ailes tournés vers l'intérieur, réserve
## montée derrière), sélecteur visible en bas à droite (bouton « Valider la formation »).
## Usage (avec affichage, pas en headless : le rendu headless ne produit pas d'image) :
##   godot --path game --resolution 1600x900 --script res://tests/cb6_formation_shot.gd -- --out=<png>
## `--probe` : contrôle texte seulement, sans image (headless possible) : sortie `CB6_FORMATION_PROBE`
## (préréglages, places par rôle, fantômes, zone, sélecteur), code 0 si tout est en place.


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
	scene.paused = true  # aucun tick : le déploiement reste ouvert
	root.add_child(scene)
	for _i in 10:
		await process_frame
	if scene.battle == null or not scene.battle.has_method("formation_slots"):
		push_error("cb6_formation_shot: battle demo failed to stage (run core/build.sh?)")
		quit(1)
		return
	scene.battle.call("set_ai", scene.player_side, false)
	var controller := DeploymentController.new()
	controller.name = "Deployment"
	scene.add_child(controller)
	if not controller.open(scene):
		push_error("cb6_formation_shot: deployment refused")
		quit(1)
		return
	scene.deployment = controller
	scene.units = scene.battle.call("get_units")
	var picker: BattleFormationPicker = scene.formation_picker
	var harrow := picker.select_index(2)
	scene.selected.clear()
	picker.on_place_pressed()
	var places: Array = picker.pending
	var anchor := picker.default_anchor()
	var point: Vector2 = anchor["point"]
	var facing := float(anchor["facing"])
	# Places par rôle, dans le repère du front (s : droite, t : vers l'ennemi).
	var right := Vector2(cos(facing), -sin(facing))
	var forward := Vector2(sin(facing), cos(facing))
	var roles := {}
	var inward := 0
	var archers := 0
	for place in places:
		var unit := _unit(scene, int(place["id"]))
		var d := Vector2(float(place["x"]), float(place["z"])) - point
		var s := d.dot(right)
		var t := d.dot(forward)
		var key := str(unit.get("category", "?"))
		if bool(unit.get("mounted", false)):
			key = "mounted"
		if not roles.has(key):
			roles[key] = []
		(roles[key] as Array).append("%.0f/%.0f" % [s, t])
		if key == "ranged":
			archers += 1
			var dir := Vector2(sin(float(place["facing"])), cos(float(place["facing"])))
			if dir.dot(right) * s < 0.0:
				inward += 1
	var zone: Dictionary = controller.zone
	var inside := true
	for place in places:
		inside = inside and float(place["x"]) >= float(zone["x0"]) - 0.01 and float(place["x"]) <= float(zone["x1"]) + 0.01 and float(place["z"]) >= float(zone["z0"]) - 0.01 and float(place["z"]) <= float(zone["z1"]) + 0.01
	var ghosts: int = scene.path_preview.live_ghost_sizes().size()
	var own := 0
	for unit in scene.units:
		if str(unit["side"]) == scene.player_side and bool(unit["present"]):
			own += 1
	var shown := picker.visible and picker.place_button.visible and picker.place_button.text == "Valider la formation"
	var ok := harrow and picker.active_id == "harrow" and places.size() == own and ghosts == own and inside and shown and archers > 0 and inward == archers
	print("CB6_FORMATION_PROBE presets=%d preset=%s places=%d/%d ghosts=%d inside=%s archers_inward=%d/%d picker=%s rect=%s roles=%s %s" % [picker.presets.size(), picker.active_id, places.size(), own, ghosts, inside, inward, archers, shown, picker.get_global_rect(), roles, "OK" if ok else "FAIL"])
	if probe or out == "":
		quit(0 if ok else 1)
		return
	scene.camera_rig.edge_pan_enabled = false
	var back := -forward
	scene.camera_rig.look_at_point(Vector3(point.x, 0, point.y) + Vector3(back.x, 0, back.y) * 50.0, 360.0, atan2(back.x, back.y))
	for _i in 30:
		await process_frame
	RenderingServer.force_draw()
	await process_frame
	var image := root.get_viewport().get_texture().get_image()
	if image == null or image.is_empty():
		push_error("cb6_formation_shot: empty viewport image (headless?)")
		quit(1)
		return
	if image.get_width() > 1600:
		image.resize(1600, int(image.get_height() * 1600.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var err := image.save_png(out)
	print("CB6_FORMATION_SHOT %s (%s)" % [out, error_string(err)])
	quit(0 if err == OK and ok else 1)


func _unit(scene: Node, id: int) -> Dictionary:
	for unit in scene.units:
		if int(unit["id"]) == id:
			return unit
	return {}
