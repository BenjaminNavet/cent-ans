class_name NavalScene
extends Node3D

## Bataille navale en temps réel (lot NV1, ADR 0028). Rendu, interface et entrées seulement :
## toutes les règles (vent, tir des châteaux, grappins, abordage, feu, brûlots, éperon, prise,
## naufrage) vivent dans `NavalBattleSim` (cœur Rust). La scène lit l'état à chaque image
## (`get_ships`, `get_shots`, `get_events`) et transmet les ordres du joueur (`issue_command`).
##
## Deux entrées : une bataille de la campagne (`configure(campaign_sim, index, seed)`, résultat
## rendu par `resolve_naval_battle` au retour) ou un scénario historique autonome
## (`--naval-scenario=sluys`). Options de ligne de commande (après `--`) : `--screenshot=<png>`
## (capture puis quitte), `--shot-at=<s>` (avance rapide jusqu'à cet instant), `--camera=<vue>`
## (overview, close, deck, melee), `--autoplay` (IA des deux camps), `--benchmark` (i/s, JSON),
## `--seed=<n>`.

signal returned(result: Dictionary)

const BENCH_FRAMES := 600
const DOUBLE_CLICK_MS := 350
const SPEEDS := [1.0, 2.0, 4.0]
const SHORE_X := 520.0

var campaign_sim: Object = null
var battle_index: int = -1
var battle_seed: int = 1340
var battle: Object = null  # NavalBattleSim
var setup_data: Dictionary = {}
var scenario_id: String = ""
var player_side: String = "attacker"
var enemy_side: String = "defender"
var side_colors: Dictionary = {}
var side_names: Dictionary = {}
var side_heraldry: Dictionary = {}
var ships: Array = []
var by_id: Dictionary = {}
var views: Dictionary = {}  # id -> NavalShipView
var selected: Array = []
var paused: bool = false
var speed: float = 1.0
var autoplay: bool = false
var standalone: bool = false
var anim_time: float = 0.0
var wind: Dictionary = {}
var sea: NavalSea
var volleys: BattleVolleys
var effects: NavalEffects
var battle_audio: BattleAudio = null
var _pending_order: String = ""
var _finished_shown: bool = false
var _returned: bool = false
var _outcome: Dictionary = {}
var _hud_timer: float = 0.0
var _screenshot_path: String = ""
var _shot_at: float = -1.0
var _camera_view: String = ""
var _benchmark: bool = false
var _bench_frames: int = 0
var _bench_time: float = 0.0
var _bench_frame_ms: PackedFloat64Array = PackedFloat64Array()
var _bench_at: float = 150.0
var _last_left_ms: int = -10000
var _last_left_id: int = -1
var _audio_director: Node = null

@onready var camera_rig: BattleCamera = $CameraRig
@onready var hud: NavalHud = $HUD
@onready var world_env: WorldEnvironment = $WorldEnvironment
@onready var sun: DirectionalLight3D = $Sun


func configure(p_campaign_sim: Object, index: int, seed: int) -> void:
	campaign_sim = p_campaign_sim
	battle_index = index
	battle_seed = seed


func _ready() -> void:
	_parse_cmdline()
	hud.card_clicked.connect(_on_card_clicked)
	hud.card_double_clicked.connect(_on_card_double_clicked)
	hud.order_pressed.connect(_on_order)
	hud.speed_pressed.connect(_on_speed)
	hud.return_pressed.connect(_on_return)
	if not ClassDB.class_exists("NavalBattleSim"):
		push_error("NavalScene: NavalBattleSim missing (GDExtension not built?)")
		_fail_headless()
		return
	battle = ClassDB.instantiate("NavalBattleSim")
	var ok := false
	if campaign_sim != null:
		setup_data = campaign_sim.call("get_naval_battle_setup", battle_index)
		ok = not setup_data.is_empty() and bool(battle.call("setup", setup_data, battle_seed))
	else:
		standalone = true
		if scenario_id == "":
			scenario_id = "sluys"
		ok = bool(battle.call("setup_scenario", _data_dir(), scenario_id, battle_seed))
		if ok:
			setup_data = battle.call("get_setup")
	if not ok:
		push_error("NavalScene: naval battle setup failed")
		_fail_headless()
		return
	_begin()
	if _screenshot_path != "":
		call_deferred("_stage_screenshot")


func _fail_headless() -> void:
	if _screenshot_path != "" or _benchmark:
		get_tree().quit(1)


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	var paths := tree.root.get_node_or_null("MapPaths") if tree != null else null
	if paths != null:
		return str(paths.get("data_dir"))
	return ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()


func _begin() -> void:
	_audio_director = get_node_or_null("/root/AudioDirector")
	if _audio_director != null and _audio_director.has_method("stop_all"):
		_audio_director.call("stop_all")
	player_side = str(setup_data.get("player_side", "attacker"))
	if player_side == "" or player_side == "<null>":
		player_side = "attacker"
		autoplay = true
	enemy_side = "defender" if player_side == "attacker" else "attacker"
	battle.call("set_ai", enemy_side, true)
	if autoplay:
		battle.call("set_ai", player_side, true)
	for side in ["attacker", "defender"]:
		var side_setup: Dictionary = setup_data[side]
		var faction := str(side_setup.get("faction", ""))
		side_names[side] = str(side_setup.get("faction_name", side))
		side_colors[side] = BattleUiKit.faction_color(faction, Color(0.2, 0.3, 0.75) if side == "attacker" else Color(0.75, 0.15, 0.12))
		side_heraldry[side] = PortraitLoader.heraldry_texture(faction)
	wind = battle.call("get_wind")
	var weather_key := "rain" if bool(setup_data.get("rain", false)) else "clear"
	BattleAtmosphere.apply(world_env, sun, weather_key, camera_rig.camera, str(setup_data.get("season", "summer")))
	sea = NavalSea.new()
	add_child(sea)
	var shore := bool(setup_data.get("shore", false))
	sea.setup(float(wind.get("to", 0.0)), float(wind.get("strength", 0.5)), Color(0.62, 0.72, 0.82), shore, SHORE_X)
	volleys = BattleVolleys.new()
	volleys.name = "Volleys"
	add_child(volleys)
	# Traits tombés à l'eau : sous la surface (invisibles) ; ceux fichés dans les ponts tiennent
	# le temps de la volée.
	volleys.setup(func(_x: float, _z: float) -> float: return -4.0)
	volleys.sound_event.connect(_on_sound_event)
	effects = NavalEffects.new()
	add_child(effects)
	effects.setup()
	ships = battle.call("get_ships")
	for ship in ships:
		var side := str(ship["side"])
		var view := NavalShipView.new()
		add_child(view)
		var units: Array = (setup_data[side] as Dictionary).get("units", [])
		view.setup(ship, units, side_colors[side], side_heraldry[side])
		var other := "defender" if side == "attacker" else "attacker"
		view.captor_color = side_colors[other]
		view.captor_heraldry = side_heraldry[other]
		if side != player_side:
			view.set_ring_color(Color(0.9, 0.25, 0.2))
		views[int(ship["id"])] = view
	_index_ships()
	camera_rig.height_at = func(x: float, z: float) -> float: return maxf(sea.height_at(x, z), 0.0) + 1.0
	camera_rig.bounds = Rect2(-1400, -1000, 2800, 2000)
	camera_rig.max_distance = 1400.0
	_frame_camera()
	var scenario: Dictionary = battle.call("get_scenario")
	var place := str(setup_data.get("place_name", ""))
	var title := "Bataille navale"
	var subtitle := place
	if not scenario.is_empty():
		title = "Bataille de %s" % str(scenario.get("name", ""))
		subtitle = "%s — %s" % [place, _date_fr(str(scenario.get("date", "")))]
	hud.set_header(title, subtitle, player_side, side_colors)
	hud.set_wind(_wind_text())
	hud.add_log([["%s contre %s" % [side_names["attacker"], side_names["defender"]], BattleUiKit.INK_SOFT]])
	battle_audio = BattleAudio.new()
	add_child(battle_audio)
	battle_audio.setup(weather_key, camera_rig.camera)
	_refresh(0.0)


func _index_ships() -> void:
	by_id.clear()
	for ship in ships:
		by_id[int(ship["id"])] = ship


static func _date_fr(iso: String) -> String:
	var parts := iso.split("-")
	if parts.size() != 3:
		return iso
	var months := ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"]
	var day := int(parts[2])
	return "%s %s %s" % ["1er" if day == 1 else str(day), months[clampi(int(parts[1]) - 1, 0, 11)], parts[0]]


## Vent : d'où il vient (rose des vents, nord = -z), force, avantage du vent.
func _wind_text() -> String:
	var to := float(wind.get("to", 0.0))
	var from := to + PI
	var bearing := fposmod(rad_to_deg(atan2(cos(from), -sin(from))), 360.0)
	var names := ["nord", "nord-est", "est", "sud-est", "sud", "sud-ouest", "ouest", "nord-ouest"]
	var direction: String = names[int(round(bearing / 45.0)) % 8]
	var strength := float(wind.get("strength", 0.5))
	var force := "faible" if strength < 0.3 else ("frais" if strength < 0.65 else "fort")
	var gauge := str(wind.get("gauge", ""))
	var gauge_text := "personne n'a l'avantage du vent"
	if gauge == "attacker" or gauge == "defender":
		gauge_text = "avantage du vent : %s" % side_names.get(gauge, gauge)
	return "Vent %s du %s — %s" % [force, direction, gauge_text]


func _frame_camera() -> void:
	var own := Vector3.ZERO
	var foe := Vector3.ZERO
	var n_own := 0
	var n_foe := 0
	for ship in ships:
		var p := Vector3(float(ship["x"]), 0.0, float(ship["z"]))
		if str(ship["side"]) == player_side:
			own += p
			n_own += 1
		else:
			foe += p
			n_foe += 1
	own /= maxf(n_own, 1)
	foe /= maxf(n_foe, 1)
	var dir := (foe - own).normalized()
	# Derrière sa propre flotte, regard vers l'ennemi.
	var yaw := atan2(-dir.x, -dir.z)
	camera_rig.look_at_point(own + dir * 120.0, 260.0, yaw)


# --- Boucle -------------------------------------------------------------------------------


func _process(delta: float) -> void:
	if battle == null:
		return
	if _benchmark:
		_bench_frame(delta)
	var dt := 0.0 if paused or _finished_shown else delta * speed
	if dt > 0.0:
		battle.call("tick", dt)
		anim_time += dt
	_refresh(delta)


func _refresh(delta: float) -> void:
	ships = battle.call("get_ships")
	_index_ships()
	var dt := 0.0 if paused else delta * speed
	var wind_to := float(wind.get("to", 0.0))
	var wind_strength := float(wind.get("strength", 0.5))
	sea.update(anim_time, camera_rig.target, ships)
	volleys.tick_time(anim_time)
	var shots: Array = battle.call("get_shots")
	var events: Array = battle.call("get_events")
	for ship in ships:
		var id := int(ship["id"])
		var view: NavalShipView = views[id]
		var focus: Variant = null
		var melee := false
		var grappled: PackedInt32Array = ship.get("grappled", PackedInt32Array())
		if grappled.size() > 0 and by_id.has(int(grappled[0])):
			var other: Dictionary = by_id[int(grappled[0])]
			focus = Vector2(float(other["x"]), float(other["z"]))
			melee = float(ship.get("melee_time", 0.0)) > 0.0
		else:
			var target := int(ship.get("target", -1))
			if target < 0:
				target = int(ship.get("last_target", -1))
			if by_id.has(target):
				focus = Vector2(float(by_id[target]["x"]), float(by_id[target]["z"]))
		view.update_view(ship, anim_time, dt, sea, wind_to, wind_strength, focus, melee)
		_update_visitors(view, ship)
	effects.update(views, ships)
	_on_shots(shots)
	_on_events(events)
	_hud_timer -= delta
	if _hud_timer <= 0.0:
		_hud_timer = 0.25
		_refresh_hud()
	if battle_audio != null:
		battle_audio.update(_audio_units(), camera_rig.target, camera_rig.camera.global_position.y, dt, delta, anim_time)
	if bool(battle.call("is_finished")) and not _finished_shown:
		_show_end()


## Abordeurs sur le pont ennemi (le navire qui aborde a l'ordre « board » sur celui-ci et la
## mêlée a commencé) et équipage de prise sur un navire pris.
func _update_visitors(view: NavalShipView, ship: Dictionary) -> void:
	var id := int(ship["id"])
	var status := str(ship["status"])
	if status == "captured":
		var captor := str(ship.get("captor", ""))
		var unit_type := _main_melee_type(captor)
		view.set_visitors("prize", unit_type, side_colors.get(captor, Color.WHITE), side_heraldry.get(captor), 14, anim_time, null, "idle")
		view.set_visitors("boarders", "", Color.WHITE, null, 0, anim_time, null, "idle")
		return
	var boarders := 0
	var from: Dictionary = {}
	for other_id in ship.get("grappled", PackedInt32Array()):
		var other: Dictionary = by_id.get(int(other_id), {})
		if other.is_empty() or str(other["order"]) != "board" or int(other.get("target", -1)) != id:
			continue
		if float(other.get("melee_time", 0.0)) < 4.0:
			continue
		boarders += int(clampf(float(other["soldiers"]) * 0.25, 0.0, 24.0))
		from = other
	if from.is_empty():
		view.set_visitors("boarders", "", Color.WHITE, null, 0, anim_time, null, "melee")
		return
	var side := str(from["side"])
	view.set_visitors("boarders", _main_melee_type(side), side_colors[side], side_heraldry[side], boarders, anim_time, Vector2(float(from["x"]), float(from["z"])), "melee")


func _main_melee_type(side: String) -> String:
	var best := "unit_men_at_arms_foot"
	var side_setup: Dictionary = setup_data.get(side, {})
	for unit in side_setup.get("units", []):
		var unit_type := str((unit as Dictionary).get("unit_type", ""))
		var kind := BattleMeshes.figure_kind_of(unit_type)
		if kind == "infantry":
			return unit_type
	return best


func _on_shots(shots: Array) -> void:
	if shots.is_empty():
		return
	var ship_boxes := {}
	for ship in ships:
		var view: NavalShipView = views[int(ship["id"])]
		ship_boxes[int(ship["id"])] = {
			"x": view.global_position.x, "z": view.global_position.z, "y": view.deck_height(),
			"width": float(ship["length"]) * 0.45, "depth": float(ship["beam"]) * 0.5,
		}
	for shot in shots:
		var shooter := int(shot["shooter"])
		if views.has(shooter):
			(views[shooter] as NavalShipView).on_shot(anim_time)
		# Traits qui tombent autour du navire visé : le sol de BattleVolleys est la mer.
		volleys.on_shot(shot, ship_boxes, camera_rig.camera.global_position)


func _on_events(events: Array) -> void:
	var lines: Array = []
	for event in events:
		var kind := str(event["kind"])
		var ship: Dictionary = by_id.get(int(event["ship"]), {})
		var other: Dictionary = by_id.get(int(event.get("other", -1)), {})
		if ship.is_empty():
			continue
		var name_text := str(ship["name"])
		var name_cap := name_text.substr(0, 1).to_upper() + name_text.substr(1)
		var pos := Vector3(float(ship["x"]), 2.0, float(ship["z"]))
		var ours := str(ship["side"]) == player_side
		var good := BattleUiKit.GOOD
		var bad := BattleUiKit.RUBRIC
		match kind:
			"grapple":
				lines.append(["%s lance ses grappins sur %s." % [name_cap, other.get("name", "?")], BattleUiKit.INK])
				_play("shield_bash", pos)
			"board":
				lines.append(["Abordage ! %s monte à l'assaut de %s." % [name_cap, other.get("name", "?")], good if ours else bad])
				_play("war_cry", pos)
				_play("contact", pos)
			"capture":
				lines.append(["%s est pris !" % name_cap, bad if ours else good])
				_play("rout_cry", pos)
			"ignite":
				lines.append(["Le feu prend à bord de %s." % name_text, bad if ours else good])
			"fireship":
				lines.append(["Un brûlot s'écrase contre %s !" % other.get("name", "?"), BattleUiKit.RUBRIC])
				effects.splash(pos, 3.0)
			"abandon":
				lines.append(["L'équipage abandonne %s." % name_text, bad if ours else good])
			"sinking":
				lines.append(["%s sombre." % name_cap, bad if ours else good])
				effects.splash(pos, 4.0)
			"sunk":
				effects.splash(pos, 3.0)
			"ram":
				lines.append(["%s éperonne %s." % [name_cap, other.get("name", "?")], BattleUiKit.INK])
				var hit := Vector3(float(other.get("x", pos.x)), 1.0, float(other.get("z", pos.z)))
				effects.splash(pos.lerp(hit, 0.6), 4.0)
				_play("ram_hit", hit)
			"flee":
				lines.append(["%s rompt le combat et fuit." % name_cap, bad if ours else good])
			"escaped":
				lines.append(["%s a échappé." % name_cap, BattleUiKit.INK_SOFT])
			"cut":
				lines.append(["%s coupe les grappins." % name_cap, BattleUiKit.INK_SOFT])
	if not lines.is_empty():
		hud.add_log(lines)


func _play(event_name: String, at: Vector3) -> void:
	if battle_audio != null:
		battle_audio.play_event(event_name, at)


func _on_sound_event(event: StringName, position: Vector3, delay: float) -> void:
	if battle_audio == null:
		return
	if delay > 0.0:
		battle_audio.schedule(str(event), position, delay)
	else:
		battle_audio.play_event(str(event), position)


## Régiments fictifs pour les nappes sonores de BattleAudio (mêlée des navires grappinés).
func _audio_units() -> Array:
	var out: Array = []
	for ship in ships:
		var grappled: PackedInt32Array = ship.get("grappled", PackedInt32Array())
		out.append({
			"id": int(ship["id"]), "side": str(ship["side"]), "x": float(ship["x"]), "z": float(ship["z"]), "y": 3.0,
			"present": str(ship["status"]) == "afloat", "state": "melee" if grappled.size() > 0 and float(ship.get("melee_time", 0.0)) > 0.0 else "idle",
			"soldiers": int(float(ship["soldiers"])), "ammo": 0, "render": "infantry",
		})
	return out


func _refresh_hud() -> void:
	var strengths := [float(battle.call("get_strength", player_side)), float(battle.call("get_strength", enemy_side))]
	hud.set_balance(strengths[0], strengths[1])
	hud.set_clock(anim_time)
	var live: Array = []
	for id in selected:
		if by_id.has(id) and str(by_id[id]["status"]) == "afloat":
			live.append(id)
	selected = live
	for id in views:
		(views[id] as NavalShipView).set_selected(selected.has(id))
	hud.update_cards(ships, selected)
	var can_ram := false
	var fire_arrows := false
	for id in selected:
		can_ram = can_ram or bool(by_id[id].get("can_ram", false))
		fire_arrows = fire_arrows or bool(by_id[id].get("fire_arrows", false))
	hud.set_orders_enabled(not selected.is_empty(), can_ram, fire_arrows)
	if selected.size() == 1:
		var ship: Dictionary = by_id[selected[0]]
		hud.set_info("%s — %s : coque %d %%, %d hommes, %d marins, moral %d" % [ship["name"], ship["class_name"], int(100.0 * float(ship["hull"]) / maxf(float(ship["hull_max"]), 1.0)), int(float(ship["soldiers"])), int(float(ship["sailors"])), int(float(ship["morale"]))])
	elif selected.size() > 1:
		hud.set_info("%d navires choisis" % selected.size())


# --- Ordres -------------------------------------------------------------------------------


func _on_card_clicked(id: int, additive: bool) -> void:
	_select(id, additive)


func _on_card_double_clicked(id: int) -> void:
	if views.has(id):
		var view: NavalShipView = views[id]
		camera_rig.look_at_point(view.global_position, 70.0, camera_rig.yaw)


func _select(id: int, additive: bool) -> void:
	if not by_id.has(id) or str(by_id[id]["side"]) != player_side or str(by_id[id]["status"]) != "afloat":
		return
	if additive:
		if selected.has(id):
			selected.erase(id)
		else:
			selected.append(id)
	else:
		selected = [id]
	_pending_order = ""
	hud.set_hint("")
	_hud_timer = 0.0


func _on_order(order: String) -> void:
	match order:
		"hold", "disengage":
			for id in selected:
				_command({"type": order, "ship": id})
		"fire_arrows":
			var enable := true
			for id in selected:
				if bool(by_id[id].get("fire_arrows", false)):
					enable = false
			for id in selected:
				_command({"type": "fire_arrows", "ship": id, "enabled": enable})
		_:
			_pending_order = order
			hud.set_hint("Cliquez sur un navire ennemi : %s." % {"board": "aborder", "shoot": "tirer", "ram": "éperonner"}.get(order, order))
	_hud_timer = 0.0


func _command(command: Dictionary) -> bool:
	var result: Dictionary = battle.call("issue_command", command)
	if not bool(result.get("ok", false)):
		hud.set_hint(str(result.get("error", "Ordre refusé")))
		_play("ui_order_refused", camera_rig.target)
		return false
	return true


func _on_speed(value: float) -> void:
	if value <= 0.0:
		paused = not paused
	else:
		paused = false
		speed = value


func _unhandled_input(event: InputEvent) -> void:
	if battle == null or _finished_shown:
		return
	if event is InputEventKey and (event as InputEventKey).pressed:
		var key := (event as InputEventKey).keycode
		if key == KEY_SPACE:
			paused = not paused
		elif key >= KEY_1 and key <= KEY_3:
			speed = SPEEDS[key - KEY_1]
		elif key == KEY_ESCAPE:
			_pending_order = ""
			hud.set_hint("")
	if not (event is InputEventMouseButton) or not (event as InputEventMouseButton).pressed:
		return
	var mouse := event as InputEventMouseButton
	var point: Variant = _sea_point(mouse.position)
	if point == null:
		return
	var picked := _pick_ship(point)
	if mouse.button_index == MOUSE_BUTTON_LEFT:
		if _pending_order != "" and picked >= 0 and str(by_id[picked]["side"]) != player_side:
			for id in selected:
				_command({"type": _pending_order, "ship": id, "target": picked})
			_pending_order = ""
			hud.set_hint("")
			return
		if picked >= 0:
			var now := Time.get_ticks_msec()
			if picked == _last_left_id and now - _last_left_ms < DOUBLE_CLICK_MS:
				_on_card_double_clicked(picked)
			_last_left_ms = now
			_last_left_id = picked
			_select(picked, mouse.shift_pressed or mouse.ctrl_pressed)
		elif not mouse.shift_pressed:
			selected = []
			_hud_timer = 0.0
	elif mouse.button_index == MOUSE_BUTTON_RIGHT and not selected.is_empty():
		if picked >= 0 and str(by_id[picked]["side"]) != player_side:
			for id in selected:
				_command({"type": "board", "ship": id, "target": picked})
		else:
			var p: Vector3 = point
			var i := 0
			for id in selected:
				# Plusieurs navires : en ligne de front, 40 m d'écart.
				var offset := (float(i) - (selected.size() - 1) * 0.5) * 40.0
				_command({"type": "move", "ship": id, "x": p.x, "z": p.z + offset})
				i += 1


func _sea_point(screen: Vector2) -> Variant:
	var cam := camera_rig.camera
	var origin := cam.project_ray_origin(screen)
	var dir := cam.project_ray_normal(screen)
	if dir.y >= -0.001:
		return null
	# Plan à hauteur de pont (les navires sont cliqués sur leur coque, pas sur l'eau devant).
	var t := (3.0 - origin.y) / dir.y
	return origin + dir * t


func _pick_ship(point: Vector3) -> int:
	var best := -1
	var best_d := INF
	for ship in ships:
		if str(ship["status"]) in ["sunk", "escaped"]:
			continue
		var view: NavalShipView = views[int(ship["id"])]
		var d := Vector2(point.x - view.global_position.x, point.z - view.global_position.z)
		var h := float(ship["heading"])
		var along := d.x * cos(h) + d.y * sin(h)
		var across := -d.x * sin(h) + d.y * cos(h)
		var ex := along / (float(ship["length"]) * 0.55 + 3.0)
		var ez := across / (float(ship["beam"]) * 0.55 + 3.0)
		var r := ex * ex + ez * ez
		if r < 1.0 and r < best_d:
			best_d = r
			best = int(ship["id"])
	return best


# --- Fin ----------------------------------------------------------------------------------


func _show_end() -> void:
	_finished_shown = true
	_outcome = battle.call("get_outcome")
	hud.show_result(_outcome, side_names, "Retour à la campagne" if campaign_sim != null else "Quitter")
	var winner := str(_outcome.get("winner", ""))
	hud.add_log([["Fin du combat : %s." % ("victoire de %s" % side_names.get(winner, winner) if side_names.has(winner) else "aucun vainqueur"), BattleUiKit.RUBRIC]])


func _on_return() -> void:
	if _returned:
		return
	_returned = true
	var result := {"ok": true, "outcome": _outcome}
	if campaign_sim != null and battle_index >= 0:
		result = campaign_sim.call("resolve_naval_battle", battle_index, _outcome)
		result["outcome"] = _outcome
	if _audio_director != null and _audio_director.has_method("resume"):
		_audio_director.call("resume")
	if standalone and campaign_sim == null:
		get_tree().quit()
		return
	returned.emit(result)


# --- Ligne de commande, captures, banc d'essai --------------------------------------------


func _parse_cmdline() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--naval-scenario="):
			scenario_id = arg.trim_prefix("--naval-scenario=")
		elif arg.begins_with("--screenshot="):
			_screenshot_path = arg.trim_prefix("--screenshot=")
			autoplay = true
		elif arg.begins_with("--shot-at="):
			_shot_at = float(arg.trim_prefix("--shot-at="))
		elif arg.begins_with("--camera="):
			_camera_view = arg.trim_prefix("--camera=")
		elif arg.begins_with("--seed="):
			battle_seed = int(arg.trim_prefix("--seed="))
		elif arg.begins_with("--bench-at="):
			_bench_at = float(arg.trim_prefix("--bench-at="))
		elif arg == "--autoplay":
			autoplay = true
		elif arg == "--benchmark":
			_benchmark = true
			autoplay = true
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
			Engine.max_fps = 0


## Avance rapide (rendu mis à jour : cadavres, feux, navires) jusqu'à `seconds`.
func _fast_forward(seconds: float) -> void:
	while anim_time < seconds and not bool(battle.call("is_finished")):
		battle.call("tick", 0.5)
		anim_time += 0.5
		_refresh(0.5)


func _stage_screenshot() -> void:
	camera_rig.edge_pan_enabled = false
	if _shot_at > 0.0:
		_fast_forward(_shot_at)
	paused = true
	_apply_camera_view()
	for _i in 90:
		await get_tree().process_frame
	var image := get_viewport().get_texture().get_image()
	var path := _screenshot_path
	if not path.is_absolute_path():
		path = ProjectSettings.globalize_path("res://").path_join(path)
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var err := image.save_png(path)
	print("NavalScene: screenshot %s at %.0f s (%s)" % [path, anim_time, error_string(err)])
	get_tree().quit(0 if err == OK else 1)


## Cadrages des captures : overview (flottes), close (navire au contact), deck (à hauteur de
## pont), melee (abordage le plus fourni).
func _apply_camera_view() -> void:
	var focus := Vector3.ZERO
	var best := -1.0
	for ship in ships:
		if str(ship["status"]) != "afloat":
			continue
		var weight := float((ship.get("grappled", PackedInt32Array()) as PackedInt32Array).size()) * 100.0 + float(ship["fire"]) * 50.0 + float(ship["soldiers"]) * 0.01
		if str(ship["side"]) == player_side:
			weight += 1.0
		if weight > best:
			best = weight
			var view: NavalShipView = views[int(ship["id"])]
			focus = view.global_position
	match _camera_view:
		"overview":
			camera_rig.look_at_point(focus, 420.0, camera_rig.yaw)
		"close":
			camera_rig.look_at_point(focus, 60.0, camera_rig.yaw + 0.6)
		"deck":
			camera_rig.look_at_point(focus + Vector3(0, 4, 0), 24.0, camera_rig.yaw + 1.2)
		"melee":
			camera_rig.look_at_point(focus, 38.0, camera_rig.yaw + 0.9)
		_:
			camera_rig.look_at_point(focus, 140.0, camera_rig.yaw + 0.4)


func _bench_frame(delta: float) -> void:
	if _bench_frames == 0 and anim_time < _bench_at:
		_fast_forward(_bench_at)
		_apply_camera_view()
		speed = 1.0
	_bench_frames += 1
	if _bench_frames <= 30:
		return  # préchauffage
	_bench_time += delta
	_bench_frame_ms.append(delta * 1000.0)
	if _bench_frames >= BENCH_FRAMES + 30:
		var sorted := _bench_frame_ms.duplicate()
		sorted.sort()
		var fps := float(_bench_frame_ms.size()) / maxf(_bench_time, 0.001)
		var afloat := 0
		for ship in ships:
			if str(ship["status"]) == "afloat":
				afloat += 1
		var report := {
			"scene": "naval", "scenario": scenario_id, "frames": _bench_frame_ms.size(), "fps": snappedf(fps, 0.1),
			"p95_ms": snappedf(sorted[int(sorted.size() * 0.95)], 0.01), "ships": ships.size(), "afloat": afloat,
			"arrows": volleys.launched, "time": snappedf(anim_time, 0.1),
		}
		print("NAVAL_BENCH " + JSON.stringify(report))
		get_tree().quit()
