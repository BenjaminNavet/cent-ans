extends SceneTree

## CB5 : sonde texte de la colonne d'alertes (pas d'image, règle CLAUDE.md sur les captures).
## Vérifie, sur la démo autonome de bataille (`battle.tscn`), que la colonne (haut à gauche) ne
## chevauche ni le journal (`BattleLog`, haut à droite), ni le bandeau du bas (`BottomBand` :
## sceau du chef, cartes, ordres). Le panneau de comparaison au survol (CB-M4) n'est pas encore
## fusionné dans cette branche : non vérifié ici, à refaire une fois CB-M4 dans `main`.
##
## Usage (probe seulement, pas d'image) :
##   godot --headless --path game --script res://tests/cb5_alerts_shot.gd -- --probe

func _init() -> void:
	var probe := false
	for arg in OS.get_cmdline_user_args():
		if arg == "--probe":
			probe = true
	if not probe:
		print("cb5_alerts_shot: usage -- --probe (sonde texte seulement, pas d'image)")
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
	var journal: Control = hud.root.get_node_or_null("BattleLog")
	if journal != null:
		var jr := journal.get_global_rect()
		report["journal_rect"] = [jr.position.x, jr.position.y, jr.size.x, jr.size.y]
		report["overlaps_journal"] = column_rect.intersects(jr)
	var bottom: Control = hud.root.get_node_or_null("BottomBand")
	if bottom != null:
		var br := bottom.get_global_rect()
		report["bottom_band_rect"] = [br.position.x, br.position.y, br.size.x, br.size.y]
		report["overlaps_bottom_band"] = column_rect.intersects(br)
	report["rows_capped_at_5"] = hud.alerts_column._box.get_child_count() <= 5
	report["note"] = "CB-M4 hover comparison panel not merged in this branch: not checked here."
	print("CB5_ALERTS_PROBE:" + JSON.stringify(report))
	var ok: bool = not bool(report.get("overlaps_journal", false)) and not bool(report.get("overlaps_bottom_band", false)) and bool(report["rows_capped_at_5"])
	scene.queue_free()
	quit(0 if ok else 1)
