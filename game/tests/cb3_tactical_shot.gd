extends SceneTree

## Lot CB3 : capture de la vue tactique (Tab) sur la démo autonome de bataille (France-Angleterre,
## graine 1337) — caméra du dessus, terrain assombri, bannières en pastilles B7, ennemis non
## repérés masqués.
## Usage (avec affichage, pas en headless : le rendu headless ne produit pas d'image) :
##   godot --path game --resolution 1600x900 --script res://tests/cb3_tactical_shot.gd -- --out=<png>
## `--probe=<dossier>` : mesure sans lecture humaine de l'image (utilisé par les agents, texte
## seulement) : écrit `probe-tactical.json` (inclinaison, distance, alpha du calque, repères
## visibles / masqués) et n'écrit une image que si `--out` est aussi donné.


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
		push_error("cb3_tactical_shot: battle demo failed to stage (run core/build.sh?)")
		quit(1)
		return
	scene.battle.call("set_ai", scene.player_side, false)
	scene.paused = true
	var tactical: BattleTacticalView = scene.tactical_view
	if tactical == null:
		push_error("cb3_tactical_shot: no tactical view node")
		quit(1)
		return
	tactical.enter()
	scene._update_markers(1.0)
	for _i in 2:
		await process_frame

	if probe != "":
		var cam: BattleCamera = scene.camera_rig
		var units: Array = scene.units
		var hidden := 0
		var shown := 0
		for unit in units:
			if str(unit["side"]) != scene.player_side:
				if bool(unit.get("spotted", true)):
					shown += 1
				else:
					hidden += 1
		var report := {
			"manual_pitch": cam.manual_pitch,
			"manual_pitch_deg": cam.manual_pitch_deg,
			"distance": cam.distance,
			"yaw": cam.yaw,
			"markers_clustered": scene.markers.clustered if scene.markers != null else false,
			"enemies_shown": shown,
			"enemies_hidden": hidden,
		}
		if not DirAccess.dir_exists_absolute(probe):
			DirAccess.make_dir_recursive_absolute(probe)
		var f := FileAccess.open(probe.path_join("probe-tactical.json"), FileAccess.WRITE)
		f.store_string(JSON.stringify(report, "  "))
		f.close()
		print("cb3_tactical_shot: probe written to %s" % probe.path_join("probe-tactical.json"))
		print("cb3_tactical_shot: %s" % JSON.stringify(report))

	if out != "":
		var img := root.get_texture().get_image()
		img.save_png(out)
		print("cb3_tactical_shot: image written to %s" % out)

	quit(0)
