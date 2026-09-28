extends SceneTree

## Lot CB2 : capture des modes d'unité et des pastilles d'état sur la démo autonome de bataille
## (`battle.tscn`, France-Angleterre, graine 1337). Les régiments du joueur prennent des modes par
## de vrais ordres `set_mode` (garde pour les fantassins, course pour les cavaliers, escarmouche
## pour les tireurs), puis la bataille avance (IA des deux camps) jusqu'à ce que des états du cœur
## apparaissent (sous le feu, hésite, charge) : cartes avec leurs glyphes de mode, pastilles au-dessus
## des bannières, boutons de mode de la barre d'ordres. Glyphes dessinés en code (icônes DA5 plus
## tard). Aucun point posé au sol (pas d'ordre de déplacement).
## Usage (avec affichage, pas en headless : le rendu headless ne produit pas d'image) :
##   godot --path game --resolution 1600x900 --script res://tests/cb2_modes_shot.gd -- --out=<png>
## `--probe` : contrôle texte seulement, sans image (headless possible) : sortie `CB2_MODES_PROBE`
## (modes pris par le cœur, pastilles par régiment), code 0 si tout est en place.

const MAX_SECONDS := 300


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
	var units: Array = scene.battle.call("get_units") if scene.battle != null else []
	if units.is_empty() or not (units[0] as Dictionary).has("modes"):
		push_error("cb2_modes_shot: battle demo failed to stage or bridge without CB2 (run core/build.sh?)")
		quit(1)
		return
	scene.paused = true
	# Modes par de vrais ordres, selon ce que chaque régiment peut prendre (verdict du cœur).
	var by_mode := {"guard": [], "run": [], "skirmish": []}
	for unit in units:
		if not bool(unit["present"]) or str(unit["side"]) != scene.player_side:
			continue
		var modes := Array(unit["modes"])
		var category := str(unit["category"])
		if category == "infantry" and modes.has("guard"):
			by_mode["guard"].append(int(unit["id"]))
		elif category == "cavalry" and modes.has("run"):
			by_mode["run"].append(int(unit["id"]))
		elif modes.has("skirmish"):
			by_mode["skirmish"].append(int(unit["id"]))
	var issued := 0
	for mode in by_mode:
		if not (by_mode[mode] as Array).is_empty():
			var result: Dictionary = scene.issue({"type": "set_mode", "units": by_mode[mode], "mode": mode, "enabled": true})
			issued += 1 if bool(result.get("ok", false)) else 0
	# La bataille avance (IA des deux camps) jusqu'aux premiers états du cœur.
	scene.battle.call("set_ai", scene.player_side, true)
	var seconds := 0
	var states := {}
	while seconds < MAX_SECONDS:
		scene.battle.call("tick", 1.0)
		seconds += 1
		states = _states(scene.battle.call("get_units"))
		if int(states.get("under_fire", 0)) > 0 and int(states.get("wavering", 0)) + int(states.get("charging", 0)) + int(states.get("engaged", 0)) > 0:
			break
	scene.battle.call("set_ai", scene.player_side, false)
	scene._refresh_view(true)
	units = scene.battle.call("get_units")
	# Sélection : les régiments en mode, pour les cartes et les boutons de mode.
	var selected: Array[int] = []
	for mode in by_mode:
		for id in by_mode[mode]:
			selected.append(int(id))
	scene.selected.assign(selected)
	scene.hud.update_cards(units, scene.player_side, scene.selected)
	var badges: Array = []
	var with_modes := 0
	var mode_badges := 0
	var state_badges := 0
	for unit in units:
		if not bool(unit["present"]):
			continue
		var list := BattleUnitMarkers.state_badges(unit)
		if not list.is_empty():
			badges.append("%d:%s" % [int(unit["id"]), ",".join(list)])
		for badge in list:
			if str(badge).begins_with("mode_"):
				mode_badges += 1
			elif badge in ["wavering", "under_fire", "charge", "melee"]:
				state_badges += 1
		if str(unit["side"]) == scene.player_side and not BattleModeIcons.active_modes(unit).is_empty():
			with_modes += 1
	var buttons: Dictionary = scene.hud._mode_buttons
	var ok := issued >= 2 and with_modes >= 3 and mode_badges > 0 and state_badges > 0 and buttons.size() == BattleModeIcons.MODES.size()
	print("CB2_MODES_PROBE orders=%d with_modes=%d seconds=%d states=%s badges=%s buttons=%d %s" % [issued, with_modes, seconds, states, badges, buttons.size(), "OK" if ok else "FAIL"])
	if probe or out == "":
		quit(0 if ok else 1)
		return
	# Caméra sur les régiments du joueur en mode.
	var center := Vector3.ZERO
	var count := 0
	for unit in units:
		if selected.has(int(unit["id"])) and bool(unit["present"]):
			center += Vector3(float(unit["x"]), 0, float(unit["z"]))
			count += 1
	if count > 0:
		center /= count
	var forward := 1.0 if scene.player_side == "attacker" else -1.0
	scene.camera_rig.edge_pan_enabled = false
	scene.camera_rig.look_at_point(center + Vector3(0, 0, forward * 40.0), 260.0, 0.0 if forward > 0.0 else PI)
	for _i in 20:
		await process_frame
	RenderingServer.force_draw()
	await process_frame
	var image := root.get_viewport().get_texture().get_image()
	if image == null or image.is_empty():
		push_error("cb2_modes_shot: empty viewport image (headless?)")
		quit(1)
		return
	if image.get_width() > 1600:
		image.resize(1600, int(image.get_height() * 1600.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var err := image.save_png(out)
	print("CB2_MODES_SHOT %s (%s)" % [out, error_string(err)])
	quit(0 if err == OK and ok else 1)


## Décompte des états du cœur parmi les régiments présents.
func _states(units: Array) -> Dictionary:
	var out := {}
	for unit in units:
		if not bool(unit["present"]):
			continue
		for key in ["under_fire", "wavering", "charging", "engaged"]:
			if bool(unit.get(key, false)):
				out[key] = int(out.get(key, 0)) + 1
	return out
