extends SceneTree

## CB5 : sonde texte de la colonne d'alertes (pas d'image, règle CLAUDE.md sur les captures).
## Vérifie, sur la démo autonome de bataille (`battle.tscn`), que la colonne (haut à gauche) ne
## chevauche ni le journal (`BattleLog`, haut à droite), ni le bandeau du bas (`BottomBand` :
## sceau du chef, cartes, ordres), ni les « Ordres du chef », le panneau de comparaison (CB-M4,
## forcé visible) et le sélecteur de formations (CB6).
##
## Usage :
##   godot --headless --path game --script res://tests/cb5_alerts_shot.gd -- --probe
##   godot --path game --resolution 1600x900 --script res://tests/cb5_alerts_shot.gd -- --out=<png>

func _init() -> void:
	var probe := false
	var out := ""
	for arg in OS.get_cmdline_user_args():
		if arg == "--probe":
			probe = true
		elif arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
	if not probe and out == "":
		print("cb5_alerts_shot: usage -- --probe | --out=<png>")
		quit(1)
		return
	await process_frame
	root.size = Vector2i(1600, 900)  # headless : la fenêtre par défaut est minuscule (64x64)
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.autoplay = true
	root.add_child(scene)
	for _i in 6:
		await process_frame
	if scene.battle == null:
		push_error("cb5_alerts_shot: battle demo failed to stage (run core/build.sh?)")
		quit(1)
		return
	var hud: CanvasLayer = scene.hud
	# Quelques alertes synthétiques pour que la colonne affiche ses 5 lignes au maximum.
	hud.alerts_column.push_alerts([
		{"kind": "general_down", "time": 0.0, "x": 0.0, "z": 0.0, "side": "attacker", "unit": 1},
		{"kind": "gate_destroyed", "time": 0.0, "x": 100.0, "z": 0.0, "side": "", "unit": -1},
		{"kind": "wall_breached", "time": 0.0, "x": 200.0, "z": 0.0, "side": "", "unit": -1},
		{"kind": "rout", "time": 0.0, "x": 300.0, "z": 0.0, "side": "attacker", "unit": 2},
		{"kind": "flanked", "time": 0.0, "x": 400.0, "z": 0.0, "side": "attacker", "unit": 3},
		{"kind": "reinforcements", "time": 0.0, "x": 500.0, "z": 0.0, "side": "attacker", "unit": 4},
		{"kind": "ammo_out", "time": 0.0, "x": 600.0, "z": 0.0, "side": "attacker", "unit": 5},
	])
	for _i in 2:
		await process_frame
	var column_rect: Rect2 = hud.alerts_column.get_global_rect()
	var report := {
		"column_rect": [column_rect.position.x, column_rect.position.y, column_rect.size.x, column_rect.size.y],
		"rows_shown": hud.alerts_column._box.get_child_count(),
	}
	# Panneau de comparaison CB-M4 forcé visible, avec des valeurs factices, pour mesurer son rectangle.
	if scene.compare_panel != null:
		scene.compare_panel.show_compare({"ours": {"soldiers": 120, "melee": 8.0}, "theirs": {"soldiers": 80, "melee": 14.0}, "advantages": {"soldiers": "ours", "melee": "theirs"}, "lines": ["soldiers", "melee"]}, "Archers", "Hommes d'armes")
	for _i in 2:
		await process_frame
	var others := {
		"journal": hud.root.get_node_or_null("BattleLog"),
		"bottom_band": hud.root.get_node_or_null("BottomBand"),
		"leader_orders": scene._leader_bar.panel if scene._leader_bar != null else null,
		"compare_panel": scene.compare_panel,
		"formation_picker": scene.formation_picker,
	}
	var overlaps: Array = []
	for key in others:
		var other: Control = others[key]
		if other == null or not other.is_visible_in_tree():
			report[key] = "absent"
			continue
		var r := other.get_global_rect()
		report[key] = [r.position.x, r.position.y, r.size.x, r.size.y]
		if column_rect.intersects(r):
			overlaps.append(key)
	report["overlaps"] = overlaps
	report["rows_capped_at_5"] = hud.alerts_column._box.get_child_count() <= 5
	print("CB5_ALERTS_PROBE:" + JSON.stringify(report))
	var ok: bool = overlaps.is_empty() and bool(report["rows_capped_at_5"])
	if out != "":
		await RenderingServer.frame_post_draw
		var image := root.get_texture().get_image()
		if image.get_width() > 1600:
			image.resize(1600, int(image.get_height() * 1600.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
		image.save_png(out)
		print("CB5_ALERTS_SHOT %s (%s)" % [out, "OK" if ok else "FAIL"])
	scene.queue_free()
	quit(0 if ok else 1)
