class_name BattleScene
extends Node3D

## Scène de bataille 3D temps réel avec pause (spec M7 § 4). Toute la simulation est dans
## `BattleSim` (Rust) : la scène avance le temps, affiche terrain, soldats (MultiMesh par camp et
## par famille), bannières, HUD, et convertit clics et touches en commandes.
##
## Lancement : depuis la carte (`configure(campaign_sim, index, seed)` avant `add_child`), ou seule
## (`godot --path game res://scenes/battle/battle.tscn`) : une campagne France est créée et une
## bataille France–Angleterre mise en scène (`debug_stage_battle`).
## Options (après `--`) : `--screenshot=<png>` (joue la bataille jusqu'au contact, capture, quitte),
## `--units=<n>` (complète chaque camp à n régiments, banc d'essai sans retour campagne),
## `--benchmark` (mesure les FPS sur 600 images puis quitte), `--autoplay` (IA des deux camps).

signal returned(result: Dictionary)

const KINDS := ["infantry", "archer", "cavalry", "siege"]
const SPEEDS := [1.0, 2.0, 4.0]
const DOUBLE_CLICK_MS := 350
const PICK_RADIUS_PX := 26.0
const BANNER_HEIGHT := 7.0

var campaign_sim: Object = null
var battle_index: int = -1
var battle_seed: int = 1
var battle: Object = null  # BattleSim
var setup: Dictionary = {}
var player_side: String = "attacker"
var enemy_side: String = "defender"
var side_colors: Dictionary = {}
var side_names: Dictionary = {}
var units: Array = []
var selected: Array[int] = []
var paused: bool = false
var speed: float = 1.0
var autoplay: bool = false
var padded: bool = false
var finished_shown: bool = false
var resolved: bool = false
var standalone: bool = false

var _mm: Dictionary = {}  # "side/kind" -> MultiMeshInstance3D
var _banners: Dictionary = {}  # id -> {node, flag_mat, label, count}
var _rings: Dictionary = {}  # id -> MeshInstance3D
var _left_press: Vector2 = Vector2(-1, -1)
var _right_press: Vector2 = Vector2(-1, -1)
var _right_press_ground: Vector3 = Vector3.ZERO
var _last_right_click_ms: int = -10000
var _drag_rect: ColorRect
var _hud_timer: float = 0.0
var _screenshot_path: String = ""
var _benchmark: bool = false
var _bench_frames: int = 0
var _bench_time: float = 0.0
var _pad_units: int = 0

@onready var terrain: BattleTerrain = $Terrain
@onready var camera_rig: BattleCamera = $CameraRig
@onready var hud: BattleHud = $HUD
@onready var world_env: WorldEnvironment = $WorldEnvironment
@onready var sun: DirectionalLight3D = $Sun


## À appeler avant `add_child` quand la bataille vient de la campagne.
func configure(p_campaign_sim: Object, index: int, seed: int) -> void:
	campaign_sim = p_campaign_sim
	battle_index = index
	battle_seed = seed


func _ready() -> void:
	_parse_cmdline()
	hud.card_clicked.connect(_on_card_clicked)
	hud.command_pressed.connect(_on_command)
	hud.return_pressed.connect(_on_return)
	_drag_rect = ColorRect.new()
	_drag_rect.color = Color(0.95, 0.8, 0.3, 0.18)
	_drag_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_drag_rect.visible = false
	hud.add_child(_drag_rect)
	if campaign_sim == null:
		standalone = true
		if not _stage_standalone():
			push_error("BattleScene: cannot stage a demo battle")
			return
	if not begin():
		push_error("BattleScene: battle setup failed")


## Démo autonome : campagne France 1337, principale armée française contre anglaise.
func _stage_standalone() -> bool:
	if not ClassDB.class_exists("CampaignSim"):
		return false
	var sim_facade := get_node_or_null("/root/SimFacade")
	var sim: Object = null
	if sim_facade != null and sim_facade.is_real and sim_facade.sim != null and sim_facade.sim.has_method("debug_stage_battle") and int(sim_facade.sim.call("get_turn")) >= 0:
		sim = sim_facade.sim
	else:
		sim = ClassDB.instantiate("CampaignSim")
		var paths := get_node_or_null("/root/MapPaths")
		var data_dir: String = paths.data_dir if paths != null else ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
		if not sim.call("new_campaign", data_dir, "fac_france", 1337):
			return false
	var armies := main_armies(sim, "fac_france", "fac_england")
	if armies.is_empty():
		return false
	var index: int = sim.call("debug_stage_battle", armies[0], armies[1])
	if index < 0:
		return false
	configure(sim, index, 1337)
	return true


## Ids de l'armée la plus nombreuse de chaque faction ([] si l'une manque).
static func main_armies(sim: Object, attacker_faction: String, defender_faction: String) -> Array:
	var best := {attacker_faction: ["", -1], defender_faction: ["", -1]}
	for id in sim.call("get_army_ids"):
		var army: Dictionary = sim.call("get_army", id)
		var faction := str(army.get("faction", ""))
		if best.has(faction) and army["units"].size() > best[faction][1]:
			best[faction] = [str(id), army["units"].size()]
	if best[attacker_faction][0] == "" or best[defender_faction][0] == "":
		return []
	return [best[attacker_faction][0], best[defender_faction][0]]


## Construit la bataille depuis `campaign_sim.get_battle_setup(battle_index)`.
func begin() -> bool:
	setup = campaign_sim.call("get_battle_setup", battle_index)
	if setup.is_empty():
		return false
	if _pad_units > 0:
		_pad_setup(_pad_units)
	battle = ClassDB.instantiate("BattleSim")
	if not battle.call("setup", setup, battle_seed):
		return false
	player_side = str(setup.get("player_side", "attacker"))
	if player_side == "":
		player_side = "attacker"
		autoplay = true
	enemy_side = "defender" if player_side == "attacker" else "attacker"
	if autoplay:
		battle.call("set_ai", player_side, true)
	for side in ["attacker", "defender"]:
		var side_setup: Dictionary = setup[side]
		side_names[side] = str(side_setup.get("faction_name", side))
		side_colors[side] = _faction_color(str(side_setup.get("faction", "")), side)
	var weather: Dictionary = battle.call("get_weather")
	terrain.build(battle.call("get_terrain"), str(weather.get("key", "clear")))
	_apply_weather(str(weather.get("key", "clear")))
	_build_soldier_layers()
	units = battle.call("get_units")
	for unit in units:
		_make_banner(unit)
	hud.set_title("Bataille de %s" % str(setup.get("province_name", "")), str(weather.get("label", "")), [side_colors[player_side], side_colors[enemy_side]])
	camera_rig.height_at = func(x: float, z: float) -> float: return terrain.height_at(x, z)
	camera_rig.bounds = Rect2(-150, -150, 1500, 1100)
	_frame_camera()
	hud.add_events(battle.call("get_events"))
	_refresh_view(true)
	return true


func _faction_color(faction: String, side: String) -> Color:
	var facade := get_node_or_null("/root/SimFacade")
	if facade != null and faction != "":
		var color: Color = facade.faction_color(faction)
		if color != Color(0.5, 0.5, 0.5):
			return color
	return Color(0.2, 0.3, 0.75) if side == "attacker" else Color(0.75, 0.15, 0.12)


## Banc d'essai `--units=n` : répète les régiments de chaque camp jusqu'à n.
func _pad_setup(count: int) -> void:
	padded = true
	for side in ["attacker", "defender"]:
		var list: Array = setup[side]["units"]
		var base := list.duplicate(true)
		var i := 0
		while list.size() < count and not base.is_empty():
			list.append(base[i % base.size()].duplicate(true))
			i += 1
		# Banc d'essai de la spec (§ 5) : régiments de 120 soldats.
		for unit in list:
			unit["soldiers"] = 120
			unit["max_soldiers"] = 120
		setup[side]["units"] = list


func _apply_weather(key: String) -> void:
	var env: Environment = world_env.environment
	match key:
		"fog":
			env.fog_enabled = true
			env.fog_light_color = Color(0.78, 0.8, 0.82)
			env.fog_density = 0.004
			sun.light_energy = 0.6
		"rain":
			env.fog_enabled = true
			env.fog_light_color = Color(0.55, 0.58, 0.62)
			env.fog_density = 0.0015
			sun.light_energy = 0.55
			_add_precipitation(Color(0.7, 0.75, 0.85, 0.5), 30.0, Vector3(0.05, 0.9, 0.05))
		"snow":
			env.fog_enabled = true
			env.fog_light_color = Color(0.88, 0.9, 0.93)
			env.fog_density = 0.002
			sun.light_energy = 0.75
			_add_precipitation(Color(1, 1, 1, 0.9), 3.0, Vector3(0.25, 0.25, 0.25))
		_:
			env.fog_enabled = false


func _add_precipitation(color: Color, fall_speed: float, drop_size: Vector3) -> void:
	var particles := CPUParticles3D.new()
	particles.name = "Precipitation"
	particles.amount = 1500
	particles.lifetime = 2.0
	particles.emission_shape = CPUParticles3D.EMISSION_SHAPE_BOX
	particles.emission_box_extents = Vector3(160, 1, 160)
	particles.direction = Vector3(0.1, -1, 0)
	particles.spread = 5.0
	particles.gravity = Vector3(0, -fall_speed, 0)
	particles.initial_velocity_min = fall_speed * 0.6
	particles.initial_velocity_max = fall_speed
	var mesh := BoxMesh.new()
	mesh.size = drop_size
	var mat := StandardMaterial3D.new()
	mat.albedo_color = color
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mesh.material = mat
	particles.mesh = mesh
	particles.position = Vector3(0, 60, 0)
	camera_rig.add_child(particles)


func _build_soldier_layers() -> void:
	for side in ["attacker", "defender"]:
		for kind in KINDS:
			var mm := MultiMesh.new()
			mm.transform_format = MultiMesh.TRANSFORM_3D
			mm.mesh = BattleMeshes.soldier(kind, side_colors[side])
			mm.instance_count = 0
			var instance := MultiMeshInstance3D.new()
			instance.name = "%s_%s" % [side, kind]
			instance.multimesh = mm
			instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			add_child(instance)
			_mm["%s/%s" % [side, kind]] = instance


func _make_banner(unit: Dictionary) -> void:
	var id := int(unit["id"])
	var node := Node3D.new()
	node.name = "Banner%d" % id
	add_child(node)
	var pole := MeshInstance3D.new()
	pole.mesh = BattleMeshes.pole()
	pole.scale = Vector3(1, BANNER_HEIGHT, 1)
	pole.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	node.add_child(pole)
	var flag := MeshInstance3D.new()
	var quad := QuadMesh.new()
	quad.size = Vector2(2.6, 1.7)
	var flag_mat := StandardMaterial3D.new()
	flag_mat.albedo_color = side_colors[str(unit["side"])]
	flag_mat.billboard_mode = BaseMaterial3D.BILLBOARD_FIXED_Y
	flag_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	flag_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	quad.material = flag_mat
	flag.mesh = quad
	flag.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	flag.position = Vector3(1.3, BANNER_HEIGHT - 0.9, 0)
	node.add_child(flag)
	var label := Label3D.new()
	label.text = BattleHud.CATEGORY_ICON.get(str(unit["render"]), "⚔") + ("★" if bool(unit["is_general"]) else "")
	label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	label.no_depth_test = true
	label.font_size = 64
	label.pixel_size = 0.02
	label.outline_size = 10
	label.modulate = Color(1, 0.97, 0.88)
	label.position = Vector3(1.3, BANNER_HEIGHT - 0.9, 0.05)
	node.add_child(label)
	var count := Label3D.new()
	count.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	count.no_depth_test = true
	count.font_size = 40
	count.pixel_size = 0.02
	count.outline_size = 8
	count.position = Vector3(1.3, BANNER_HEIGHT - 2.3, 0)
	node.add_child(count)
	_banners[id] = {"node": node, "flag_mat": flag_mat, "label": label, "count": count}
	var ring := MeshInstance3D.new()
	var ring_mat := StandardMaterial3D.new()
	ring_mat.albedo_color = Color(1.0, 0.85, 0.2)
	ring_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	ring_mat.no_depth_test = true
	ring_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	ring.material_override = ring_mat
	ring.visible = false
	add_child(ring)
	_rings[id] = ring


func _frame_camera() -> void:
	var sx := 0.0
	var sz := 0.0
	var n := 0
	for unit in units:
		if str(unit["side"]) == player_side:
			sx += float(unit["x"])
			sz += float(unit["z"])
			n += 1
	var center := Vector3(sx / maxf(n, 1), 0, sz / maxf(n, 1))
	var yaw := PI if player_side == "attacker" else 0.0
	# Regarder un peu devant sa propre ligne, vers l'ennemi.
	center.z += 70.0 if player_side == "attacker" else -70.0
	camera_rig.look_at_point(center, 260.0, yaw)


# --- Boucle ---------------------------------------------------------------------------


func _process(delta: float) -> void:
	if battle == null:
		return
	if not paused and not battle.call("is_finished"):
		battle.call("tick", delta * speed)
	_refresh_view(false, delta)
	if battle.call("is_finished") and not finished_shown:
		_show_end()
	if _benchmark:
		_bench_frames += 1
		_bench_time += delta
		if _bench_frames == 600:
			var fps := _bench_frames / maxf(_bench_time, 0.001)
			var soldiers := 0
			for unit in units:
				soldiers += int(unit["soldiers"])
			print("BattleScene benchmark: %d units, %d soldiers, %.1f FPS average over %d frames (engine %d FPS)" % [units.size(), soldiers, fps, _bench_frames, Engine.get_frames_per_second()])
			get_tree().quit(0)


func _refresh_view(force: bool, delta: float = 0.0) -> void:
	units = battle.call("get_units")
	for side in ["attacker", "defender"]:
		for kind in KINDS:
			var mm: MultiMesh = (_mm["%s/%s" % [side, kind]] as MultiMeshInstance3D).multimesh
			var buffer: PackedFloat32Array = battle.call("get_soldier_buffer", side, kind)
			var count := buffer.size() / 12
			if count > mm.instance_count:
				mm.instance_count = count
			if mm.instance_count == 0:
				continue
			buffer.resize(mm.instance_count * 12)
			mm.buffer = buffer
			mm.visible_instance_count = count
	var banner_scale := _banner_scale()
	for unit in units:
		var id := int(unit["id"])
		var banner: Dictionary = _banners[id]
		var node: Node3D = banner["node"]
		var present: bool = unit["present"]
		node.visible = present
		var ring: MeshInstance3D = _rings[id]
		ring.visible = present and selected.has(id)
		if not present:
			continue
		var pos := Vector3(float(unit["x"]), float(unit["y"]), float(unit["z"]))
		node.position = pos + Vector3(0, 0, 0)
		node.scale = Vector3.ONE * banner_scale
		var routing := str(unit["state"]) == "routing"
		var flag_mat: StandardMaterial3D = banner["flag_mat"]
		flag_mat.albedo_color = Color(0.92, 0.92, 0.9) if routing else side_colors[str(unit["side"])]
		var count: Label3D = banner["count"]
		count.text = str(int(unit["soldiers"]))
		count.modulate = Color(1, 0.85, 0.3) if selected.has(id) else Color(1, 1, 1)
		if ring.visible:
			ring.position = pos + Vector3(0, 0.6, 0)
			ring.rotation = Vector3(0, float(unit["facing"]), 0)
			var size := Vector2(float(unit["width"]) + 3.0, float(unit["depth"]) + 3.0)
			if not ring.has_meta("size") or (ring.get_meta("size") as Vector2).distance_to(size) > 0.5:
				ring.set_meta("size", size)
				ring.mesh = BattleMeshes.outline(size.x, size.y, 0.45)
	_hud_timer -= delta
	if force or _hud_timer <= 0.0:
		_hud_timer = 0.1
		hud.set_clock(float(battle.call("get_elapsed")), speed, paused)
		hud.set_balance(side_names[player_side], int(battle.call("get_strength", player_side)), side_names[enemy_side], int(battle.call("get_strength", enemy_side)))
		hud.update_cards(units, player_side, selected)
		var events: Array = battle.call("get_events")
		if not events.is_empty():
			hud.add_events(events)


func _banner_scale() -> float:
	return clampf(camera_rig.distance * 0.014, 2.0, 9.0)


func _show_end() -> void:
	finished_shown = true
	var outcome: Dictionary = battle.call("get_outcome")
	var winner := str(outcome.get("winner", "defender"))
	var won := winner == player_side
	var lines: Array[String] = []
	lines.append("Vainqueur : %s (%s)." % [side_names[winner], "attaquant" if winner == "attacker" else "défenseur"])
	for side in [player_side, enemy_side]:
		var result: Dictionary = outcome.get(side, {})
		var start := 0
		for unit in units:
			if str(unit["side"]) == side:
				start += int(unit["initial_soldiers"])
		var general := ""
		if bool(result.get("general_killed", false)):
			general = " Le général est tombé."
		elif bool(result.get("general_captured", false)):
			general = " Le général est capturé."
		lines.append("%s : %d hommes engagés, %d pertes.%s" % [side_names[side], start, int(result.get("total_losses", 0)), general])
	lines.append("Durée : %d min %02d s." % [int(outcome.get("duration", 0.0)) / 60, int(outcome.get("duration", 0.0)) % 60])
	hud.show_end("Victoire !" if won else "Défaite…", "\n".join(lines))


## « Retour à la campagne » : applique le résultat (`resolve_battle`) puis rend la main.
func _on_return() -> void:
	if resolved:
		return
	resolved = true
	var result := {"ok": false, "error": "bataille non terminée", "events": []}
	if battle != null and battle.call("is_finished") and campaign_sim != null and not padded:
		result = campaign_sim.call("resolve_battle", battle_index, battle.call("get_outcome"))
		if not result.get("ok", false):
			push_error("BattleScene: resolve_battle refused: %s" % result.get("error", "?"))
	returned.emit(result)
	if standalone:
		get_tree().change_scene_to_file("res://scenes/start_menu.tscn")


# --- Entrées --------------------------------------------------------------------------


func _unhandled_input(event: InputEvent) -> void:
	if battle == null:
		return
	if event is InputEventKey and event.pressed and not event.echo:
		match event.keycode:
			KEY_SPACE:
				_toggle_pause()
			KEY_1:
				speed = SPEEDS[0]
			KEY_2:
				speed = SPEEDS[1]
			KEY_3:
				speed = SPEEDS[2]
			KEY_F:
				_on_command("formation")
			KEY_G:
				_on_command("fire_at_will")
			KEY_H:
				_on_command("halt")
			KEY_ESCAPE:
				selected.clear()
			KEY_F12:
				_take_screenshot(ProjectSettings.globalize_path("res://").path_join("../docs/img/godot-battle-%d.png" % Time.get_unix_time_from_system()).simplify_path(), false)
	elif event is InputEventMouseButton:
		var button := event as InputEventMouseButton
		if button.button_index == MOUSE_BUTTON_LEFT:
			if button.pressed:
				_left_press = button.position
			else:
				_finish_left(button.position, button.shift_pressed)
		elif button.button_index == MOUSE_BUTTON_RIGHT:
			if button.pressed:
				_right_press = button.position
				_right_press_ground = ground_point(button.position)
			else:
				_finish_right(button.position)
	elif event is InputEventMouseMotion and _left_press.x >= 0.0:
		var motion := event as InputEventMouseMotion
		var rect := Rect2(_left_press, motion.position - _left_press).abs()
		_drag_rect.visible = rect.size.length() > 8.0
		_drag_rect.position = rect.position
		_drag_rect.size = rect.size


func _toggle_pause() -> void:
	paused = not paused


func _finish_left(position: Vector2, additive: bool) -> void:
	var rect := Rect2(_left_press, position - _left_press).abs()
	_left_press = Vector2(-1, -1)
	_drag_rect.visible = false
	if not additive:
		selected.clear()
	if rect.size.length() > 8.0:
		for unit in units:
			if str(unit["side"]) != player_side or not bool(unit["present"]):
				continue
			var screen := _unit_screen(unit)
			if screen.x > -1e5 and rect.has_point(screen) and not selected.has(int(unit["id"])):
				selected.append(int(unit["id"]))
		return
	var picked := pick_unit(position, player_side)
	if picked >= 0:
		if additive and selected.has(picked):
			selected.erase(picked)
		elif not selected.has(picked):
			selected.append(picked)


func _finish_right(position: Vector2) -> void:
	var press := _right_press
	_right_press = Vector2(-1, -1)
	if selected.is_empty():
		return
	var now := Time.get_ticks_msec()
	var double_click := now - _last_right_click_ms < DOUBLE_CLICK_MS
	_last_right_click_ms = now
	if press.distance_to(position) > 20.0:
		# Glisser-droit : ligne de p0 à p1, front tourné à l'opposé de la caméra.
		var p0 := _right_press_ground
		var p1 := ground_point(position)
		var dir := Vector2(p1.x - p0.x, p1.z - p0.z)
		if dir.length() < 2.0:
			return
		var normal := Vector2(-dir.y, dir.x).normalized()
		var mid := (p0 + p1) * 0.5
		var cam := camera_rig.camera.global_position
		if normal.dot(Vector2(mid.x - cam.x, mid.z - cam.z)) < 0.0:
			normal = -normal
		issue({"type": "move", "units": selected.duplicate(), "x": mid.x, "z": mid.z, "run": double_click, "facing": atan2(normal.x, normal.y)})
		return
	var enemy := pick_unit(position, enemy_side)
	if enemy >= 0:
		issue({"type": "attack", "units": selected.duplicate(), "target": enemy, "run": true})
		return
	var point := ground_point(position)
	issue({"type": "move", "units": selected.duplicate(), "x": point.x, "z": point.z, "run": double_click})


## Envoie une commande à la simulation ; les refus s'affichent au journal.
func issue(command: Dictionary) -> Dictionary:
	var result: Dictionary = battle.call("issue_command", command)
	if not result.get("ok", false):
		hud.add_events([{"time": battle.call("get_elapsed"), "text_fr": "Ordre refusé : %s" % result.get("error", "?")}])
	return result


func _on_card_clicked(unit_id: int, additive: bool) -> void:
	if not additive:
		selected.clear()
	if not selected.has(unit_id):
		selected.append(unit_id)


func _on_command(command: String) -> void:
	match command:
		"pause":
			_toggle_pause()
			return
		"withdraw_all":
			var all: Array[int] = []
			for unit in units:
				if str(unit["side"]) == player_side and bool(unit["present"]) and str(unit["state"]) != "routing" and not bool(unit["withdrawing"]):
					all.append(int(unit["id"]))
			if not all.is_empty():
				issue({"type": "withdraw", "units": all})
			return
	var ids := _available_selection()
	if ids.is_empty():
		return
	match command:
		"halt":
			issue({"type": "halt", "units": ids})
		"withdraw":
			issue({"type": "withdraw", "units": ids})
		"fire_at_will":
			var shooters: Array[int] = []
			var enable := false
			for unit in units:
				if ids.has(int(unit["id"])) and bool(unit["can_shoot"]):
					shooters.append(int(unit["id"]))
					enable = enable or not bool(unit["fire_at_will"])
			if not shooters.is_empty():
				issue({"type": "fire_at_will", "units": shooters, "enabled": enable})
		"formation":
			for unit in units:
				if ids.has(int(unit["id"])):
					issue({"type": "formation", "units": [int(unit["id"])], "kind": _next_formation(unit)})


func _available_selection() -> Array[int]:
	var ids: Array[int] = []
	for unit in units:
		if selected.has(int(unit["id"])) and bool(unit["present"]) and str(unit["state"]) != "routing":
			ids.append(int(unit["id"]))
	return ids


## Formation suivante autorisée pour la famille de l'unité.
func _next_formation(unit: Dictionary) -> String:
	var cycle: Array = ["line", "column"]
	match str(unit["category"]):
		"infantry":
			cycle = ["line", "column", "square"]
		"cavalry":
			cycle = ["line", "wedge", "column"]
		"siege":
			cycle = ["line"]
	var current := cycle.find(str(unit["formation"]))
	return cycle[(current + 1) % cycle.size()]


# --- Picking --------------------------------------------------------------------------


func _unit_screen(unit: Dictionary) -> Vector2:
	var camera := camera_rig.camera
	var pos := Vector3(float(unit["x"]), float(unit["y"]) + 1.5, float(unit["z"]))
	if camera.is_position_behind(pos):
		return Vector2(-1e6, -1e6)
	return camera.unproject_position(pos)


## Unité de `side` la plus proche du curseur (rectangle projeté ou bannière), -1 sinon.
func pick_unit(screen: Vector2, side: String) -> int:
	var camera := camera_rig.camera
	var best := -1
	var best_distance := INF
	for unit in units:
		if str(unit["side"]) != side or not bool(unit["present"]):
			continue
		var center := _unit_screen(unit)
		if center.x < -1e5:
			continue
		var half_width := float(unit["width"]) * 0.5
		var facing := float(unit["facing"])
		var edge := Vector3(float(unit["x"]) + cos(facing) * half_width, float(unit["y"]) + 1.5, float(unit["z"]) - sin(facing) * half_width)
		var radius := maxf(PICK_RADIUS_PX, camera.unproject_position(edge).distance_to(center))
		var banner_top := camera.unproject_position(Vector3(float(unit["x"]), float(unit["y"]) + BANNER_HEIGHT * _banner_scale(), float(unit["z"])))
		var d := minf(screen.distance_to(center), screen.distance_to(banner_top))
		if d < radius and d < best_distance:
			best_distance = d
			best = int(unit["id"])
	return best


## Point du sol sous le curseur (rayon caméra, itération sur la hauteur du terrain).
func ground_point(screen: Vector2) -> Vector3:
	var camera := camera_rig.camera
	var origin := camera.project_ray_origin(screen)
	var dir := camera.project_ray_normal(screen)
	if absf(dir.y) < 1e-4:
		return Vector3(origin.x, 0, origin.z)
	var t := (terrain.height_at(origin.x, origin.z) - origin.y) / dir.y
	for _i in 8:
		var p := origin + dir * t
		var next := (terrain.height_at(p.x, p.z) - origin.y) / dir.y
		if absf(next - t) < 0.05:
			t = next
			break
		t = next
	var point := origin + dir * maxf(t, 0.0)
	point.x = clampf(point.x, 5.0, 1195.0)
	point.z = clampf(point.z, 5.0, 795.0)
	return point


# --- Ligne de commande, captures ------------------------------------------------------


func _parse_cmdline() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--screenshot="):
			_screenshot_path = arg.trim_prefix("--screenshot=")
			autoplay = true
		elif arg.begins_with("--units="):
			_pad_units = int(arg.trim_prefix("--units="))
		elif arg == "--benchmark":
			_benchmark = true
			autoplay = true
		elif arg == "--autoplay":
			autoplay = true
	if _screenshot_path != "":
		call_deferred("_stage_screenshot")


## Capture : IA des deux camps jusqu'au premier contact (+ 12 s), sélection de deux régiments
## du joueur, caméra sur la mêlée, capture après quelques images.
func _stage_screenshot() -> void:
	if battle == null:
		get_tree().quit(1)
		return
	camera_rig.edge_pan_enabled = false
	var contact_time := -1.0
	for _i in 3000:
		battle.call("tick", 0.1)
		if contact_time < 0.0:
			for unit in battle.call("get_units"):
				if str(unit["state"]) == "melee":
					contact_time = float(battle.call("get_elapsed"))
					break
		elif float(battle.call("get_elapsed")) > contact_time + 12.0 or battle.call("is_finished"):
			break
	paused = true
	units = battle.call("get_units")
	var focus := Vector3.ZERO
	var n := 0
	for unit in units:
		if str(unit["state"]) == "melee":
			focus += Vector3(float(unit["x"]), 0, float(unit["z"]))
			n += 1
	if n > 0:
		focus /= n
		camera_rig.look_at_point(focus + Vector3(0, 0, -25 if player_side == "attacker" else 25), 120.0, (PI if player_side == "attacker" else 0.0) + 0.5)
	for unit in units:
		if str(unit["side"]) == player_side and bool(unit["present"]) and selected.size() < 2:
			selected.append(int(unit["id"]))
	_refresh_view(true)
	for _i in 40:
		await get_tree().process_frame
	_take_screenshot(_screenshot_path, true)


func _take_screenshot(path: String, quit_after: bool) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var err := image.save_png(path)
	print("BattleScene: screenshot %s (%s)" % [path, error_string(err)])
	if quit_after:
		get_tree().quit(0 if err == OK else 1)
