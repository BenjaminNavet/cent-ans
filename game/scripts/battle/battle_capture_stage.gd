class_name BattleCaptureStage
extends RefCounted

## Captures d'écran et mises en scène de la bataille (SC BT12) : `--screenshot=<png>` joue la
## bataille jusqu'au contact (ou `--shot-at`, `--closeup`, `--standard-shot`, `--result-shot`,
## `--deploy-shot`), cadre la caméra (`--camera=`), enregistre l'image puis quitte. Sert à
## `game/tests/readme_gallery.gd` et aux bancs d'essai. Extrait de `BattleScene`, qui lui passe
## les arguments de la ligne de commande et lui laisse le champ `screenshot_path`.

var screenshot_path: String = ""
var result_shot: bool = false
var deploy_shot: bool = false
var closeup: bool = false
var closeup_distance: float = 26.0  # FG5 : `--closeup-distance=<m>` (captures du LOD0 par soldat)
var shot_at: float = -1.0  # B4 : `--shot-at=<s>`
var standard_side: String = ""  # DA1b : `--standard-side=` (camp cadré par `--standard-shot`)
var standard_shot: String = ""  # EP5 : `--standard-shot=<foot|mounted|line|fallen|captured>`
var camera_override: String = ""

var _scene: BattleScene = null


func _init(scene: BattleScene) -> void:
	_scene = scene


## Lit un argument de capture ; faux s'il n'en est pas un.
func parse_arg(arg: String) -> bool:
	if arg.begins_with("--screenshot="):
		screenshot_path = arg.trim_prefix("--screenshot=")
	elif arg == "--deploy-shot":
		deploy_shot = true
	elif arg == "--result-shot":
		result_shot = true
	elif arg.begins_with("--shot-at="):
		shot_at = float(arg.trim_prefix("--shot-at="))
	elif arg.begins_with("--standard-shot="):
		standard_shot = arg.trim_prefix("--standard-shot=")
	elif arg.begins_with("--standard-side="):
		standard_side = arg.trim_prefix("--standard-side=")
	elif arg.begins_with("--camera="):
		camera_override = arg.trim_prefix("--camera=")
	elif arg == "--closeup":
		closeup = true
	elif arg.begins_with("--closeup-distance="):
		closeup_distance = float(arg.trim_prefix("--closeup-distance="))
	else:
		return false
	return true


## Capture : IA des deux camps jusqu'au premier contact (+ 12 s), sélection de deux régiments
## du joueur, caméra sur la mêlée, capture après quelques images.
func run() -> void:
	if _scene.battle == null:
		_scene.get_tree().quit(1)
		return
	_scene.camera_rig.edge_pan_enabled = false
	if deploy_shot:
		await run_deploy()
		return
	if result_shot:
		await run_result()
		return
	var contact_time := -1.0
	# EP7 : sur une carte historique, les batailles françaises montent longtemps avant le choc.
	for _i in (9000 if not _scene.historical.is_empty() else 3000):
		_scene.battle.call("tick", 0.1)
		# Les soldats tombés pendant l'avance rapide laissent aussi leurs cadavres.
		_scene.units = _scene.battle.call("get_units")
		_scene.soldiers.update(_scene.battle, _scene.units, 0.1, [])
		_scene._update_effects(0.1)
		if _scene.staging != null:
			_scene.staging.update(_scene.units, 0.1, 0.1, bool(_scene.battle.call("is_finished")))
		if standard_shot == "fallen" or standard_shot == "captured":
			# EP5 : dès qu'un étendard gît depuis 2 s (le porte-étendard a fini de tomber).
			if _scene.standards != null:
				_scene.standards.update(_scene.units, _scene.soldiers, _scene._camera_position())
			var down := false
			for unit in _scene.units:
				if standard_shot == "fallen" and str(unit.get("standard", "")) == "fallen" and float(unit.get("standard_timer", 99.0)) < 3.0:
					down = true
				elif standard_shot == "captured" and int(unit.get("standard_by", -1)) >= 0:
					down = true
			if down:
				break
			continue
		if shot_at > 0.0:
			if float(_scene.battle.call("get_elapsed")) >= shot_at:
				break
			continue
		if contact_time < 0.0:
			for unit in _scene.battle.call("get_units"):
				if str(unit["state"]) == "melee":
					contact_time = float(_scene.battle.call("get_elapsed"))
					break
		elif _scene.battle.call("is_finished"):
			break
		elif closeup:
			# A1-06 : cliché au choc (1 s après le contact), ou dès que la mêlée cesse (une charge
			# met souvent l'adversaire en déroute en quelques secondes).
			var since := float(_scene.battle.call("get_elapsed")) - contact_time
			if since >= 1.0 or not _melee_ongoing(_scene.units):
				break
		elif float(_scene.battle.call("get_elapsed")) > contact_time + 12.0:
			break
	_scene.paused = true
	print("BattleScene: capture at %.0f s, %d corpses, %d missiles" % [float(_scene.battle.call("get_elapsed")), _scene.soldiers.corpse_count, _scene.effects.launched if _scene.effects != null else 0])
	if _scene.effects != null and _scene.blood != null:
		print("BattleScene: BV1 %d volley arrows, %d stuck, %d blood decals (level %d), last at %s" % [_scene.effects.volleys.launched, _scene.effects.volleys.stuck_count, _scene.blood.decal_count, _scene.blood.level, _scene.blood.last_pos])
	_scene.units = _scene.battle.call("get_units")
	if _scene.grass_flatten != null:
		print("BattleScene: BV3 %d corpses marked on the grass, last at %s" % [_scene.grass_flatten.corpse_marks, _scene.grass_flatten.last_corpse])
		for unit in _scene.units:
			if str(unit["state"]) == "melee":
				print("BattleScene: BV3 melee at (%.0f, %.0f), grass flattened %.2f, blood %.2f" % [float(unit["x"]), float(unit["z"]), _scene.grass_flatten.flatten_at(float(unit["x"]), float(unit["z"])), _scene.grass_flatten.blood_at(float(unit["x"]), float(unit["z"]))])
				break
	var focus := Vector3.ZERO
	var n := 0
	for unit in _scene.units:
		if str(unit["state"]) == "melee":
			focus += Vector3(float(unit["x"]), 0, float(unit["z"]))
			n += 1
	if closeup:
		var shot := closeup_shot(_scene.units)
		_scene.camera_rig.look_at_point(shot["focus"], closeup_distance, float(shot["yaw"]))
	elif n > 0:
		focus /= n
		_scene.camera_rig.look_at_point(focus + Vector3(0, 0, -25 if _scene.player_side == "attacker" else 25), 120.0, (PI if _scene.player_side == "attacker" else 0.0) + 0.5)
	for unit in _scene.units:
		if str(unit["side"]) == _scene.player_side and bool(unit["present"]) and _scene.selected.size() < 2:
			_scene.selected.append(int(unit["id"]))
	if _scene.markers != null and not _scene.selected.is_empty() and not closeup:
		_scene.markers.world_hover = _scene.selected[0]  # B2 : la capture montre aussi le nom au survol
	_scene._refresh_view(true)
	apply_camera_override()
	_apply_standard_shot()
	# B4 : laisser la poussière se lever (les particules vivent en temps réel, bataille en pause).
	for _i in 150 if _scene.effects != null else 40:
		await _scene.get_tree().process_frame
	take_screenshot(screenshot_path, true)


## Capture `--deploy-shot` : phase de déploiement ouverte, deux régiments du joueur rangés en
## ligne dans la zone, un placement refusé (toast), vue plongeante sur la zone.
func run_deploy() -> void:
	if _scene.deployment == null:
		_scene.get_tree().quit(1)
		return
	var zone: Dictionary = _scene.deployment.zone
	var center := Vector3((float(zone["x0"]) + float(zone["x1"])) * 0.5, 0, (float(zone["z0"]) + float(zone["z1"])) * 0.5)
	for unit in _scene.units:
		if str(unit["side"]) == _scene.player_side and bool(unit["present"]) and _scene.selected.size() < 2:
			_scene.selected.append(int(unit["id"]))
	print("BattleScene: deployment zone %s, player side %s" % [zone, _scene.player_side])
	var ahead := 1.0 if _scene.player_side == "attacker" else -1.0
	var cam := center + Vector3(0, 300, -400 * ahead)
	_scene.deployment.place(_scene.selected.duplicate(), center + Vector3(-60, 0, 0), center + Vector3(60, 0, 0), cam)
	var outside := Vector3(center.x, 0, (float(zone["z1"]) + 150.0) if ahead > 0.0 else (float(zone["z0"]) - 150.0))
	_scene.deployment.place(_scene.selected.slice(0, 1), outside, outside, cam)
	_scene.camera_rig.look_at_point(center + Vector3(0, 0, 60 * ahead), 420.0, (PI if ahead > 0.0 else 0.0) + 0.35)
	_scene._refresh_view(true)
	apply_camera_override()
	for _i in 40:
		await _scene.get_tree().process_frame
	take_screenshot(screenshot_path, true)


## Capture `--result-shot` : bataille jouée par l'IA jusqu'au bout, écran de fin affiché.
func run_result() -> void:
	for _i in 36000:
		_scene.battle.call("tick", 0.1)
		if _scene.battle.call("is_finished"):
			break
	_scene.soldiers.update(_scene.battle, _scene.battle.call("get_units"), 0.1, [])
	_scene._refresh_view(true)
	if not _scene.finished_shown:
		_scene._show_end()
	for _i in 30:
		await _scene.get_tree().process_frame
	take_screenshot(screenshot_path, true)


## Un régiment au moins est au corps à corps (capture `--closeup`).
func _melee_ongoing(p_units: Array) -> bool:
	for unit in p_units:
		if bool(unit["present"]) and str(unit["state"]) == "melee":
			return true
	return false


## Gros plan `--closeup` (A1-06) : cadre le point de contact réel de la mêlée — les deux soldats
## ennemis les plus proches parmi les couples de régiments au corps à corps (à défaut, les plus
## proches tout court) —, vu de trois quarts, perpendiculairement à la ligne de front, depuis le
## camp du joueur. Avant : milieu décalé vers le régiment du joueur (souvent la cavalerie restée
## en arrière), sans ennemi dans le cadre.
func closeup_shot(p_units: Array) -> Dictionary:
	var best := INF
	var focus := Vector3.ZERO
	var yaw := 0.0
	for unit in p_units:
		if str(unit["side"]) != _scene.player_side or not bool(unit["present"]):
			continue
		for other in p_units:
			if str(other["side"]) == _scene.player_side or not bool(other["present"]):
				continue
			var a := Vector2(float(unit["x"]), float(unit["z"]))
			var b := Vector2(float(other["x"]), float(other["z"]))
			if a.distance_to(b) > 150.0:
				continue
			var in_melee := ["melee", "charging"].has(str(unit["state"])) or str(other["state"]) == "melee"
			var ours := _scene.soldiers.soldier_positions(int(unit["id"]), 48)
			var theirs := _scene.soldiers.soldier_positions(int(other["id"]), 48)
			var pa := Vector3(a.x, 0, a.y)
			var pb := Vector3(b.x, 0, b.y)
			var gap := a.distance_to(b)
			for s1 in ours:
				for s2 in theirs:
					var d := Vector2(s1.x, s1.z).distance_to(Vector2(s2.x, s2.z))
					if d < gap:
						gap = d
						pa = s1
						pb = s2
			var score := gap - (1000.0 if in_melee else 0.0)
			if score < best:
				best = score
				focus = (pa + pb) * 0.5
				# Axe du front : de l'ennemi vers le joueur (centres des régiments) ; caméra du
				# côté du joueur, décalée de ~60° pour voir les deux lignes de profil.
				yaw = atan2(a.x - b.x, a.y - b.y) + 1.05
	focus.y = 0.0
	return {"focus": focus, "yaw": yaw}


## Capture EP5 (`--standard-shot=`) : cadre un porte-étendard (à pied, à cheval), la ligne de
## bataille et ses étendards au loin, ou un étendard tombé.
func _apply_standard_shot() -> void:
	if standard_shot == "" or _scene.standards == null:
		return
	_scene.selected.clear()
	var best: Dictionary = {}
	for unit in _scene.units:
		if not bool(unit["present"]) or not unit.has("bearer_slots"):
			continue
		var render := str(unit["render"])
		var ok := false
		match standard_shot:
			"foot":
				ok = render == "infantry" and str(unit.get("standard", "")) == "carried"
			"mounted":
				ok = render == "cavalry" and str(unit.get("standard", "")) == "carried"
			"fallen":
				ok = str(unit.get("standard", "")) in ["fallen", "lost"]
			"line":
				ok = str(unit["side"]) == _scene.player_side
			"captured":
				for other in _scene.units:
					if int(other.get("standard_by", -1)) == int(unit["id"]):
						ok = true
		if ok and (best.is_empty() or _standard_shot_score(unit) > _standard_shot_score(best)):
			best = unit
	if best.is_empty():
		push_warning("BattleScene: no regiment for --standard-shot=%s" % standard_shot)
		return
	var facing := float(best["facing"])
	var ahead := Vector3(sin(facing), 0, cos(facing))
	if standard_shot == "line":
		# Derrière la ligne du joueur, haut et loin : les étendards ennemis à 300-600 m.
		_scene.camera_rig.look_at_point(Vector3(float(best["x"]), 0, float(best["z"])) + ahead * 25.0, 45.0, atan2(-ahead.x, -ahead.z) + 0.35)
		print("BattleScene: EP5 line shot, %d standards shown, %d figures" % [_scene.standards.shown_count, _scene.standards.figure_count])
		return
	var point := Vector3(float(best.get("standard_x", best["x"])), 0, float(best.get("standard_z", best["z"])))
	if standard_shot != "fallen":
		var frame: Variant = _scene.soldiers.figure_at(int(best["id"]), int((best["bearer_slots"] as PackedInt32Array)[0]))
		if frame != null:
			point = (frame as Transform3D).origin
	# De trois quarts, devant le porte-étendard.
	var yaw := atan2(ahead.x, ahead.z) + 0.6
	_scene.camera_rig.look_at_point(point, 8.0 if standard_shot == "foot" else 12.0, yaw)
	if standard_shot == "fallen" and _scene.camera_rig.camera != null:
		# Vue plongeante : l'étendard gît dans la mêlée, caché par les hommes debout.
		_scene.camera_rig.set_process(false)
		var ground := _scene.terrain.height_at(point.x, point.z)
		var at := Vector3(point.x, ground, point.z)
		_scene.camera_rig.camera.global_position = at + Vector3(sin(yaw), 0, cos(yaw)) * 6.0 + Vector3(0, 7.5, 0)
		_scene.camera_rig.camera.look_at(at, Vector3.UP)
	print("BattleScene: EP5 %s shot on %s (%s) at %s, standard %s, %d standards, %d figures, %d fallen" % [standard_shot, str(best["name"]), str(best["type"]), point, str(best.get("standard", "")), _scene.standards.shown_count, _scene.standards.figure_count, _scene.standards.fallen_count])


## Préférence de `--standard-shot` : le camp voulu d'abord, puis le général et sa retenue noble
## (DA1b : leurs étendards portent les armes de la maison du général).
func _standard_shot_score(unit: Dictionary) -> int:
	var wanted := standard_side if standard_side != "" else _scene.player_side
	var score := 4 if str(unit["side"]) == wanted else 0
	if bool(unit.get("is_general", false)):
		score += 2
	elif BattleStandards.is_house_retinue(unit):
		score += 1
	return score


## Capture : `--camera=x,z,distance,lacet_en_degrés` place la caméra (réglage du rendu).
func apply_camera_override() -> void:
	if camera_override == "":
		return
	var parts := camera_override.split(",")
	if parts.size() < 4:
		return
	var cam_z := float(parts[1])
	if parts[1] == "river":
		cam_z = _scene.terrain.river_center_z(float(parts[0]))  # VN : cadrage sur la rivière (captures)
	_scene.camera_rig.look_at_point(Vector3(float(parts[0]), 0, cam_z), float(parts[2]), deg_to_rad(float(parts[3])))


func take_screenshot(path: String, quit_after: bool) -> void:
	if CmdArgs.has("--no-hud"):
		# EP2 : captures de décor sans interface.
		for layer in _scene.find_children("*", "CanvasLayer", true, false):
			(layer as CanvasLayer).visible = false
		for control in _scene.find_children("*", "Control", true, false):
			if not (control.get_parent() is Control):
				(control as Control).visible = false
		# CR1 : l'interface 3D aussi (contours, trajets, arcs de tir), sinon elle fuit dans les
		# captures « sans interface ».
		for overlay: Node3D in [_scene.outlines, _scene.path_preview, _scene.range_arc]:
			if overlay != null:
				overlay.visible = false
		await _scene.get_tree().process_frame
	await RenderingServer.frame_post_draw
	var image := _scene.get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var err := image.save_png(path)
	print("BattleScene: screenshot %s (%s)" % [path, error_string(err)])
	if quit_after:
		_scene.get_tree().quit(0 if err == OK else 1)
