extends SceneTree

## TW2 : captures de vérification visuelle du chantier TW2 (barres de siège SB, points de
## capture T4, fenêtre de la place prise T1, panneau des mercenaires T3, panneau des traditions
## T5). Chaque image force l'état voulu (via les fonctions `debug_*` déjà utilisées par les tests
## du chantier) plutôt que d'attendre un tirage aléatoire.
## Usage (avec affichage) :
##   godot --path game --resolution 1280x720 --script res://tests/tw2_shot.gd -- \
##     --out=<dossier> [--prefix=tw2] \
##     [--only=siege_bars,capture_flags,sack_dialog,mercenaries,traditions]

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")

var _out := ""
var _prefix := "tw2"
var _only: PackedStringArray = []


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
		elif arg.begins_with("--prefix="):
			_prefix = arg.trim_prefix("--prefix=")
		elif arg.begins_with("--only="):
			_only = arg.trim_prefix("--only=").split(",", false)
	if _out == "":
		push_error("tw2_shot: --out=<dossier> required")
		quit(1)
		return
	DirAccess.make_dir_recursive_absolute(_out)
	await process_frame
	if _wants("siege_bars") or _wants("capture_flags"):
		await _siege_shots()
	if _wants("sack_dialog") or _wants("mercenaries") or _wants("traditions"):
		await _campaign_shots()
	print("tw2_shot: done")
	quit(0)


func _wants(key: String) -> bool:
	return _only.is_empty() or _only.has(key)


# --- Bataille de siège : barres de vie (SB) et points de capture (T4) -----------------------


func _siege_shots() -> void:
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	var sim: Object = ClassDB.instantiate("CampaignSim")
	if not sim.call("new_campaign", data_dir, "fac_france", 1337):
		push_error("tw2_shot: new_campaign failed")
		return
	var armies: Array = BattleScene.main_armies(sim, "fac_france", "fac_england")
	var index: int = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	if index < 0:
		push_error("tw2_shot: cannot stage the siege")
		return
	BattleScene.demo_args = PackedStringArray(["--siege-engines=unit_trebuchet", "--no-speech"])
	var scene: Node = (load("res://scenes/battle/battle.tscn") as PackedScene).instantiate()
	scene.configure(sim, index, 7)
	root.add_child(scene)
	for _i in 30:
		await process_frame
	var battle: Object = scene.battle
	if scene.deployment != null and scene.deployment.active:
		scene.deployment.finish()
	if battle.call("is_deploying"):
		battle.call("start_battle")
	battle.call("set_ai", "attacker", true)
	battle.call("set_ai", "defender", true)
	scene.camera_rig.edge_pan_enabled = false
	var view: BattleSiege = scene.siege_view
	if view == null or view.health_bars == null or view.capture_points == null:
		push_error("tw2_shot: siege view has no health bars / capture points")
		scene.queue_free()
		await process_frame
		return
	if _wants("siege_bars"):
		await _shot_siege_bars(scene, battle, view)
	if _wants("capture_flags"):
		await _shot_capture_flags(scene, battle, view)
	scene.queue_free()
	await process_frame


## Cadre un mur/porte endommagé (bar forcée via `debug_set_piece_hp`) avec un engin proche
## (le bélier, toujours présent à l'assaut, avance naturellement vers la porte).
func _shot_siege_bars(scene: Node, battle: Object, view: BattleSiege) -> void:
	var siege: Dictionary = battle.call("get_siege")
	var gate := int(siege["gate"])
	var max_hp := float((siege["pieces"] as Array)[gate]["max_hp"])
	battle.call("debug_set_piece_hp", gate, max_hp * 0.5)
	# Laisse le bélier approcher de la porte pendant quelques dizaines de secondes de simulation.
	var limit := float(battle.call("get_elapsed")) + 200.0
	var ram := {}
	while float(battle.call("get_elapsed")) < limit and not battle.call("is_finished"):
		scene._fast_forward(float(battle.call("get_elapsed")) + 0.5)
		for unit in battle.call("get_units"):
			if str(unit.get("render", "")) == "ram" and bool(unit["present"]):
				var pos := Vector2(float(unit["x"]), float(unit["z"]))
				siege = battle.call("get_siege")
				var gate_piece: Dictionary = (siege["pieces"] as Array)[gate]
				var mid: Vector2 = (Vector2(gate_piece["a"]) + Vector2(gate_piece["b"])) * 0.5
				if pos.distance_to(mid) < 40.0:
					ram = unit
		if not ram.is_empty():
			break
	siege = battle.call("get_siege")
	battle.call("debug_set_piece_hp", gate, max_hp * 0.5)
	await process_frame
	var focus := _gate_point(siege, 6.0)
	scene.paused = true
	scene.camera_rig.look_at_point(focus, 30.0, _yaw_out(siege, gate) + 0.9)
	await _frames(6)
	var state := view.health_bars.state("piece:%d" % gate)
	print("tw2_shot: siege_bars gate shown=%s ratio=%.2f ram_present=%s" % [state.get("shown", false), float(state.get("ratio", 0.0)), not ram.is_empty()])
	await _shot("%s_siege_bars.png" % _prefix)
	scene.paused = false


## Cadre la place avec les drapeaux de capture ; tente une vraie prise en cours (jusqu'à un
## budget de temps de simulation), sinon capture les drapeaux au repos.
func _shot_capture_flags(scene: Node, battle: Object, view: BattleSiege) -> void:
	var siege: Dictionary = battle.call("get_siege")
	var center: Vector2 = siege.get("center", Vector2(600, 560))
	var limit := float(battle.call("get_elapsed")) + 2000.0
	var contested := false
	while float(battle.call("get_elapsed")) < limit and not battle.call("is_finished"):
		scene._fast_forward(float(battle.call("get_elapsed")) + 1.0)
		siege = battle.call("get_siege")
		for point in siege.get("points", []):
			if str(point.get("status", "")) == "capturing":
				contested = true
		if contested:
			break
	await process_frame
	scene.paused = true
	scene.camera_rig.look_at_point(Vector3(center.x, 0.0, center.y - 20.0), 90.0, 0.4)
	await _frames(6)
	var flags := 0
	for point in siege.get("points", []):
		var key := str(point.get("kind", ""))
		if view.capture_points.flag(key) != null:
			flags += 1
	print("tw2_shot: capture_flags contested=%s flags_shown=%d elapsed=%.0f" % [contested, flags, float(battle.call("get_elapsed"))])
	await _shot("%s_capture_flags.png" % _prefix)
	scene.paused = false


func _gate_point(siege: Dictionary, offset: float) -> Vector3:
	var gate: Dictionary = siege["pieces"][int(siege["gate"])]
	var a: Vector2 = gate["a"]
	var b: Vector2 = gate["b"]
	var mid := (a + b) * 0.5
	var out := _out_dir(siege, gate)
	return Vector3(mid.x + out.x * offset, 0.0, mid.y + out.y * offset)


func _out_dir(siege: Dictionary, piece: Dictionary) -> Vector2:
	var a: Vector2 = piece["a"]
	var b: Vector2 = piece["b"]
	var d := (b - a).normalized()
	var out := Vector2(d.y, -d.x)
	if out.dot((a + b) * 0.5 - (siege.get("center", Vector2(600, 560)) as Vector2)) < 0.0:
		out = -out
	return out


func _yaw_out(siege: Dictionary, piece: int) -> float:
	var out := _out_dir(siege, siege["pieces"][piece])
	return atan2(out.x, out.y)


func _frames(n: int) -> void:
	for _i in n:
		await process_frame


# --- Campagne : place prise (T1), mercenaires (T3), traditions (T5) -------------------------


func _campaign_shots() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not (map.load_ok and map.sim != null and facade.is_real):
		push_error("tw2_shot: campaign map with the real simulation failed to start")
		map.queue_free()
		return
	var sim: Object = map.sim
	if sim.has_method("set_chronicle_enabled"):
		sim.call("set_chronicle_enabled", false)
	if _wants("sack_dialog"):
		await _shot_sack_dialog(map, sim)
	if _wants("mercenaries"):
		await _shot_mercenaries(map, sim)
	if _wants("traditions"):
		await _shot_traditions(map, sim)
	map.queue_free()
	await process_frame


## Fenêtre du sort de la ville prise (ChronicleWindow), ouverte d'office après une prise forcée.
func _shot_sack_dialog(map: Node, sim: Object) -> void:
	if not sim.has_method("get_pending_captures"):
		push_error("tw2_shot: CampaignSim.get_pending_captures missing (run core/build.sh)")
		return
	var controller: CaptureController = map.get("capture_fate")
	if controller == null:
		push_error("tw2_shot: campaign map has no CaptureController")
		return
	if not sim.call("debug_capture_place", "prov_guyenne"):
		push_error("tw2_shot: debug capture of Guyenne refused")
		return
	map.refresh_all()
	await process_frame
	await process_frame
	var window: ChronicleWindow = controller.window
	if window == null or not window.visible:
		push_error("tw2_shot: capture window did not open")
		return
	print("tw2_shot: sack_dialog window visible, decision=%d" % window.current_decision())
	await _shot("%s_sack_dialog.png" % _prefix)


## Panneau des mercenaires ouvert depuis le bandeau d'une armée du joueur.
func _shot_mercenaries(map: Node, sim: Object) -> void:
	if not sim.has_method("get_mercenaries"):
		push_error("tw2_shot: CampaignSim.get_mercenaries missing (run core/build.sh)")
		return
	var army_id := ""
	for id in map.player_army_ids():
		if _hireable(sim.call("get_mercenaries", id)) != "":
			army_id = id
			break
	if army_id == "":
		print("tw2_shot: mercenaries — no French army stands where a company can be hired in 1337")
		return
	map.select_army(army_id)
	await process_frame
	var strip: Node = map.ui.army_strip
	var button: Button = strip.find_child("MercenaryButton", true, false)
	if button == null or not button.visible:
		push_error("tw2_shot: army strip has no « Mercenaires » button")
		return
	button.pressed.emit()
	await process_frame
	var panel: Control = map.hud.mercenary_panel
	if panel == null or not panel.visible:
		push_error("tw2_shot: mercenary panel did not open")
		return
	var rows := panel.find_children("PoolLabel", "Label", true, false)
	print("tw2_shot: mercenaries panel visible, %d companies" % rows.size())
	await _shot("%s_mercenaries.png" % _prefix)
	button.pressed.emit()
	await process_frame


func _hireable(info: Dictionary) -> String:
	for option: Dictionary in info.get("options", []):
		if bool(option.get("available", false)):
			return str(option.get("unit_type", ""))
	return ""


## Panneau des traditions ouvert après avoir amené une armée au rang 1 (`debug_grant_army_xp`).
func _shot_traditions(map: Node, sim: Object) -> void:
	if not sim.has_method("get_army_traditions"):
		push_error("tw2_shot: CampaignSim.get_army_traditions missing (run core/build.sh)")
		return
	var army_id := ""
	for id in map.player_army_ids():
		var army: Dictionary = sim.call("get_army", id)
		if str(army.get("general", "")) != "":
			army_id = id
			break
	if army_id == "":
		print("tw2_shot: traditions — no French army led by a general")
		return
	var controller: TraditionsController = map.get("traditions")
	if controller == null:
		push_error("tw2_shot: campaign map has no traditions controller")
		return
	var first := int(sim.call("get_army_traditions", army_id).get("next_threshold", 0))
	if not sim.call("debug_grant_army_xp", army_id, first):
		push_error("tw2_shot: debug_grant_army_xp refused")
		return
	map.refresh_all()
	await process_frame
	map.select_army(army_id)
	await process_frame
	controller.button.pressed.emit()
	await process_frame
	if not (controller.panel.visible and controller.army_id == army_id):
		push_error("tw2_shot: traditions panel did not open")
		return
	var rows := controller.panel.find_child("Options", true, false)
	var count := rows.get_child_count() if rows != null else 0
	print("tw2_shot: traditions panel visible, %d branches" % count)
	await _shot("%s_traditions.png" % _prefix)


func _shot(file_name: String) -> void:
	await RenderingServer.frame_post_draw
	var img := root.get_viewport().get_texture().get_image()
	var path := _out.path_join(file_name)
	var err := img.save_png(path)
	print("tw2_shot: %s (%s)" % [path, error_string(err)])
