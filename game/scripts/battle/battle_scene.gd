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
## `--benchmark` (mesure les FPS sur 600 images puis quitte ; `--bench-at=<s>` avance d'abord la bataille), `--autoplay` (IA des deux camps),
## `--siege` (démo autonome : assaut français de la Guyenne, bataille de siège M8),
## `--closeup` (capture : caméra rapprochée sur la mêlée), `--weather=<clear|fog|rain|snow>`
## (rendu seulement : force l'aspect de la météo, la simulation garde la sienne),
## `--camera=x,z,distance,lacet` (capture : position de caméra imposée), `--deploy-shot` (avec
## `--screenshot=` : capture de la phase de déploiement, F5c), `--result-shot` (avec
## `--screenshot=` : bataille jouée jusqu'au bout, capture de l'écran de fin, B2),
## `--no-effects` (sans poussière ni traits, B4 : captures « avant », mesures A/B),
## `--shot-at=<s>` (capture : à cet instant de la bataille plutôt qu'au premier contact, B4).

signal returned(result: Dictionary)

const KINDS := ["infantry", "archer", "cavalry", "siege"]
const SPEEDS := [1.0, 2.0, 4.0]
const DOUBLE_CLICK_MS := 350
const PICK_RADIUS_PX := 26.0
const BANNER_HEIGHT := 7.0
const BANNER_SHADER := preload("res://shaders/battle_banner.gdshader")
## Barre des ordres du chef (F10b).
const LEADER_ORDERS_BAR := preload("res://scripts/battle/leader_orders_bar.gd")
## Musique dynamique par intensité (B3 / T4).
const BATTLE_MUSIC := preload("res://scripts/battle/battle_music.gd")

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
var _returned: bool = false  # UB1 : « Retour à la campagne » déjà émis
var standalone: bool = false
var siege_view: BattleSiege = null  # batailles de siège (M8)
var siege_demo: bool = false

var _mm: Dictionary = {}  # unit id -> MultiMeshInstance3D (BattleSoldiers.layers)
var soldiers: BattleSoldiers = null
var effects: BattleEffects = null  # B4 : poussière, traits, fumée des bombardes, gués
var _weather_key: String = "clear"
var _no_effects: bool = false  # `--no-effects` : captures « avant » et mesures A/B
var _banners: Dictionary = {}  # id -> {node, flag_mat, routing}
var markers: BattleUnitMarkers = null  # B2 : bannières flottantes (repères 2D)
var result_screen: BattleResultScreen = null  # B2 : écran de fin
var _result_shot: bool = false
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
var _bench_at: float = -1.0  # `--bench-at=<s>` : avance rapide avant la mesure
var _bench_start_elapsed: float = 0.0
var _pad_units: int = 0
var _closeup: bool = false
var _shot_at: float = -1.0  # B4 : `--shot-at=<s>`
var _weather_override: String = ""
var _camera_override: String = ""
var _last_group_ms: int = -10000
var deployment: DeploymentController = null  # F5c : phase de déploiement du joueur
var _deploy_shot: bool = false
var _sortie_shown: bool = false
var music: BattleMusicDirector = null  # B3 : musique dynamique par intensité
var battle_audio: BattleAudio = null  # AU1 : sons spatialisés (mêlée, volées, siège, météo)
var _siege_audio_timer: float = 0.0
var _audio_director: Node = null  # B3 : mis en veille pendant la bataille, réveillé au retour

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
	hud.card_double_clicked.connect(_on_card_double_clicked)
	hud.command_pressed.connect(_on_command)
	hud.speed_pressed.connect(_on_speed_pressed)
	hud.minimap_clicked.connect(_on_minimap_clicked)
	hud.leader_clicked.connect(_on_leader_clicked)  # UB1 : sceau du chef
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
	var index: int = -1
	if siege_demo and sim.has_method("debug_stage_siege"):
		index = sim.call("debug_stage_siege", armies[0], "prov_guyenne")
	else:
		index = sim.call("debug_stage_battle", armies[0], armies[1])
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
	BattleTerrain.apply_site_overrides(setup)  # B5 : --terrain= --season= --village --coast
	# B3 : la musique de campagne cède la place à la musique de bataille (réveillée au retour).
	_audio_director = get_node_or_null("/root/AudioDirector")
	if _audio_director != null:
		_audio_director.call("stop_all")
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
	var weather_key := _weather_override if _weather_override != "" else str(weather.get("key", "clear"))
	_weather_key = weather_key
	var terrain_data: Dictionary = battle.call("get_terrain")
	terrain.build(terrain_data, weather_key)
	if terrain_data.has("siege"):
		siege_view = BattleSiege.new()
		siege_view.name = "Siege"
		add_child(siege_view)
		siege_view.build(terrain_data["siege"], func(x: float, z: float) -> float: return terrain.height_at(x, z))
	BattleAtmosphere.apply(world_env, sun, weather_key, camera_rig.camera)
	_open_deployment()
	units = battle.call("get_units")
	_build_soldier_layers()
	for unit in units:
		_make_banner(unit)
	_build_markers()
	var title := ("Assaut %s" if siege_view != null else "Bataille %s") % BattleScene.de(str(setup.get("province_name", "")))
	# `--weather=` ne force que le rendu (outil de capture) : la simulation, donc les règles
	# (tir, fatigue) et le libellé, gardent la météo tirée par `core`. On le signale au bandeau
	# plutôt que d'afficher une météo que les règles n'appliquent pas.
	var weather_label := str(weather.get("label", ""))
	if weather_key != str(weather.get("key", "clear")):
		weather_label += " (rendu forcé : %s)" % weather_key
		print("BattleScene: --weather=%s overrides rendering only; simulated weather is %s" % [weather_key, weather.get("key", "?")])
	hud.set_title(title, weather_label, [side_colors[player_side], side_colors[enemy_side]])
	hud.set_site(str(terrain_data.get("site_label", "")))
	hud.player_faction = str((setup[player_side] as Dictionary).get("faction", ""))
	hud.set_leader((setup[player_side] as Dictionary).get("general", null), hud.player_faction)
	camera_rig.height_at = func(x: float, z: float) -> float: return terrain.world_height(x, z)
	camera_rig.bounds = Rect2(-150, -150, 1500, 1100)
	_frame_camera()
	hud.minimap.flipped = player_side == "attacker"
	hud.minimap.setup(terrain_data, side_colors)
	hud.add_events(battle.call("get_events"))
	add_child(LEADER_ORDERS_BAR.new(self))
	music = BATTLE_MUSIC.new()
	music.name = "Music"
	add_child(music)
	music.setup(self)
	battle_audio = BattleAudio.new()
	add_child(battle_audio)
	battle_audio.setup(_weather_key, camera_rig.camera)
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


func _build_soldier_layers() -> void:
	soldiers = BattleSoldiers.new()
	soldiers.name = "Soldiers"
	add_child(soldiers)
	var factions := {}
	for side in ["attacker", "defender"]:
		factions[side] = str((setup[side] as Dictionary).get("faction", ""))
	soldiers.setup(units, side_colors, factions)
	_mm = soldiers.layers
	if _no_effects:
		return
	effects = BattleEffects.new()
	effects.name = "Effects"
	add_child(effects)
	var river: Dictionary = terrain.terrain.get("river", {})
	var half_width := float(river.get("width", 0.0)) * 0.5
	effects.setup(_weather_key, func(x: float, z: float) -> float: return terrain.world_height(x, z), func(x: float, z: float) -> int: return 1 if half_width > 0.0 and terrain.river_distance(x, z) < half_width else 0)


## B4 : effets (poussière, traits…) d'après l'état des régiments ; `dt` = temps simulé écoulé.
func _update_effects(dt: float) -> void:
	terrain.update_trample(units, dt)  # B7 : neige piétinée (sans effet hors neige au sol)
	if effects == null:
		return
	var camera := get_viewport().get_camera_3d()
	var camera_pos := camera.global_position if camera != null else Vector3.ZERO
	effects.update(units, soldiers, soldiers.anim_time, dt, camera_pos)


func _make_banner(unit: Dictionary) -> void:
	var id := int(unit["id"])
	var side := str(unit["side"])
	var node := Node3D.new()
	node.name = "Banner%d" % id
	add_child(node)
	var pole := MeshInstance3D.new()
	pole.mesh = BattleMeshes.pole()
	pole.scale = Vector3(1, BANNER_HEIGHT, 1)
	pole.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	node.add_child(pole)
	var flag := MeshInstance3D.new()
	var flag_mat := ShaderMaterial.new()
	flag_mat.shader = BANNER_SHADER
	flag_mat.set_shader_parameter("livery", side_colors[side])
	flag_mat.set_shader_parameter("phase", float(id) * 1.7)
	var cloth := _banner_cloth(unit, str((setup[side] as Dictionary).get("faction", "")))
	var flag_size: Vector2 = cloth["size"]
	flag.mesh = BattleMeshes.flag(flag_size.x, flag_size.y)
	flag_mat.set_shader_parameter("heraldry", cloth["texture"])
	flag_mat.set_shader_parameter("has_heraldry", cloth["texture"] != null)
	flag_mat.set_shader_parameter("full_texture", cloth["full"])
	flag_mat.set_shader_parameter("flag_length", flag_size.x)
	flag.material_override = flag_mat
	flag.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	flag.position = Vector3(0.03, BANNER_HEIGHT - 0.05, 0)
	node.add_child(flag)
	_banners[id] = {"node": node, "flag_mat": flag_mat, "routing": false}
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


## Étoffe d'un drapeau de régiment : bannière peinte de la faction (`heraldry/banners/`,
## 256×512, tissu dans le haut, bas transparent) pour la noblesse, fanion à queue d'aronde (4:1)
## pour les autres, étendards royaux pour le général de France / d'Angleterre ; à défaut, centre
## de l'écu de la faction (repli).
func _banner_cloth(unit: Dictionary, faction: String) -> Dictionary:
	var dir := "res://assets/heraldry/banners/"
	var noble := str(unit.get("type", "")) in ["unit_knights", "unit_men_at_arms_foot"]
	var candidates: Array = []
	if bool(unit.get("is_general", false)) and faction in ["fac_france", "fac_england"]:
		# Pas de quartier : oriflamme (France) / dragon (Angleterre) ; sinon Saint-Georges pour
		# l'armée royale anglaise, bannière de la faction pour la française.
		var side := str(unit.get("side", ""))
		var no_quarter: bool = battle != null and battle.has_method("get_no_quarter") and bool(battle.call("get_no_quarter", side))
		if no_quarter:
			candidates.append([dir + ("oriflamme.png" if faction == "fac_france" else "dragon.png"), Vector2(1.3, 2.6)])
		elif faction == "fac_england":
			candidates.append([dir + "st_george.png", Vector2(1.3, 2.6)])
	if noble or str(unit.get("render", "")) == "siege":
		candidates.append([dir + "%s_banner.png" % faction, Vector2(1.3, 2.6)])
	else:
		candidates.append([dir + "%s_pennon.png" % faction, Vector2(3.0, 0.75)])
		candidates.append([dir + "%s_banner.png" % faction, Vector2(1.3, 2.6)])
	for candidate in candidates:
		var texture := PortraitLoader.load_texture(candidate[0])
		if texture != null:
			return {"texture": texture, "size": candidate[1], "full": true}
	return {"texture": PortraitLoader.heraldry_texture(faction), "size": Vector2(2.6, 1.7), "full": false}


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
	# A1-06 : vue d'ouverture plus basse et plus proche (on voit des hommes, pas des points).
	camera_rig.look_at_point(center, 170.0, yaw)


# --- Boucle ---------------------------------------------------------------------------


func _process(delta: float) -> void:
	if battle == null:
		return
	if not paused and not battle.call("is_finished"):
		battle.call("tick", delta * speed)
	if music != null:
		music.update(delta)
	_refresh_view(false, delta)
	_update_audio(delta)
	if battle.call("is_finished") and not finished_shown:
		_show_end()
	if _benchmark:
		if _bench_frames == 0:
			if _bench_at > 0.0:
				_fast_forward(_bench_at)
			_bench_start_elapsed = float(battle.call("get_elapsed"))
			soldiers.start_timing()
			_apply_camera_override()  # banc d'essai rapproché (lot B1)
		_bench_frames += 1
		_bench_time += delta
		if _bench_frames == 600:
			var fps := _bench_frames / maxf(_bench_time, 0.001)
			var soldiers := 0
			for unit in units:
				soldiers += int(unit["soldiers"])
			print("BattleScene benchmark: %d units, %d soldiers, %.1f FPS average over %d frames (engine %d FPS)%s" % [units.size(), soldiers, fps, _bench_frames, Engine.get_frames_per_second(), self.soldiers.timing_report()])
			print("BattleScene benchmark: measured from %.0f s, %d missiles launched" % [_bench_start_elapsed, effects.launched if effects != null else 0])
			get_tree().quit(0)


## AU1 : sons spatialisés d'après les régiments (et le siège, 4 fois par seconde).
func _update_audio(delta: float) -> void:
	if battle_audio == null:
		return
	var running: bool = not paused and not battle.call("is_finished")
	var elapsed := float(battle.call("get_elapsed"))
	var height := camera_rig.camera.global_position.y - camera_rig.target.y
	battle_audio.update(units, camera_rig.target, height, delta * speed if running else 0.0, delta, elapsed)
	if siege_view != null and running:
		_siege_audio_timer -= delta
		if _siege_audio_timer <= 0.0:
			_siege_audio_timer = 0.25
			battle_audio.update_siege(battle.call("get_siege"), elapsed)


## Avance la simulation (pas de 0,1 s) jusqu'à `seconds`, cadavres et effets compris.
func _fast_forward(seconds: float) -> void:
	while float(battle.call("get_elapsed")) < seconds and not battle.call("is_finished"):
		battle.call("tick", 0.1)
		units = battle.call("get_units")
		soldiers.update(battle, units, 0.1, [])
		_update_effects(0.1)


func _refresh_view(force: bool, delta: float = 0.0) -> void:
	units = battle.call("get_units")
	var running: bool = not paused and not battle.call("is_finished")
	soldiers.update(battle, units, delta * speed if running else 0.0, selected)
	_update_effects(delta * speed if running else 0.0)
	if siege_view != null:
		siege_view.update(battle.call("get_siege"), units)
	var banner_scale := _banner_scale()
	# Les drapeaux se présentent de trois quarts à la caméra (lisibles sans être des panneaux).
	var cam_yaw := camera_rig.yaw + PI * 0.5 + 0.35
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
		node.rotation.y = cam_yaw
		var routing := str(unit["state"]) == "routing"
		if routing != bool(banner["routing"]):
			banner["routing"] = routing
			(banner["flag_mat"] as ShaderMaterial).set_shader_parameter("routing", routing)
		if ring.visible:
			ring.position = pos + Vector3(0, 0.6, 0)
			ring.rotation = Vector3(0, float(unit["facing"]), 0)
			var size := Vector2(float(unit["width"]) + 3.0, float(unit["depth"]) + 3.0)
			if not ring.has_meta("size") or (ring.get_meta("size") as Vector2).distance_to(size) > 0.5:
				ring.set_meta("size", size)
				ring.mesh = BattleMeshes.outline(size.x, size.y, 0.45)
	_update_markers(banner_scale)
	_hud_timer -= delta
	if force or _hud_timer <= 0.0:
		_hud_timer = 0.1
		hud.set_clock(float(battle.call("get_elapsed")), speed, paused)
		hud.set_balance(side_names[player_side], int(battle.call("get_strength", player_side)), side_names[enemy_side], int(battle.call("get_strength", enemy_side)))
		if siege_view != null:
			hud.set_siege_status(siege_status(battle.call("get_siege")))
			_check_sortie()
		hud.update_cards(units, player_side, selected)
		hud.minimap.update(units, camera_frame())
		var events: Array = battle.call("get_events")
		if not events.is_empty():
			hud.add_events(events)


## Ligne d'état du siège pour le HUD : murailles, brèches, porte, tenue de la place.
static func siege_status(siege: Dictionary) -> String:
	if siege.is_empty():
		return ""
	var breaches := 0
	var gate_open := false
	for piece in siege.get("pieces", []):
		if not bool(piece["intact"]):
			if str(piece["kind"]) == "gate":
				gate_open = true
			else:
				breaches += 1
	var text := "Murailles %d %%" % int(round(float(siege.get("integrity", 1.0)) * 100.0))
	text += " · %d brèche(s)" % breaches
	var burning := int(siege.get("houses_burning", 0))
	if burning > 0:
		text += " · %d maison(s) en feu" % burning
	text += " · porte %s" % ("enfoncée" if gate_open else ("en feu" if str((siege.get("gate_fire", {}) as Dictionary).get("state", "")) == "burning" else "tenue"))
	var hold := float(siege.get("hold_time", 0.0))
	if hold > 0.0:
		text += " · place centrale tenue %d / %d s" % [int(hold), int(float(siege.get("hold_to_win", 60.0)))]
	if bool(siege.get("sortie", false)):
		text += " · sortie de la garnison !"
	return text


## B2 : repères 2D au-dessus des troupes, sous les panneaux du HUD (premier enfant de sa racine).
func _build_markers() -> void:
	markers = BattleUnitMarkers.new()
	hud.root.add_child(markers)
	hud.root.move_child(markers, 0)
	markers.setup(side_colors, player_side)
	markers.marker_clicked.connect(_on_card_clicked)
	markers.marker_right_clicked.connect(_on_marker_right_clicked)


## Ancre écran de chaque repère : au-dessus du drapeau 3D du régiment.
func _update_markers(banner_scale: float) -> void:
	if markers == null:
		return
	var anchors := {}
	if markers.visible:
		var camera := camera_rig.camera
		var screen := get_viewport().get_visible_rect().grow(60.0)
		for unit in units:
			if not bool(unit["present"]):
				continue
			var top := Vector3(float(unit["x"]), float(unit["y"]) + (BANNER_HEIGHT + 0.6) * banner_scale, float(unit["z"]))
			if camera.is_position_behind(top):
				continue
			var point := camera.unproject_position(top)
			if screen.has_point(point):
				anchors[int(unit["id"])] = point
	markers.update(units, anchors, selected, camera_rig.distance)


## Clic droit sur le repère d'un ennemi : la sélection l'attaque (au pas de course).
func _on_marker_right_clicked(unit_id: int) -> void:
	if selected.is_empty() or deployment != null and deployment.active:
		return
	for unit in units:
		if int(unit["id"]) == unit_id and str(unit["side"]) == enemy_side:
			issue({"type": "attack", "units": selected.duplicate(), "target": unit_id, "run": true})
			return


func _banner_scale() -> float:
	return clampf(camera_rig.distance * 0.014, 0.8, 9.0)


## B2 / T2 : écran de fin mis en scène (verdict, écus, pertes par régiment, mentions).
func _show_end() -> void:
	finished_shown = true
	var outcome: Dictionary = battle.call("get_outcome")
	var sides := {}
	for side in ["attacker", "defender"]:
		sides[side] = {"name": side_names[side], "faction": str((setup[side] as Dictionary).get("faction", "")), "color": side_colors[side]}
	# UB1 : le résultat est appliqué dès la fin, pour montrer ses suites (captifs, rançons,
	# expérience) sur l'écran de fin ; « Retour à la campagne » ne fait plus que rendre la main.
	var side_setup: Dictionary = setup[player_side]
	var general: Variant = side_setup.get("general", null)
	var general_id := str(general.get("character", "")) if general is Dictionary else ""
	var before := BattleAftermath.snapshot(campaign_sim, str(side_setup.get("army", "")), general_id)
	_resolve_now()
	var aftermath := {}
	if bool(_resolution.get("ok", false)):
		aftermath = BattleAftermath.diff(before, BattleAftermath.snapshot(campaign_sim, str(side_setup.get("army", "")), general_id))
	result_screen = BattleResultScreen.new()
	hud.root.add_child(result_screen)
	result_screen.return_pressed.connect(_on_return)
	result_screen.show_result(hud.title_label.text, player_side, sides, battle.call("get_units"), outcome, aftermath)


var _resolution: Dictionary = {}


## Applique le résultat (`resolve_battle`) une seule fois.
func _resolve_now() -> void:
	if resolved:
		return
	resolved = true
	_resolution = {"ok": false, "error": "bataille non terminée", "events": []}
	if battle != null and battle.call("is_finished") and campaign_sim != null and not padded:
		_resolution = campaign_sim.call("resolve_battle", battle_index, battle.call("get_outcome"))
		if not _resolution.get("ok", false):
			push_error("BattleScene: resolve_battle refused: %s" % _resolution.get("error", "?"))


## « Retour à la campagne » : applique le résultat s'il ne l'est pas encore, puis rend la main.
func _on_return() -> void:
	if _returned:
		return
	_returned = true
	_resolve_now()
	var result := _resolution
	if _audio_director != null:  # B3 : la carte retrouve sa musique de contexte
		_audio_director.call("refresh_context")
	returned.emit(result)
	if standalone:
		get_tree().change_scene_to_file("res://scenes/start_menu.tscn")


# --- Entrées --------------------------------------------------------------------------


func _unhandled_input(event: InputEvent) -> void:
	if battle == null:
		return
	if event is InputEventKey and event.pressed and not event.echo:
		var key := event as InputEventKey
		# F5b : chiffres de la rangée (touche physique, AZERTY compris) = groupes de sélection.
		if key.physical_keycode >= KEY_1 and key.physical_keycode <= KEY_9:
			handle_group_key(int(key.physical_keycode - KEY_0), key.ctrl_pressed or key.meta_pressed)
			return
		match event.keycode:
			KEY_ENTER, KEY_KP_ENTER:
				if deployment != null:
					deployment.finish()
			KEY_SPACE:
				_toggle_pause()
			KEY_PLUS, KEY_EQUAL, KEY_KP_ADD:
				_on_speed_pressed(mini(SPEEDS.find(speed) + 1, SPEEDS.size() - 1))
			KEY_MINUS, KEY_KP_SUBTRACT:
				_on_speed_pressed(maxi(SPEEDS.find(speed) - 1, 0))
			KEY_F1:
				hud.toggle_help()
			KEY_U:
				if markers != null:
					markers.toggle()
			KEY_F:
				_on_command("formation")
			KEY_G:
				_on_command("fire_at_will")
			KEY_H:
				_on_command("halt")
			KEY_C:
				_toggle_camera_follow()
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
	elif event is InputEventMouseMotion and _left_press.x < 0.0 and markers != null:
		# B2 : survol d'une troupe sur le terrain = repère mis en évidence.
		var hover_at := (event as InputEventMouseMotion).position
		var hover := pick_unit(hover_at, player_side)
		markers.world_hover = hover if hover >= 0 else pick_unit(hover_at, enemy_side)
	elif event is InputEventMouseMotion and _left_press.x >= 0.0:
		var motion := event as InputEventMouseMotion
		var rect := Rect2(_left_press, motion.position - _left_press).abs()
		_drag_rect.visible = rect.size.length() > 8.0
		_drag_rect.position = rect.position
		_drag_rect.size = rect.size


func _toggle_pause() -> void:
	paused = not paused


## Boutons de vitesse du HUD (F5b) : -1 = pause (bascule), 0..2 = SPEEDS[index] et reprise.
func _on_speed_pressed(index: int) -> void:
	if index < 0:
		_toggle_pause()
	else:
		speed = SPEEDS[index]
		paused = false
	hud.set_clock(float(battle.call("get_elapsed")), speed, paused)


## Ctrl+n (Cmd+n sous macOS) : enregistre la sélection ; n : la rappelle, et un second appui
## rapide centre la caméra sur le groupe.
func handle_group_key(number: int, save: bool) -> void:
	if save:
		hud.groups.save(number, selected)
		return
	var ids := hud.groups.recall(number, units)
	if ids.is_empty():
		return
	var again := selected == ids and Time.get_ticks_msec() - _last_group_ms < 600
	_last_group_ms = Time.get_ticks_msec()
	selected = ids
	if again:
		var center := Vector3.ZERO
		for unit in units:
			if ids.has(int(unit["id"])):
				center += Vector3(float(unit["x"]), 0, float(unit["z"]))
		camera_rig.look_at_point(center / ids.size(), camera_rig.distance, camera_rig.yaw)


## Cadre de la caméra au sol (x, z) : les quatre coins de l'écran projetés sur le terrain.
func camera_frame() -> PackedVector2Array:
	var frame := PackedVector2Array()
	var screen := get_viewport().get_visible_rect().size
	for corner in [Vector2(0, 0), Vector2(screen.x, 0), screen, Vector2(0, screen.y)]:
		var point := ground_point(corner)
		frame.append(Vector2(point.x, point.z))
	return frame


func _on_minimap_clicked(world: Vector2) -> void:
	camera_rig.look_at_point(Vector3(world.x, 0, world.y), camera_rig.distance, camera_rig.yaw)


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
	if deployment != null and deployment.active:
		_deploy_selection(press, position)
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
	UiSounds.play_order_result(result)  # UB1 / U13 : ordre donné ou refusé
	if not result.get("ok", false):
		hud.add_events([{"time": battle.call("get_elapsed"), "text_fr": "Ordre refusé : %s" % result.get("error", "?")}])
	return result


func _on_card_clicked(unit_id: int, additive: bool) -> void:
	if not additive:
		selected.clear()
	if not selected.has(unit_id):
		selected.append(unit_id)


## UB1 : clic sur le sceau du chef = sélectionner sa garde ; double clic = y centrer la caméra.
func _on_leader_clicked(double: bool) -> void:
	var id := hud.leader_unit_id()
	if id < 0:
		return
	if double:
		_on_card_double_clicked(id)
	else:
		_on_card_clicked(id, false)


## B3 / T6 : double-clic sur une carte d'unité = centrer la caméra sur ce régiment (comme TW).
func _on_card_double_clicked(unit_id: int) -> void:
	for unit in units:
		if int(unit["id"]) == unit_id and bool(unit["present"]):
			camera_rig.look_at_point(Vector3(float(unit["x"]), 0.0, float(unit["z"])), camera_rig.distance, camera_rig.yaw)
			return


## Touche `C` : verrouille la caméra sur la sélection (premier régiment présent) ou, à défaut, le
## général du joueur ; un second appui pendant un suivi le libère.
func _toggle_camera_follow() -> void:
	if camera_rig.is_following():
		camera_rig.stop_follow()
		return
	var id := _follow_candidate()
	if id >= 0:
		camera_rig.follow_unit(id, _unit_world_position)


func _follow_candidate() -> int:
	for unit in units:
		if selected.has(int(unit["id"])) and bool(unit["present"]):
			return int(unit["id"])
	for unit in units:
		if str(unit["side"]) == player_side and bool(unit.get("is_general", false)) and bool(unit["present"]):
			return int(unit["id"])
	return -1


## Callable passée à `BattleCamera.follow_unit` : position au sol du régiment, `null` s'il a
## quitté le champ (le suivi se libère alors de lui-même).
func _unit_world_position(id: int) -> Variant:
	for unit in units:
		if int(unit["id"]) == id and bool(unit["present"]):
			return Vector3(float(unit["x"]), 0.0, float(unit["z"]))
	return null


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
		elif arg.begins_with("--bench-at="):
			_bench_at = float(arg.trim_prefix("--bench-at="))
		elif arg == "--benchmark":
			_benchmark = true
			autoplay = true
		elif arg == "--autoplay":
			autoplay = true
		elif arg == "--deploy-shot":
			_deploy_shot = true
		elif arg == "--siege":
			siege_demo = true
		elif arg.begins_with("--camera="):
			_camera_override = arg.trim_prefix("--camera=")
		elif arg == "--result-shot":
			_result_shot = true
		elif arg.begins_with("--shot-at="):
			_shot_at = float(arg.trim_prefix("--shot-at="))
		elif arg == "--no-effects":
			_no_effects = true
		elif arg == "--closeup":
			_closeup = true
		elif arg.begins_with("--weather="):
			_weather_override = arg.trim_prefix("--weather=")
	if _screenshot_path != "":
		call_deferred("_stage_screenshot")


## Capture : IA des deux camps jusqu'au premier contact (+ 12 s), sélection de deux régiments
## du joueur, caméra sur la mêlée, capture après quelques images.
func _stage_screenshot() -> void:
	if battle == null:
		get_tree().quit(1)
		return
	camera_rig.edge_pan_enabled = false
	if _deploy_shot:
		await _stage_deploy_screenshot()
		return
	if siege_view != null:
		await _stage_siege_screenshot()
		return
	if _result_shot:
		await _stage_result_screenshot()
		return
	var contact_time := -1.0
	for _i in 3000:
		battle.call("tick", 0.1)
		# Les soldats tombés pendant l'avance rapide laissent aussi leurs cadavres.
		units = battle.call("get_units")
		soldiers.update(battle, units, 0.1, [])
		_update_effects(0.1)
		if _shot_at > 0.0:
			if float(battle.call("get_elapsed")) >= _shot_at:
				break
			continue
		if contact_time < 0.0:
			for unit in battle.call("get_units"):
				if str(unit["state"]) == "melee":
					contact_time = float(battle.call("get_elapsed"))
					break
		elif battle.call("is_finished"):
			break
		elif _closeup:
			# A1-06 : cliché au choc (1 s après le contact), ou dès que la mêlée cesse (une charge
			# met souvent l'adversaire en déroute en quelques secondes).
			var since := float(battle.call("get_elapsed")) - contact_time
			if since >= 1.0 or not _melee_ongoing(units):
				break
		elif float(battle.call("get_elapsed")) > contact_time + 12.0:
			break
	paused = true
	print("BattleScene: capture at %.0f s, %d corpses, %d missiles" % [float(battle.call("get_elapsed")), soldiers.corpse_count, effects.launched if effects != null else 0])
	units = battle.call("get_units")
	var focus := Vector3.ZERO
	var n := 0
	for unit in units:
		if str(unit["state"]) == "melee":
			focus += Vector3(float(unit["x"]), 0, float(unit["z"]))
			n += 1
	if _closeup:
		var shot := _closeup_shot(units)
		camera_rig.look_at_point(shot["focus"], 26.0, float(shot["yaw"]))
	elif n > 0:
		focus /= n
		camera_rig.look_at_point(focus + Vector3(0, 0, -25 if player_side == "attacker" else 25), 120.0, (PI if player_side == "attacker" else 0.0) + 0.5)
	for unit in units:
		if str(unit["side"]) == player_side and bool(unit["present"]) and selected.size() < 2:
			selected.append(int(unit["id"]))
	if markers != null and not selected.is_empty() and not _closeup:
		markers.world_hover = selected[0]  # B2 : la capture montre aussi le nom au survol
	_refresh_view(true)
	_apply_camera_override()
	# B4 : laisser la poussière se lever (les particules vivent en temps réel, bataille en pause).
	for _i in 150 if effects != null else 40:
		await get_tree().process_frame
	_take_screenshot(_screenshot_path, true)


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
func _closeup_shot(p_units: Array) -> Dictionary:
	var best := INF
	var focus := Vector3.ZERO
	var yaw := 0.0
	for unit in p_units:
		if str(unit["side"]) != player_side or not bool(unit["present"]):
			continue
		for other in p_units:
			if str(other["side"]) == player_side or not bool(other["present"]):
				continue
			var a := Vector2(float(unit["x"]), float(unit["z"]))
			var b := Vector2(float(other["x"]), float(other["z"]))
			if a.distance_to(b) > 150.0:
				continue
			var in_melee := ["melee", "charging"].has(str(unit["state"])) or str(other["state"]) == "melee"
			var ours := soldiers.soldier_positions(int(unit["id"]), 48)
			var theirs := soldiers.soldier_positions(int(other["id"]), 48)
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


## Capture de siège : l'assaut jusqu'aux premières échelles (+ 10 s) ou 4 min, vue sur le front
## des murailles depuis l'extérieur.
func _stage_siege_screenshot() -> void:
	var climb_time := -1.0
	for _i in 2400:
		battle.call("tick", 0.1)
		units = battle.call("get_units")
		soldiers.update(battle, units, 0.1, [])
		_update_effects(0.1)
		var elapsed := float(battle.call("get_elapsed"))
		if climb_time < 0.0:
			for unit in battle.call("get_units"):
				if int(unit.get("climbing", -1)) >= 0 or bool(unit.get("on_wall", false)) and str(unit["side"]) == "attacker":
					climb_time = elapsed
					break
		elif elapsed > climb_time + 10.0:
			break
		if battle.call("is_finished"):
			break
	paused = true
	units = battle.call("get_units")
	var siege: Dictionary = battle.call("get_siege")
	var gate: Dictionary = siege["pieces"][int(siege["gate"])]
	var focus := Vector3((gate["a"] as Vector2).x, 0, (gate["a"] as Vector2).y)
	camera_rig.look_at_point(focus + Vector3(-10, 0, -30), 105.0, PI + 0.5)
	for unit in units:
		if str(unit["side"]) == player_side and bool(unit["present"]) and selected.size() < 2 and int(unit.get("climbing", -1)) >= 0:
			selected.append(int(unit["id"]))
	_refresh_view(true)
	_apply_camera_override()
	for _i in 40:
		await get_tree().process_frame
	_take_screenshot(_screenshot_path, true)


## Capture `--result-shot` : bataille jouée par l'IA jusqu'au bout, écran de fin affiché.
func _stage_result_screenshot() -> void:
	for _i in 36000:
		battle.call("tick", 0.1)
		if battle.call("is_finished"):
			break
	soldiers.update(battle, battle.call("get_units"), 0.1, [])
	_refresh_view(true)
	if not finished_shown:
		_show_end()
	for _i in 30:
		await get_tree().process_frame
	_take_screenshot(_screenshot_path, true)


## Capture : `--camera=x,z,distance,lacet_en_degrés` place la caméra (réglage du rendu).
func _apply_camera_override() -> void:
	if _camera_override == "":
		return
	var parts := _camera_override.split(",")
	if parts.size() < 4:
		return
	camera_rig.look_at_point(Vector3(float(parts[0]), 0, float(parts[1])), float(parts[2]), deg_to_rad(float(parts[3])))


func _take_screenshot(path: String, quit_after: bool) -> void:
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var err := image.save_png(path)
	print("BattleScene: screenshot %s (%s)" % [path, error_string(err)])
	if quit_after:
		get_tree().quit(0 if err == OK else 1)


## « de » élidé devant voyelle (« d'Île-de-France », « de Guyenne ») ; même règle que
## `events::de` côté Rust.
static func de(name: String) -> String:
	if name != "" and "AEIOUYÉÈÊÂÎÔaeiouyéèêâîô".contains(name[0]):
		return "d'" + name
	return "de " + name


# --- F5c : déploiement et sortie de la garnison ------------------------------------------


## Phase de déploiement pour toute bataille du joueur (champ et siège) ; sautée quand l'IA
## joue les deux camps (`--autoplay`, `--screenshot`, smoke), sauf `--deploy-shot`.
func _open_deployment() -> void:
	if autoplay and not _deploy_shot:
		return
	deployment = DeploymentController.new()
	deployment.name = "Deployment"
	add_child(deployment)
	if not deployment.open(self):
		deployment.queue_free()
		deployment = null


func _deploy_selection(press: Vector2, release: Vector2) -> void:
	var p1 := ground_point(release)
	var p0 := _right_press_ground if press.x >= 0.0 and press.distance_to(release) > 20.0 else p1
	deployment.place(selected.duplicate(), p0, p1, camera_rig.camera.global_position)
	_refresh_view(true)


## Sortie de la garnison (F5a) : message éphémère une fois, et mention dans la ligne du siège.
func _check_sortie() -> void:
	if _sortie_shown or not bool(battle.call("get_siege").get("sortie", false)):
		return
	_sortie_shown = true
	hud.show_toast("La garnison ouvre ses portes et fait une sortie !", player_side == "attacker")


## Capture `--deploy-shot` : phase de déploiement ouverte, deux régiments du joueur rangés en
## ligne dans la zone, un placement refusé (toast), vue plongeante sur la zone.
func _stage_deploy_screenshot() -> void:
	if deployment == null:
		get_tree().quit(1)
		return
	var zone: Dictionary = deployment.zone
	var center := Vector3((float(zone["x0"]) + float(zone["x1"])) * 0.5, 0, (float(zone["z0"]) + float(zone["z1"])) * 0.5)
	for unit in units:
		if str(unit["side"]) == player_side and bool(unit["present"]) and selected.size() < 2:
			selected.append(int(unit["id"]))
	print("BattleScene: deployment zone %s, player side %s" % [zone, player_side])
	var ahead := 1.0 if player_side == "attacker" else -1.0
	var cam := center + Vector3(0, 300, -400 * ahead)
	deployment.place(selected.duplicate(), center + Vector3(-60, 0, 0), center + Vector3(60, 0, 0), cam)
	var outside := Vector3(center.x, 0, (float(zone["z1"]) + 150.0) if ahead > 0.0 else (float(zone["z0"]) - 150.0))
	deployment.place(selected.slice(0, 1), outside, outside, cam)
	camera_rig.look_at_point(center + Vector3(0, 0, 60 * ahead), 420.0, (PI if ahead > 0.0 else 0.0) + 0.35)
	_refresh_view(true)
	_apply_camera_override()
	for _i in 40:
		await get_tree().process_frame
	_take_screenshot(_screenshot_path, true)
