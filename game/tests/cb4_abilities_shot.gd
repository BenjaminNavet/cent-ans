extends SceneTree

## Lot CB4 : capture des boutons de capacité sous les cartes d'unité, sur la démo autonome de
## bataille (`battle.tscn`, France-Angleterre, graine 1337). La bataille avance (IA des deux camps)
## jusqu'à ce qu'un régiment du joueur ait une capacité grisée par une condition (au contact,
## plus de traits…) ; puis, bataille en pause, un autre régiment emploie la sienne (bouton actif,
## anneau doré) et un troisième l'emploie puis la lève (bouton en recharge, cadran). Tout passe
## par de vrais ordres `use_ability` ; aucun point posé au sol (pas d'ordre de déplacement).
## Glyphes dessinés en code (icônes DA5 plus tard).
## Usage (avec affichage, pas en headless : le rendu headless ne produit pas d'image) :
##   godot --path game --resolution 1600x900 --script res://tests/cb4_abilities_shot.gd -- --out=<png>
## `--probe` : contrôle texte seulement, sans image (headless possible) : sortie `CB4_ABILITIES_PROBE`
## (boutons actif, en recharge, grisé), code 0 si tout est en place.

const MAX_SECONDS := 400


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
	if scene.battle == null or not scene.battle.has_method("get_ability_catalog"):
		push_error("cb4_abilities_shot: battle demo failed to stage or bridge without CB4 (run core/build.sh?)")
		quit(1)
		return
	scene.paused = true
	# La bataille avance jusqu'à une capacité grisée par une condition (pas par la recharge).
	scene.battle.call("set_ai", scene.player_side, true)
	var seconds := 0
	var greyed := -1
	while seconds < MAX_SECONDS and greyed < 0:
		scene.battle.call("tick", 1.0)
		seconds += 1
		greyed = _greyed(scene, scene.battle.call("get_units"))
	scene.battle.call("set_ai", scene.player_side, false)
	# Deux autres régiments : l'un emploie sa capacité, l'autre l'emploie puis la lève.
	var free: Array[int] = []
	for unit in scene.battle.call("get_units"):
		var abilities := Array(unit.get("abilities", []))
		if str(unit["side"]) == scene.player_side and int(unit["id"]) != greyed and not abilities.is_empty() and bool(abilities[0]["available"]) and not bool(abilities[0]["active"]):
			free.append(int(unit["id"]))
	var active := -1
	var cooling := -1
	if free.size() >= 2:
		active = free[0]
		cooling = free[1]
		var ability := str(Array(_unit(scene, active)["abilities"])[0]["id"])
		scene.issue({"type": "use_ability", "units": [active], "ability": ability})
		ability = str(Array(_unit(scene, cooling)["abilities"])[0]["id"])
		scene.issue({"type": "use_ability", "units": [cooling], "ability": ability})
		scene.issue({"type": "use_ability", "units": [cooling], "ability": ability})
	scene._refresh_view(true)
	var units: Array = scene.battle.call("get_units")
	var selected: Array[int] = []
	for id in [active, cooling, greyed]:
		if id >= 0:
			selected.append(id)
	scene.selected.assign(selected)
	scene.hud.update_cards(units, scene.player_side, scene.selected)
	var shown := {"active": 0, "cooling": 0, "greyed": 0}
	var reasons: PackedStringArray = []
	for id in scene.hud._cards:
		for button: Button in (scene.hud._cards[id] as UnitCard).ability_buttons():
			var state: Dictionary = button.get_meta("state", {})
			if bool(state.get("active", false)):
				shown["active"] += 1
			elif float(state.get("cooldown_remaining", 0.0)) > 0.0:
				shown["cooling"] += 1
			elif button.disabled:
				shown["greyed"] += 1
				reasons.append(str(state.get("reason", "")))
	var ok: bool = int(shown["active"]) > 0 and int(shown["cooling"]) > 0 and int(shown["greyed"]) > 0
	print("CB4_ABILITIES_PROBE seconds=%d active=%d cooling=%d greyed=%d reasons=%s %s" % [seconds, shown["active"], shown["cooling"], shown["greyed"], reasons, "OK" if ok else "FAIL"])
	if probe or out == "":
		quit(0 if ok else 1)
		return
	# Caméra sur les régiments choisis ; les cartes sont dans le bandeau du bas.
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
	scene.camera_rig.look_at_point(center, 260.0, 0.0 if forward > 0.0 else PI)
	for _i in 20:
		await process_frame
	RenderingServer.force_draw()
	await process_frame
	var image := root.get_viewport().get_texture().get_image()
	if image == null or image.is_empty():
		push_error("cb4_abilities_shot: empty viewport image (headless?)")
		quit(1)
		return
	if image.get_width() > 1600:
		image.resize(1600, int(image.get_height() * 1600.0 / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var err := image.save_png(out)
	print("CB4_ABILITIES_SHOT %s (%s)" % [out, error_string(err)])
	quit(0 if err == OK and ok else 1)


## Un régiment du joueur dont une capacité est grisée par une condition (-1 sinon).
func _greyed(scene: Node, units: Array) -> int:
	for unit in units:
		if str(unit["side"]) != scene.player_side or not bool(unit["present"]) or str(unit["state"]) == "routing":
			continue
		for state: Dictionary in Array(unit.get("abilities", [])):
			var reason := str(state.get("reason", ""))
			if not bool(state["available"]) and reason != "" and not reason.begins_with("recharge"):
				return int(unit["id"])
	return -1


func _unit(scene: Node, id: int) -> Dictionary:
	for unit in scene.battle.call("get_units"):
		if int(unit["id"]) == id:
			return unit
	return {}
