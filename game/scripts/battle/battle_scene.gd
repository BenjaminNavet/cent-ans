class_name BattleScene
extends Node3D

const _TerrainTip := preload("res://scripts/battle/battle_terrain_tip.gd")

## Scène de bataille 3D temps réel avec pause (spec M7 § 4). Toute la simulation est dans
## `BattleSim` (Rust) : la scène avance le temps, affiche terrain, soldats (MultiMesh par camp et
## par famille), bannières, HUD, et convertit clics et touches en commandes.
##
## Lancement : depuis la carte (`configure(campaign_sim, index, seed)` avant `add_child`), ou seule
## (`godot --path game res://scenes/battle/battle.tscn`) : une campagne France est créée et une
## bataille France–Angleterre mise en scène (`debug_stage_battle`).
## SC BT12 : la scène compose des composants dédiés : `banners` (BattleBannerLayer : drapeaux),
## `replay` (BattleReplayController : rejeu EP13), `audio` (BattleAudioDirector : musique, sons,
## répliques) et `capture` (BattleCaptureStage : `--screenshot=` et cadrages).
## Options (après `--`) : `--screenshot=<png>` (joue la bataille jusqu'au contact, capture, quitte),
## `--units=<n>` (complète chaque camp à n régiments, essai sans retour campagne), `--autoplay` (IA des deux camps ; écrit une ligne « autoplay: seed=… winner=… end=… t=… losses=… » à la fin), `--seed=<n>` (graine de la bataille),
## `--siege` (démo autonome : assaut français de la Guyenne, bataille de siège M8),
## `--closeup` (capture : caméra rapprochée sur la mêlée, à `--closeup-distance=<m>`, 26 par
## défaut), `--weather=<clear|fog|rain|snow>`
## (rendu seulement : force l'aspect de la météo, la simulation garde la sienne),
## `--camera=x,z,distance,lacet` (capture : position de caméra imposée), `--deploy-shot` (avec
## `--screenshot=` : capture de la phase de déploiement, F5c), `--result-shot` (avec
## `--screenshot=` : bataille jouée jusqu'au bout, capture de l'écran de fin, B2),
## `--shot-at=<s>` (capture : à cet instant de la bataille plutôt qu'au premier contact, B4),
## `--standard-shot=<foot|mounted|line|fallen|captured>` (capture EP5 : gros plan d'un
## porte-étendard à pied ou à cheval, ligne de bataille et ses étendards au loin, étendard tombé,
## étendard pris porté par le vainqueur) ; `--standard-side=<attacker|defender>` choisit le camp
## cadré (DA1b : étendards aux armes de la maison du général ennemi).
## `--horizon-province=<id>`, `--panorama=<id>` (captures EP2), `--hour=<dawn|morning|midday|
## afternoon|dusk|night|h>` (heure de début : phase ou heure décimale, règle du cœur).

signal returned(result: Dictionary)

## Ralenti ×0,5 ajouté (le − descend jusque-là) ; index par défaut sur ×1 (`speed = 1.0`).
const SPEEDS := [0.5, 1.0, 2.0, 4.0]
## Vitesses du rejeu (barre de rejeu, + / −).
const REPLAY_SPEEDS := [1.0, 2.0, 4.0, 8.0]
const PICK_RADIUS_PX := 26.0
## Barre des ordres du chef (F10b).
const LEADER_ORDERS_BAR := preload("res://scripts/battle/leader_orders_bar.gd")
## Musique dynamique par intensité (B3 / T4).

var campaign_sim: Object = null
var battle_index: int = -1
var battle_seed: int = 1
var battle: Object = null  # BattleSim
var setup: Dictionary = {}
var player_side: String = "attacker"
var enemy_side: String = "defender"
var side_colors: Dictionary = {}
var _side_houses: Dictionary = {}  # Maison du général par camp (écus, étendards)
var side_names: Dictionary = {}
var units: Array = []
## État du siège lu par `_refresh_view` pour l'image courante (vide hors siège).
var _frame_siege: Dictionary = {}
var selected: Array[int] = []
var paused: bool = false
var speed: float = 1.0
var autoplay: bool = false
var padded: bool = false
var finished_shown: bool = false
## Secondes d'acclamation du vainqueur montrées avant l'écran de fin (0 sans affichage,
## en banc d'essai) ; `_end_wait` les compte.
const VICTORY_HOLD := 3.0
var _end_wait: float = 0.0
var resolved: bool = false
var _returned: bool = false  # « Retour à la campagne » déjà émis
var standalone: bool = false
var siege_view: BattleSiege = null  # batailles de siège (M8)
var engines_fx: SiegeEnginesFx = null  # Engins de siège animés
var assault_fx: SiegeAssaultFx = null  # Engins, échelles, porte, huile (événements du cœur)
var _siege_engines := ""  # `--siege-engines=` (captures, banc d'essai)
var siege_demo: bool = false
## Province de la démo de siège (`--siege-province=prov_ile_de_france` : Paris).
var siege_province: String = "prov_guyenne"
## Ville assiégée dans son plan (`--siege-landmark=avignon`, guerre déclarée au détenteur si
## besoin) et faction de l'assiégeant (`--siege-attacker=fac_england`).
var siege_landmark: String = ""
var siege_attacker: String = "fac_france"
## Options d'une démo lancée depuis le menu (`BattleDemosMenu`), lues comme la ligne de
## commande puis oubliées.
static var demo_args: PackedStringArray = []
## Bataille personnalisée composée dans `CustomBattleScreen` (configuration de
## `BattleSim.setup_custom`), lue au `_ready` puis oubliée. `--custom-battle` rejoue la dernière
## composition gardée dans les réglages (captures, bancs).
static var custom_config: Dictionary = {}
var _custom: Dictionary = {}
## Bataille-prologue guidée (`BattlePrologue.launch`), lue au `_ready` avec
## `custom_config` puis oubliée ; pas de déploiement, de discours ni de conseiller spontané.
static var prologue_data: Dictionary = {}
var _prologue_data: Dictionary = {}
var prologue: BattlePrologue = null
var landmark_town: LandmarkSiegeTown = null

var _mm: Dictionary = {}  # unit id -> MultiMeshInstance3D (BattleSoldiers.layers)
var soldiers: BattleSoldiers = null
var effects: BattleEffects = null  # Poussière, traits, fumée des bombardes, gués
var _weather_key: String = "clear"
var blood: BattleBlood = null  # Sang au sol (réglage « Sang »)
var grass_flatten: BattleGrassFlatten = null  # Herbe couchée et tachée de sang
var standards: BattleStandards = null  # Vent, porte-étendards
var speech: BattleSpeech = null  # Discours du général avant la bataille
var enemy_speech: BattleSpeech = null  # Discours du général adverse, après celui du joueur
var _unit_size_override: float = -1.0  # `--unit-size=<k>` (banc d'essai BV1)
var _blood_override: int = -1  # `--blood=<0|1|2>`
var banners: BattleBannerLayer = null  # drapeaux-repères des régiments
var markers: BattleUnitMarkers = null  # Bannières flottantes (repères 2D)
var result_screen: BattleResultScreen = null  # Écran de fin
## Contours de formation (décales), remplacent l'anneau jaune de sélection.
var outlines: BattleFormationOutline = null
var _hovered_ids: Array[int] = []  # régiments survolés (terrain, repère), réutilisé
## Aperçu du trajet, curseur contextuel, carte du HUD survolée (-1 : aucune).
var path_preview: BattlePathPreview = null
var cursor: BattleCursor = null
## Portée au sol des tireurs sélectionnés ou survolés, comparaison au survol d'un ennemi.
var range_arc: BattleRangeArc = null
var compare_panel: BattleComparePanel = null
## Sélecteur de formation de groupe (bas droite, au-dessus du bandeau des cartes).
var formation_picker: BattleFormationPicker = null
var card_hover := -1
## Infobulle « file d'ordres pleine » (Maj tenue).
var queue_tip: BattleQueueTip = null
## Étiquette du terrain (décor) sous le curseur.
var terrain_tip = null
## Dernier `hover_context` du cœur et nombre d'appels (tests : au plus un par image).
var last_hover: Dictionary = {}
var hover_calls := 0
var _hover_mouse := Vector2(-1, -1)
var _hover_dirty := false
var _hover_key := ""
var _drag_rect: ColorRect  # Rectangle de sélection, lu et positionné par `input`
var _hud_timer: float = 0.0
var capture: BattleCaptureStage = null  # SC BT12 : `--screenshot=` et cadrages de capture
var _pad_units: int = 0
var _seed_arg: int = -1  # `--seed=<n>` : graine imposée (sonde d'issue `--autoplay`, reproductible)
var _scale_tier: String = ""  # Palier d'échelle forcé (`--scale=`), sinon selon l'effectif
var _weather_override: String = ""
var deployment: DeploymentController = null  # Phase de déploiement du joueur
var _sortie_shown: bool = false
var music: BattleMusicDirector:  # Musique dynamique par intensité
	get:
		return audio.music if audio != null else null
var battle_audio: BattleAudio:  # Sons spatialisés (mêlée, volées, siège, météo)
	get:
		return audio.battle_audio if audio != null else null
var voices: BattleVoices:  # Répliques des régiments
	get:
		return audio.voices if audio != null else null
var audio: BattleAudioDirector = null  # SC BT12 : musique, sons spatialisés, répliques
var replay: BattleReplayController = null  # SC BT12 : rejeu d'après bataille (EP13)
var staging: BattleStaging = null  # Heure, nuages, fumées, oiseaux, plan cinématique
var _hour_override: String = ""  # `--hour=`
var _title_text: String = ""
## Bataille historique (`--historical=<id>`, `--historical-side=<attacker|defender>`, vide :
## IA contre IA) ; `historical` = `BattleSim.get_historical()` (vide hors carte historique).
var _historical: String = ""
var _historical_side: String = ""
var historical: Dictionary = {}
var _sim_weather: String = ""
var _weather_poll: float = 0.0
var _weather_text: String = ""
var _tod_key: String = ""
var _tod_clock: String = ""  # Dernière heure affichée au bandeau (« Midi, 11 h 00 »)
## Rejeu d'après bataille. `--replay=<fichier>` (menu « Rejeux ») ou « Revoir la bataille »
## sur l'écran de fin : le cœur re-simule la bataille enregistrée, la scène la montre sans ordre.
var replay_mode: bool = false
var _step_thread_on: bool = false
## RJ-b : figurines et régiments interpolés entre deux pas de simulation (démarche continue) ;
## coupé en headless (tests).
var _pose_lerp_on: bool = false
var replay_bar: BattleReplayBar = null
var replay_error: String = ""
var replay_saved_path: String = ""  # fichier écrit à la fin de la bataille (vide : non enregistré)
var _replay_path: String = ""
var _leader_bar: CanvasLayer = null
## Dictionnaires passés à `issue()`, pour le test d'équivalence des entrées
## (`cb0_input_equivalence_test.gd`). Rempli seulement si `log_orders_for_test` est vrai (mis par
## le test) ou sous la fonctionnalité moteur « test ».
var issued_log: Array = []
var log_orders_for_test: bool = false

@onready var terrain: BattleTerrain = $Terrain
@onready var camera_rig: BattleCamera = $CameraRig
@onready var hud: BattleHud = $HUD
@onready var world_env: WorldEnvironment = $WorldEnvironment
@onready var sun: DirectionalLight3D = $Sun
## Entrées (clics, glisser, touches, groupes), nœud enfant créé au premier `_ready`.
var input: BattleInput = null
## Vue tactique (Tab), nœud enfant créé au premier `_ready` comme `input`.
var tactical_view: BattleTacticalView = null


## À appeler avant `add_child` quand la bataille vient de la campagne.
func configure(p_campaign_sim: Object, index: int, seed: int) -> void:
	campaign_sim = p_campaign_sim
	battle_index = index
	battle_seed = seed


func _ready() -> void:
	# SC BT12 : composants (captures, rejeu, son, drapeaux), créés avant la ligne de commande.
	capture = BattleCaptureStage.new(self)
	replay = BattleReplayController.new(self)
	audio = BattleAudioDirector.new(self)
	banners = BattleBannerLayer.new()
	banners.attach(self)
	_parse_cmdline()
	# Nœud d'entrées, connecté aux appels encore portés par la scène (ordres, rendu, pont).
	input = BattleInput.new()
	input.name = "BattleInput"
	input.scene = self
	add_child(input)
	input.command_requested.connect(_on_input_command)
	input.selection_changed.connect(_on_input_selection_changed)
	input.camera_focus_requested.connect(_on_input_camera_focus)
	input.pause_toggled.connect(_toggle_pause)
	hud.quit_confirmed.connect(quit_battle)
	hud.quit_cancelled.connect(func() -> void: paused = _paused_before_quit_menu)
	input.speed_step.connect(_on_speed_step)
	input.help_toggled.connect(hud.toggle_help)
	input.markers_toggled.connect(_on_input_markers_toggled)
	input.screenshot_requested.connect(_on_input_screenshot_requested)
	input.tactical_view_toggled.connect(_on_input_tactical_view_toggled)
	_setup_decor_hover()
	# Vue tactique (Tab).
	tactical_view = BattleTacticalView.new()
	tactical_view.name = "BattleTacticalView"
	tactical_view.scene = self
	add_child(tactical_view)
	hud.card_clicked.connect(_on_card_clicked)
	hud.card_double_clicked.connect(_on_card_double_clicked)
	hud.card_hovered.connect(func(id: int) -> void: card_hover = id)  # Contour au survol
	hud.command_pressed.connect(input._on_command)
	hud.ability_pressed.connect(input.use_card_ability)  # Bouton de capacité d'une carte
	hud.speed_pressed.connect(_on_speed_pressed)
	hud.minimap_clicked.connect(_on_minimap_clicked)
	hud.minimap_order.connect(func(world: Vector2, queued: bool) -> void: input.order_move_to(Vector3(world.x, 0, world.y), queued))
	hud.leader_clicked.connect(_on_leader_clicked)  # Sceau du chef
	hud.alerts_column.pinged.connect(_on_alert_pinged)  # CB5
	_drag_rect = ColorRect.new()
	_drag_rect.color = Color(0.95, 0.8, 0.3, 0.18)
	_drag_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_drag_rect.visible = false
	hud.add_child(_drag_rect)
	if _replay_path != "":
		# Rejeu d'un fichier (menu « Rejeux »), hors campagne.
		standalone = true
		if not replay.begin(_replay_path):
			push_error("BattleScene: replay %s failed: %s" % [_replay_path, replay_error])
			replay.failed()
		return
	if campaign_sim == null and not custom_config.is_empty():
		_custom = custom_config
	custom_config = {}
	if campaign_sim == null and not prologue_data.is_empty() and not _custom.is_empty():
		_prologue_data = prologue_data
	prologue_data = {}
	if campaign_sim == null and not _custom.is_empty():
		# Bataille personnalisée (menu « Bataille personnalisée »), hors campagne.
		standalone = true
		if not begin_custom(_custom):
			push_error("BattleScene: custom battle setup failed")
		elif not _prologue_data.is_empty():
			_start_prologue()
		return
	if campaign_sim == null and _historical != "":
		# Carte historique jouée hors campagne (menu « Batailles historiques »).
		standalone = true
		if not begin_historical():
			push_error("BattleScene: historical battle %s failed" % _historical)
		return
	if campaign_sim == null:
		standalone = true
		if not _stage_standalone():
			push_error("BattleScene: cannot stage a demo battle")
			# Un banc d'essai qui ne peut pas se lancer doit échouer bruyamment (JSON +
			# code de sortie ≠ 0) plutôt que laisser une fenêtre ouverte sans jamais quitter
			# (l'une des causes des exécutions « sans résultat », cf. docs/archive/chantiers.md).
			return
	if not begin():
		push_error("BattleScene: battle setup failed")


## Démo autonome : campagne France 1337, principale armée française contre anglaise.
func _stage_standalone() -> bool:
	if not ClassDB.class_exists("CampaignSim"):
		return false
	var sim_facade := get_node_or_null("/root/SimFacade")
	var sim: Object = null
	if sim_facade != null and sim_facade.is_real and sim_facade.sim != null and int(sim_facade.sim.call("get_turn")) >= 0:
		sim = sim_facade.sim
	else:
		sim = ClassDB.instantiate("CampaignSim")
		if not sim.call("new_campaign", DataFile.data_dir(), "fac_france", 1337):
			return false
	var armies := main_armies(sim, "fac_france", "fac_england")
	if armies.is_empty():
		return false
	var index: int = -1
	if siege_demo and siege_landmark != "":
		var besieger: String = armies[1] if siege_attacker == "fac_england" else armies[0]
		index = sim.call("debug_stage_landmark_siege", besieger, siege_landmark)
	elif siege_demo:
		index = sim.call("debug_stage_siege", armies[0], siege_province)
		if index < 0:
			# Une ville française (Paris, Rouen) est assiégée par l'armée anglaise.
			index = sim.call("debug_stage_siege", armies[1], siege_province)
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
	BattleTerrain.apply_site_overrides(setup)  # --terrain= --season= --coast
	# La musique de campagne cède la place à la musique de bataille (réveillée au retour).
	audio.silence_campaign()
	if _pad_units > 0:
		_pad_setup(_pad_units)
	if _siege_engines != "" and setup.get("siege") != null:
		SiegeAssaultFx.add_engines(setup, _siege_engines.split(",", false))  # Captures, banc
		padded = true  # régiments hors campagne : pas de résultat à rapporter
	battle = ClassDB.instantiate("BattleSim")
	if _scale_tier != "":
		battle.call("set_scale_tier", _scale_tier)  # --scale=<skirmish|large|epic>
	if _seed_arg >= 0:
		battle_seed = _seed_arg
	if not battle.call("setup", setup, battle_seed):
		return false
	if _hour_override != "":
		# Heure de début imposée (bataille rapide, captures) ; la règle reste au cœur.
		if _hour_override.is_valid_float():
			battle.call("set_start_hour", float(_hour_override))
		elif not battle.call("set_start_phase", _hour_override):
			push_warning("BattleScene: unknown --hour=%s" % _hour_override)
	return _build_scene()


## Bataille historique `_historical` (site réel, ordres de bataille, météo, heure) ; le
## résultat n'est pas conservé (hors campagne).
func begin_historical() -> bool:
	if not ClassDB.class_exists("BattleSim"):
		return false
	audio.silence_campaign()
	battle = ClassDB.instantiate("BattleSim")
	if not battle.call("setup_historical", DataFile.data_dir(), _historical, _historical_side, battle_seed):
		return false
	setup = battle.call("get_setup")
	padded = true  # hors campagne : pas de résultat à rapporter
	if _hour_override != "":
		if _hour_override.is_valid_float():
			battle.call("set_start_hour", float(_hour_override))
		else:
			battle.call("set_start_phase", _hour_override)
	return _build_scene()


## Bataille personnalisée `config` (armées achetées, champ, siège) ; le résultat n'est pas
## conservé (hors campagne).
func begin_custom(config: Dictionary) -> bool:
	if not ClassDB.class_exists("BattleSim"):
		return false
	audio.silence_campaign()
	battle = ClassDB.instantiate("BattleSim")
	# Un champ différent à chaque bataille, sauf graine imposée (`seed` : tests, captures).
	battle_seed = int(config.get("seed", randi() % 1000000))
	if not battle.call("setup_custom", DataFile.data_dir(), config, battle_seed):
		return false
	setup = battle.call("get_setup")
	padded = true  # hors campagne : pas de résultat à rapporter
	if _hour_override != "":
		if _hour_override.is_valid_float():
			battle.call("set_start_hour", float(_hour_override))
		else:
			battle.call("set_start_phase", _hour_override)
	if not _build_scene():
		return false
	# LR-12 (NT4) : l'adversaire du joueur tient sa position (option du cœur, enregistrée au rejeu).
	if bool(config.get("hold_opponent", false)) and player_side != "":
		battle.call("set_hold", enemy_side, true)
	var attacker := str((setup.get("attacker", {}) as Dictionary).get("faction_name", ""))
	var defender := str((setup.get("defender", {}) as Dictionary).get("faction_name", ""))
	_title_text = "Bataille personnalisée : %s contre %s" % [attacker, defender]
	if not _prologue_data.is_empty():
		_title_text = str(_prologue_data.get("title", _title_text))  # NT4
	hud.set_title(_title_text, _weather_text, [side_colors[player_side], side_colors[enemy_side]])
	return true


## Branche le guide de la bataille-prologue (étapes, ennemi passif au début).
func _start_prologue() -> void:
	prologue = BattlePrologue.new()
	prologue.name = "BattlePrologue"
	prologue.setup(_prologue_data)
	add_child(prologue)
	prologue.attach(self)


## Scène de bataille (terrain, soldats, interface) une fois `battle` et `setup` prêts.
func _build_scene() -> bool:
	var setup_side: Variant = setup.get("player_side", "attacker")
	player_side = str(setup_side) if setup_side != null else ""
	if player_side == "" and standalone and siege_landmark != "":
		# Démo d'Avignon ou de Bruges, sans le joueur de la campagne : il mène l'assaut.
		player_side = "attacker"
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
		# Maison du général (armes du HUD et des figurines nobles), rendu seulement.
		var general: Variant = side_setup.get("general", null)
		if general is Dictionary and not (general as Dictionary).has("house"):
			(general as Dictionary)["house"] = HouseArms.house_of(str((general as Dictionary).get("character", "")), campaign_sim)
	var weather: Dictionary = battle.call("get_weather")
	var weather_key := _weather_override if _weather_override != "" else str(weather.get("key", "clear"))
	# Une carte historique est rendue sous son ciel final ; l'averse du début tombe de ce
	# ciel et cesse quand la simulation change de météo (Crécy).
	historical = battle.call("get_historical")
	_sim_weather = str(weather.get("key", "clear"))
	if not historical.is_empty() and _weather_override == "":
		weather_key = str(historical.get("weather_end", weather_key))
	_weather_key = weather_key
	var terrain_data: Dictionary = battle.call("get_terrain")
	terrain.province_id = str(setup.get("province", ""))  # Relief réel et panorama du lieu
	# Tuile d'horizon du site historique (campagne sur le site ou carte du menu).
	terrain.horizon_site = str(historical.get("horizon", setup.get("historical_horizon", "")))
	terrain.build(terrain_data, weather_key)
	if terrain.decor_view != null:
		terrain.decor_view.bind(battle)  # Pillage des camps
	if terrain_data.has("siege"):
		siege_view = BattleSiege.new()
		siege_view.name = "Siege"
		siege_view.side_colors = side_colors  # SB : barres de vie à la couleur des camps
		add_child(siege_view)
		siege_view.build(terrain_data["siege"], func(x: float, z: float) -> float: return terrain.height_at(x, z))
		# Ville emblématique (Paris) en toile de fond derrière la ville assiégée.
		var backdrop := LandmarkBackdrop.create(setup, terrain_data["siege"], func(x: float, z: float) -> float: return terrain.height_at(x, z))
		if backdrop != null:
			add_child(backdrop)
		# Ville assiégée tirée du plan (rues pavées ; murailles et maisons viennent du cœur).
		landmark_town = LandmarkSiegeTown.create(battle.call("get_siege_landmark"), func(x: float, z: float) -> float: return terrain.height_at(x, z))
		if landmark_town != null:
			add_child(landmark_town)
	# Heure de rendu (phase du cœur, sinon graine) ; EP8 anime la lumière si l'heure avance.
	var tod: Dictionary = battle.call("get_time_of_day")
	var time_key := BattleAtmosphere.time_key_for(tod, battle_seed, weather_key)
	BattleAtmosphere.apply(world_env, sun, weather_key, camera_rig.camera, terrain.season_key, time_key, not tod.is_empty())
	if _sim_weather == "rain" and weather_key != "rain":
		BattleAtmosphere.add_shower(camera_rig.camera)  # Averse qui cessera
	if terrain.horizon != null:
		terrain.horizon.apply_atmosphere(world_env.environment, sun, weather_key)  # EP2
	var field_center := terrain.field_center()  # (600, 400) au palier standard
	BattleAtmosphere.add_ground_mist(self, weather_key, Vector3(field_center.x, terrain.height_at(field_center.x, field_center.y), field_center.y), Vector2(terrain.FIELD_W + 300.0, terrain.FIELD_D + 300.0))
	# Taille des unités = figurines par homme simulé (rendu seulement).
	battle.call("set_figure_scale", _figure_scale())
	_open_deployment()
	units = battle.call("get_units")
	_build_soldier_layers()
	banners.build(units)
	outlines = BattleFormationOutline.new()
	add_child(outlines)
	outlines.setup(side_colors, player_side, func(x: float, z: float) -> float: return terrain.height_at(x, z), func(x: float, z: float) -> float: return terrain.surface_height(x, z))
	path_preview = BattlePathPreview.new()
	add_child(path_preview)
	path_preview.setup(battle, func(x: float, z: float) -> float: return terrain.surface_height(x, z), side_colors.get(player_side, Color(0.9, 0.8, 0.3)), func(x: float, z: float) -> float: return terrain.height_at(x, z))
	cursor = BattleCursor.new()
	range_arc = BattleRangeArc.new()
	add_child(range_arc)
	range_arc.setup(battle, side_colors, player_side)
	compare_panel = BattleComparePanel.new()
	hud.root.add_child(compare_panel)
	formation_picker = BattleFormationPicker.new()
	hud.root.add_child(formation_picker)
	formation_picker.setup(self)
	_build_markers()
	var title := ("Assaut %s" if siege_view != null else "Bataille %s") % BattleScene.de(str(setup.get("province_name", "")))
	if not historical.is_empty():
		# « Bataille de Crécy (26 août 1346) » ; sur le site en campagne : « … , champ de Crécy ».
		if bool(historical.get("site_only", false)):
			title += " — champ de bataille de %s" % str(historical.get("place", ""))
		else:
			title = "%s (%s)" % [str(historical.get("name", title)), str(historical.get("date_fr", ""))]
	if landmark_town != null:
		title = "Assaut %s (%s)" % [BattleScene.de(str(landmark_town.landmark.get("name", ""))), str(landmark_town.landmark.get("gate_name", ""))]
	# `--weather=` ne force que le rendu (outil de capture) : la simulation, donc les règles
	# (tir, fatigue) et le libellé, gardent la météo tirée par `core`. On le signale au bandeau
	# plutôt que d'afficher une météo que les règles n'appliquent pas.
	var weather_label := str(weather.get("label", ""))
	if not historical.is_empty() and _weather_override == "" and str(historical.get("weather_label", "")) != "":
		weather_label = str(historical["weather_label"])  # « Averse d'orage, puis… »
	elif weather_key != str(weather.get("key", "clear")):
		weather_label += " (rendu forcé : %s)" % weather_key
		print("BattleScene: --weather=%s overrides rendering only; simulated weather is %s" % [weather_key, weather.get("key", "?")])
	_title_text = title
	_weather_text = weather_label
	hud.set_title(title, weather_label, [side_colors[player_side], side_colors[enemy_side]])
	hud.set_site(str(terrain_data.get("site_label", "")))
	hud.set_opening(battle.call("get_opening"), player_side)  # CV3-2
	hud.player_faction = str((setup[player_side] as Dictionary).get("faction", ""))
	hud.set_leader((setup[player_side] as Dictionary).get("general", null), hud.player_faction)
	# Textes des capacités pour les infobulles des boutons de carte.
	hud.ability_catalog = battle.call("get_ability_catalog")
	# RJ-a : formations de régiment (menu du bouton « Formation », infobulles).
	hud.setup_formations(battle.call("unit_formations"), battle.call("formation_reform_rules"))
	camera_rig.height_at = func(x: float, z: float) -> float: return terrain.world_height(x, z)
	camera_rig.bounds = Rect2(-150, -150, terrain.FIELD_W + 300.0, terrain.FIELD_D + 300.0)  # EP1
	# Recul maximal selon la largeur du champ (900 m au standard, 1350 m à 2400 m).
	camera_rig.max_distance = 900.0 * (0.5 + 0.5 * maxf(terrain.field_scale_x(), 1.0))
	if deployment != null and deployment.active and siege_view == null:
		deployment.frame_zone()  # RX batvis : la zone de déploiement, pas le cadrage d'ouverture
	else:
		_frame_camera()
	_setup_staging(terrain_data)
	hud.minimap.flipped = player_side == "attacker"
	hud.minimap.player_side = player_side
	hud.minimap.setup(terrain_data, side_colors)
	hud.add_events(battle.call("get_events"))
	_leader_bar = LEADER_ORDERS_BAR.new(self)
	add_child(_leader_bar)
	if compare_panel != null:
		compare_panel.above = _leader_bar.panel
	audio.build(_weather_key)
	_refresh_view(true)
	if not replay_mode:  # Pas de discours ni de conseil pendant un rejeu
		_start_speech()
		_advise_first_battle()
	return true


## Mise en scène (heure, nuages, poussière, fumées, oiseaux, plan cinématique).
func _setup_staging(terrain_data: Dictionary) -> void:
	staging = BattleStaging.new()
	add_child(staging)
	var decor_view: BattleDecor = terrain.decor_view
	if decor_view != null and decor_view.has_camps():
		staging.auto_campfires = false  # Les camps du décor portent leurs feux
	staging.setup(self, battle, world_env.environment, sun, _weather_key, terrain_data)
	staging.configure_effects(effects, str(terrain_data.get("terrain", "plains")), terrain.season_key)
	if decor_view != null:
		decor_view.attach_smoke(self, staging, _weather_key)
	if effects != null:
		effects.cannon_fired.connect(staging.on_cannon_fired)
	if staging.cinematic != null and (autoplay or capture.screenshot_path != ""):
		# Jamais en banc d'essai, en capture ni quand l'IA joue les deux camps.
		staging.cinematic.enabled = false
	_update_time_label()


## Pose une source de fumée durable (EP6 : feux des camps) ; -1 si coupée ou hors budget.
func add_smoke_source(position: Vector3, intensity: float = 1.0, kind: String = "campfire") -> int:
	return staging.add_smoke_source(position, intensity, kind) if staging != null else -1


## L'heure au bandeau (« Temps clair · Crépuscule, 18 h 40 (portée des tireurs −30 %) ») ;
## L'heure elle-même s'affiche (`BattleTimeOfDay.clock_label`), pas seulement le
## nom de la phase, pour que le temps compressé de la bataille (0,2 min de jour par seconde
## simulée) reste lisible ; rafraîchi à chaque changement de phase ou d'heure affichée (arrondie à
## 10 min).
func _update_time_label() -> void:
	if staging == null or staging.tod.is_empty():
		return
	var key := str(staging.tod.get("key", ""))
	var clock := BattleTimeOfDay.clock_label(staging.tod)
	if key == _tod_key and clock == _tod_clock:
		return
	_tod_key = key
	_tod_clock = clock
	var label := clock
	var visibility := float(staging.tod.get("visibility", 1.0))
	if visibility < 0.999:
		label += " (portée des tireurs −%d %%)" % int(round((1.0 - visibility) * 100.0))
	hud.set_title(_title_text, "%s · %s" % [_weather_text, label] if _weather_text != "" else label, [side_colors[player_side], side_colors[enemy_side]])


## Le conseiller parle après le discours adverse s'il est en cours.
func _advise_after_speeches(trigger: String) -> void:
	if enemy_speech != null and is_instance_valid(enemy_speech) and enemy_speech.active:
		enemy_speech.finished.connect(func() -> void: Advisor.say_trigger(trigger), CONNECT_ONE_SHOT)
	else:
		Advisor.say_trigger(trigger)


## Le conseiller commente la première bataille (ou le premier assaut), après le discours.
func _advise_first_battle() -> void:
	if autoplay or not _prologue_data.is_empty():  # Le guide parle déjà
		return
	var trigger := "first_assault" if siege_view != null else "first_battle"
	if speech != null:
		speech.finished.connect(_advise_after_speeches.bind(trigger), CONNECT_ONE_SHOT)
	else:
		Advisor.say_trigger(trigger)


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


## `kept_impostors` : imposteurs repris d'avant un saut arrière du rejeu (EP13), sinon créés.
func _build_soldier_layers(kept_impostors: BattleImpostors = null) -> void:
	soldiers = BattleSoldiers.new()
	soldiers.name = "Soldiers"
	add_child(soldiers)
	# Imposteurs lointains, cuits au début de la bataille (ADR 0024).
	soldiers.impostors = kept_impostors if kept_impostors != null else BattleImpostors.new()
	soldiers.impostors.name = "Impostors"
	soldiers.add_child(soldiers.impostors)
	var factions := {}
	var houses := {}  # Maison du général par camp
	for side in ["attacker", "defender"]:
		factions[side] = str((setup[side] as Dictionary).get("faction", ""))
		var general: Variant = (setup[side] as Dictionary).get("general", null)
		houses[side] = str((general as Dictionary).get("house", "")) if general is Dictionary else ""
	_side_houses = houses
	soldiers.setup(units, side_colors, factions, houses)
	_mm = soldiers.layers
	_setup_standards()
	effects = BattleEffects.new()
	effects.name = "Effects"
	add_child(effects)
	# L'eau à la largeur locale de la rivière, et les ruisseaux.
	effects.setup(_weather_key, func(x: float, z: float) -> float: return terrain.world_height(x, z), func(x: float, z: float) -> int: return 1 if terrain.in_water(x, z) else 0)
	effects.configure_ground(str(terrain.terrain.get("ground", "dry")), _weather_key)
	effects.volleys.figure_scale = float(battle.call("get_figure_scale"))
	effects.volleys.sound_event.connect(audio.on_sound_event)
	effects.sound_event.connect(audio.on_sound_event)
	BattleAudio.auto_volley = false
	blood = BattleBlood.new()
	blood.name = "Blood"
	effects.add_child(blood)
	blood.setup(func(x: float, z: float) -> float: return terrain.world_height(x, z), _blood_level(), func(x: float, z: float) -> int: return 1 if terrain.in_water(x, z) else 0)
	# Les morts portent la flaque au sol et les traits fichés dans les corps ; la gerbe reste à
	# `BattleGore`, une seule source par événement.
	soldiers.corpse_fallen.connect(func(pos: Vector3, side: String, kind: String, cause: String) -> void:
		blood.on_corpse(pos, side, kind, cause, _camera_position())
		effects.volleys.on_corpse(pos, side, kind, cause))
	# Engins animés (trébuchet, mangonneau, bombarde, roues du bélier et du beffroi).
	engines_fx = SiegeEnginesFx.new()
	engines_fx.name = "EnginesFx"
	add_child(engines_fx)
	engines_fx.setup(soldiers, effects, siege_view)
	effects.engine_fx = engines_fx
	if siege_view != null:
		assault_fx = SiegeAssaultFx.new()
		assault_fx.name = "AssaultFx"
		add_child(assault_fx)
		assault_fx.engines_fx = engines_fx
		assault_fx.setup(siege_view, effects, soldiers, func(x: float, z: float) -> float: return terrain.height_at(x, z))
	_setup_grass_flatten()


## Vent de la météo (drapeaux, herbe) et porte-étendards des régiments.
func _setup_standards() -> void:
	var wind := BattleStandards.wind_for(_weather_key, battle_seed)
	standards = BattleStandards.new()
	standards.name = "Standards"
	add_child(standards)
	var factions := {}
	for side in ["attacker", "defender"]:
		factions[side] = str((setup.get(side, {}) as Dictionary).get("faction", ""))
	standards.setup(units, side_colors, func(unit: Dictionary) -> Dictionary: return banners.cloth_for(unit, str((setup[str(unit["side"])] as Dictionary).get("faction", ""))), wind, factions, battle, _side_houses)
	for flag_material in banners.flag_materials():
		standards.apply_wind(flag_material)
	if terrain.vegetation != null:
		terrain.vegetation.set_wind(wind["dir"], float(wind["strength"]) * float(wind["grass_scale"]))


## Discours du général du joueur, au début du déploiement (ou de la bataille), en jeu
## seulement (pas en `--autoplay`, captures ni bancs).
func _start_speech() -> void:
	if autoplay or not _prologue_data.is_empty():
		return
	var ours := float(battle.call("get_strength", player_side))
	var theirs := maxf(float(battle.call("get_strength", enemy_side)), 1.0)
	var text := BattleSpeech.compose(setup, player_side, ours / theirs, terrain.terrain_key, _weather_key, battle_seed)
	speech = BattleSpeech.new()
	speech.name = "Speech"
	add_child(speech)
	if not speech.start(self, text, units, player_side):
		speech.queue_free()
		speech = null
		return
	speech.finished.connect(_start_enemy_speech, CONNECT_ONE_SHOT)


## Discours du général adverse, joué juste après celui du joueur ; un « passer » du joueur écarte aussi celui de l'adversaire.
func _start_enemy_speech() -> void:
	if speech == null or speech.skipped:
		return
	var ours := float(battle.call("get_strength", player_side))
	var theirs := maxf(float(battle.call("get_strength", enemy_side)), 1.0)
	var text := BattleSpeech.compose(setup, enemy_side, theirs / maxf(ours, 1.0), terrain.terrain_key, _weather_key, battle_seed)
	enemy_speech = BattleSpeech.new()
	enemy_speech.name = "EnemySpeech"
	add_child(enemy_speech)
	if not enemy_speech.start(self, text, units, enemy_side):
		enemy_speech.queue_free()
		enemy_speech = null


## Herbe couchée par les troupes et sous les corps, sang lisible en prairie ; pavois du
## dos masqué quand la rangée de BV1 est plantée.
func _setup_grass_flatten() -> void:
	soldiers.hide_planted_pavise = true  # BV1 plante les rangées de pavois
	if terrain.vegetation == null:
		return
	grass_flatten = BattleGrassFlatten.new()
	grass_flatten.setup(Vector2(terrain.FIELD_W, terrain.FIELD_D))
	terrain.vegetation.set_flatten(grass_flatten)
	var blood_amount: float = 0.0 if blood == null else [0.0, 0.6, 1.0][blood.level]
	soldiers.corpse_fallen.connect(func(pos: Vector3, _side: String, kind: String, cause: String) -> void:
		grass_flatten.on_corpse(pos, kind, 0.0 if cause == "fire" else blood_amount))


## Réglages du joueur lus par la bataille (BV1) : `--unit-size=` / `--blood=` les forcent.
func _unit_size() -> float:
	if _unit_size_override > 0.0:
		return _unit_size_override
	var settings := get_node_or_null("/root/Settings")
	return float(settings.call("get_value", "battle/unit_size")) if settings != null else 1.0


## Taille des unités effective. Le multiplicateur choisi est abaissé pour que le total de
## figurines tienne dans « Figurines maximum » (`battle/max_figures`) ; `--unit-size=` l'emporte.
func _figure_scale() -> float:
	if _unit_size_override > 0.0:
		return _unit_size_override
	var settings := get_node_or_null("/root/Settings")
	var budget: int = int(settings.call("get_value", "battle/max_figures")) if settings != null else 0
	var men := 0
	for unit in battle.call("get_units"):
		men += int(unit.get("soldiers", 0))
	return BattleScene.capped_figure_scale(_unit_size(), men, budget)


## Multiplicateur de figurines borné par le plafond `budget` pour `men` hommes simulés
## (0 = pas de plafond).
static func capped_figure_scale(unit_size: float, men: int, budget: int) -> float:
	if men <= 0 or budget <= 0:
		return unit_size
	return minf(unit_size, float(budget) / float(men))


func _blood_level() -> int:
	if _blood_override >= 0:
		return _blood_override
	return BattleGore.blood_level()  # même lecture que BV2 (`--blood=off|moderate|full|0|1|2`)


func _camera_position() -> Vector3:
	var camera := get_viewport().get_camera_3d()
	return camera.global_position if camera != null else Vector3.ZERO


## Effets (poussière, traits…) d'après l'état des régiments ; `dt` = temps simulé écoulé.
func _update_effects(dt: float) -> void:
	terrain.update_trample(units, dt)  # Neige piétinée (sans effet hors neige au sol)
	if grass_flatten != null:
		grass_flatten.update(units, dt)
	if effects == null:
		return
	var camera := get_viewport().get_camera_3d()
	var camera_pos := camera.global_position if camera != null else Vector3.ZERO
	var shots: Variant = battle.call("get_shots")
	if engines_fx != null:
		engines_fx.update(units, shots, soldiers.anim_time, dt)
	effects.update(units, soldiers, soldiers.anim_time, dt, camera_pos, shots)
	if blood != null:
		blood.tick_time(soldiers.anim_time)
		blood.update(units, camera_pos)
	if assault_fx != null:
		assault_fx.update(battle.call("get_siege_events"), units, soldiers.anim_time, dt)


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
	if siege_view != null and player_side == "attacker" and n > 0 and _frame_siege_camera(center):
		return
	var yaw := PI if player_side == "attacker" else 0.0
	# Cadrage sur le centre de l'armée du joueur (léger décalage vers l'ennemi), à ~66 m de
	# hauteur, regard vers l'ennemi (réglages `battle.opening_*` de `camera_feel.json`).
	var ahead := CameraFeel.get_value("battle", "opening_ahead_m")
	center.z += ahead if player_side == "attacker" else -ahead
	camera_rig.look_at_point(center, CameraFeel.get_value("battle", "opening_distance_m"), yaw)


## Q4 (Q3 : caméra d'assaut cadrant un bélier sur une plaine vide) : l'assaillant ouvre derrière
## son armée, face à la porte, murailles et régiments dans le même plan.
func _frame_siege_camera(army: Vector3) -> bool:
	var siege: Dictionary = battle.call("get_siege")
	var pieces: Array = siege.get("pieces", [])
	var gate_index := int(siege.get("gate", -1))
	if gate_index < 0 or gate_index >= pieces.size():
		return false
	var gate: Dictionary = pieces[gate_index]
	var mid: Vector2 = ((gate["a"] as Vector2) + (gate["b"] as Vector2)) * 0.5
	var wall := Vector3(mid.x, 0, mid.y)
	var to_wall := Vector3(wall.x - army.x, 0, wall.z - army.z)
	var gap := to_wall.length()
	if gap < 1.0:
		return false
	var dir := to_wall / gap
	# Point visé un peu au-delà du milieu, recul selon l'écart (murs et armée à l'écran).
	var focus := army + dir * gap * 0.45
	var distance := clampf(gap * 0.9 + 140.0, 220.0, 520.0)
	camera_rig.look_at_point(focus, distance, atan2(-dir.x, -dir.z))
	return true


# --- Rejeu (EP13) : raccourcis vers `replay` (BattleReplayController) ----------------------


func replay_toggle_play() -> void:
	replay.toggle_play()


func replay_set_speed(value: float) -> void:
	replay.set_speed(value)


func replay_seek(seconds: float) -> void:
	replay.seek(seconds)


func start_replay_in_place() -> bool:
	return replay.start_in_place()


# --- Boucle ---------------------------------------------------------------------------


func _process(delta: float) -> void:
	if battle == null:
		return
	# Ralenti du plan cinématique (temps de bataille et animations).
	var slow := staging.time_scale() if staging != null else 1.0
	slow = minf(slow, _general_slow_factor(delta))
	var running: bool = not paused and not battle.call("is_finished")
	if running:
		_configure_step_thread()
		battle.call("tick", delta * speed * slow)
	_refresh_view(false, delta * slow)
	_update_hover_cursor()
	if staging != null:
		staging.update(units, delta * speed * slow if running else 0.0, delta, bool(battle.call("is_finished")))
		_update_time_label()
	audio.update(delta, units, not paused and not battle.call("is_finished"), speed, float(battle.call("get_elapsed")), _frame_siege, siege_view != null)
	_poll_weather(delta)
	if replay_mode:
		replay.update()  # Pas d'écran de fin pendant un rejeu
	elif battle.call("is_finished") and not finished_shown:
		_end_wait += delta
		if _end_wait >= _victory_hold():
			if autoplay:
				print(outcome_line())  # Sonde d'issue : une ligne de texte par bataille
				if DisplayServer.get_name() == "headless" and standalone:
					get_tree().quit()
					return
			if _start_victory_shot():
				return
			if staging != null and staging.cinematic != null and staging.cinematic.active:
				return  # Plan de victoire en cours : l'écran de fin attend sa fin
			_show_end()


## TW bfeel (top 6) : plan de victoire avant l'écran de fin, si le joueur a gagné (orbite sur son
## général vainqueur, passable). Pas en IA contre IA, en capture, en rejeu ni sans fenêtre.
func _start_victory_shot() -> bool:
	if autoplay or replay_mode or staging == null or staging.cinematic == null or DisplayServer.get_name() == "headless":
		return false
	if staging.cinematic.victory_done or not staging.cfg.has("victory_shot"):
		return false
	var outcome: Dictionary = battle.call("get_outcome")
	if str(outcome.get("winner", "")) != player_side:
		return false
	var focus := victory_focus(units, player_side)
	if focus.is_empty():
		return false
	return staging.cinematic.start_victory(focus["point"], float(focus["yaw"]), staging.cfg["victory_shot"])


## Point et lacet du plan de victoire : le général de `side` s'il est encore là, sinon le centre
## de ses régiments présents ({} si aucun). Lacet : face à la mêlée, vers l'ennemi le plus proche.
static func victory_focus(units_now: Array, side: String) -> Dictionary:
	var best := {}
	var sum := Vector3.ZERO
	var count := 0
	for unit in units_now:
		if str(unit["side"]) != side or not bool(unit["present"]) or str(unit.get("category", "")) == "siege":
			continue
		var at := Vector3(float(unit["x"]), float(unit["y"]), float(unit["z"]))
		if bool(unit.get("is_general", false)):
			best = {"point": at, "yaw": float(unit.get("facing", 0.0))}
			break
		sum += at
		count += 1
	if not best.is_empty():
		return best
	if count == 0:
		return {}
	return {"point": sum / float(count), "yaw": 0.0}


## RX batsim : issue d'une bataille finie en une ligne de texte (`--autoplay --seed=<n>`) :
## `autoplay: seed=… winner=… end=… t=…s losses=att/def`.
func outcome_line() -> String:
	var outcome: Dictionary = battle.call("get_outcome")
	var attacker: Dictionary = outcome.get("attacker", {})
	var defender: Dictionary = outcome.get("defender", {})
	return "autoplay: seed=%d winner=%s end=%s t=%ds losses=%d/%d" % [battle_seed, outcome.get("winner", "?"), outcome.get("end", "?"), int(battle.call("get_elapsed")), int(attacker.get("total_losses", 0)), int(defender.get("total_losses", 0))]


## Le pas de simulation suivant se calcule sur un fil pendant que l'image
## montre le pas courant (même bataille, au bit près). Synchrone en headless (tests), pendant un
## rejeu.
func _configure_step_thread() -> void:
	_configure_pose_lerp()
	var wanted := not replay_mode and DisplayServer.get_name() != "headless"
	if wanted == _step_thread_on:
		return
	_step_thread_on = wanted
	battle.call("set_step_thread", wanted)


## RJ-b : poses interpolées entre deux pas (fraction du pas en cours, `step_fraction` du cœur).
## `--pose-lerp` force l'interpolation en headless (test, banc).
func _configure_pose_lerp() -> void:
	var forced := CmdArgs.has("--pose-lerp")
	var wanted := (forced or DisplayServer.get_name() != "headless")
	if wanted == _pose_lerp_on:
		return
	_pose_lerp_on = wanted
	battle.call("set_pose_lerp", wanted)


## La météo d'une carte historique change pendant la bataille (averse de Crécy) : la pluie
## cesse de tomber, le journal l'annonce (le ciel est déjà celui de la fin).
func _poll_weather(delta: float) -> void:
	if historical.is_empty():
		return
	_weather_poll -= delta
	if _weather_poll > 0.0:
		return
	_weather_poll = 1.0
	var key := str((battle.call("get_weather") as Dictionary).get("key", _sim_weather))
	if key == _sim_weather:
		return
	var camera := camera_rig.camera
	if _sim_weather == "rain" and camera.get_node_or_null("Precipitation") != null and _weather_key != "rain":
		camera.get_node("Precipitation").queue_free()
	elif key == "rain" and _weather_key != "rain":
		BattleAtmosphere.add_shower(camera)
	_sim_weather = key


## Avance la simulation (pas de 0,1 s) jusqu'à `seconds`, cadavres et effets compris. 
func _fast_forward(seconds: float) -> void:
	while float(battle.call("get_elapsed")) < seconds and not battle.call("is_finished"):
		battle.call("tick", 0.1)
		units = battle.call("get_units")
		soldiers.update(battle, units, 0.1, [])
		_update_effects(0.1)
		if staging != null:
			staging.update(units, 0.1, 0.0, false)


func _refresh_view(force: bool, delta: float = 0.0) -> void:
	units = battle.call("get_units")
	var finished: bool = battle.call("is_finished")
	var running: bool = not paused and not finished
	# Une fois la bataille finie, le camp vainqueur acclame (son horloge d'animation
	# continue ; les autres régiments restent figés, cf. BattleSoldiers.victor_side).
	if finished and soldiers.victor_side == "":
		var winner := str((battle.call("get_outcome") as Dictionary).get("winner", ""))
		soldiers.victor_side = winner if winner != "" else "-"
	elif not finished:
		soldiers.victor_side = ""
	var anim_dt := delta * speed if running else (delta if finished and not paused else 0.0)
	soldiers.update(battle, units, anim_dt, selected)
	_update_effects(delta * speed if running else 0.0)
	if siege_view != null:
		# Lu une seule fois par image (gros dictionnaire construit par le cœur) : HUD, sortie
		# et sons du siège reprennent cette lecture.
		_frame_siege = battle.call("get_siege")
		siege_view.update(_frame_siege, units)
	var banner_scale := _banner_scale()
	# Les drapeaux se présentent de trois quarts à la caméra (lisibles sans être des panneaux).
	banners.update(units, banner_scale, camera_rig.yaw + PI * 0.5 + 0.35, standards)
	_update_outlines()
	if path_preview != null:
		path_preview.update_orders(units, selected, Time.get_ticks_msec() / 1000.0, camera_rig.distance)
	if standards != null:
		standards.update(units, soldiers, _camera_position())
	_update_markers(banner_scale)
	_hud_timer -= delta
	if force or _hud_timer <= 0.0:
		_hud_timer = 0.1
		hud.set_clock(float(battle.call("get_elapsed")), speed, paused)
		hud.set_balance(side_names[player_side], int(battle.call("get_strength", player_side)), side_names[enemy_side], int(battle.call("get_strength", enemy_side)))
		if siege_view != null:
			hud.set_siege_status(siege_status(_frame_siege))
			_check_sortie(_frame_siege)
		hud.update_cards(units, player_side, selected)
		hud.minimap.update(units, camera_frame())
		var events: Array = battle.call("get_events")
		if not events.is_empty():
			hud.add_events(events)
		var alerts: Array = battle.call("get_alerts")  # CB5
		if not alerts.is_empty():
			hud.alerts_column.push_alerts(alerts)
			_auto_pause_on_alerts(alerts)
			_check_general_slowmo(alerts)
		_update_time_left()


## TW bfeel (top 5) : ralenti bref à la chute d'un général. Pas en IA contre IA, en rejeu ni quand
## le réglage « Ralenti du plan » est coupé. Réglages : `data/fx/battle_staging.json`.
var _general_slow_left := 0.0
var _general_slow_scale := 1.0
var _time_left_prev := INF


func _check_general_slowmo(alerts: Array) -> void:
	if autoplay or replay_mode or staging == null or staging.cinematic == null or not staging.cinematic.slowmo:
		return
	var cfg: Dictionary = staging.cfg.get("general_fall_slowmo", {})
	if not bool(cfg.get("enabled", true)):
		return
	for alert in alerts:
		if str((alert as Dictionary).get("kind", "")) == "general_down":
			_general_slow_left = float(cfg.get("duration_s", 1.2))
			_general_slow_scale = float(cfg.get("scale", 0.35))
			return


## Facteur de temps du ralenti de la chute du général (1 hors ralenti) ; le temps réel écoulé
## `delta` l'use.
func _general_slow_factor(delta: float) -> float:
	if _general_slow_left <= 0.0:
		return 1.0
	_general_slow_left -= delta
	return _general_slow_scale if _general_slow_left > 0.0 else 1.0


## TW bfeel (ia-sieges top 3) : temps restant avant la nuit au HUD, avertissements aux seuils.
func _update_time_left() -> void:
	var left := float(battle.call("get_time_left_s"))
	var thresholds: Array = staging.cfg.get("time_warnings_s", []) if staging != null else []
	hud.set_time_left(left, _time_left_prev if not is_inf(_time_left_prev) else left, thresholds, siege_view != null)
	_time_left_prev = left


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


## Contours de formation. Survol = troupe sous la souris sur le terrain (`world_hover`,
## tenu par `BattleInput`) ou repère B2 survolé (tous les régiments d'un groupe B7).
func _update_outlines() -> void:
	if outlines == null:
		return
	_hovered_ids.clear()
	if markers != null:
		if markers.world_hover >= 0:
			_hovered_ids.append(markers.world_hover)
		if markers.hovered >= 0:
			_hovered_ids.append_array(markers.marker_members(markers.hovered))
	if card_hover >= 0:
		_hovered_ids.append(card_hover)
	outlines.update(units, selected, _hovered_ids)
	if range_arc != null:
		range_arc.update(units, selected, _hovered_ids, camera_rig.distance)
		compare_panel.refresh(null if replay_mode else battle, units, selected, _hovered_ids, player_side, Time.get_ticks_msec() / 1000.0)


## Repères 2D au-dessus des troupes, sous les panneaux du HUD (premier enfant de sa racine).
func _build_markers() -> void:
	markers = BattleUnitMarkers.new()
	hud.root.add_child(markers)
	hud.root.move_child(markers, 0)
	markers.setup(side_colors, player_side)
	markers.marker_clicked.connect(_on_card_clicked)
	markers.marker_right_clicked.connect(_on_marker_right_clicked)


## Ancre écran de chaque repère : au-dessus du drapeau 3D du régiment.
## En vue tactique, les ennemis non `spotted` sont retirés et les pastilles sont forcées
## (regroupées, comme la vue très lointaine B7).
func _update_markers(banner_scale: float) -> void:
	if markers == null:
		return
	var tactical := tactical_view != null and tactical_view.active
	var shown: Array = BattleTacticalView.filter_spotted(units, player_side) if tactical else units
	var anchors := {}
	if markers.visible:
		var camera := camera_rig.camera
		var screen := get_viewport().get_visible_rect().grow(60.0)
		for unit in shown:
			if not bool(unit["present"]):
				continue
			var top := Vector3(float(unit["x"]), float(unit["y"]) + (BattleBannerLayer.BANNER_HEIGHT + 0.6) * banner_scale, float(unit["z"]))
			if camera.is_position_behind(top):
				continue
			var point := camera.unproject_position(top)
			if screen.has_point(point):
				anchors[int(unit["id"])] = point
	markers.update(shown, anchors, selected, camera_rig.distance, tactical)


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


## Durée d'acclamation avant l'écran de fin (aucune sans affichage).
func _victory_hold() -> float:
	if DisplayServer.get_name() == "headless":
		return 0.0
	return VICTORY_HOLD


## Écran de fin mis en scène (verdict, écus, pertes par régiment, mentions).
func _show_end() -> void:
	finished_shown = true
	var outcome: Dictionary = battle.call("get_outcome")
	if not autoplay and _prologue_data.is_empty():  # Conseiller
		Advisor.say_trigger("first_victory" if str(outcome.get("winner", "")) == player_side else "first_defeat")
	var sides := {}
	for side in ["attacker", "defender"]:
		sides[side] = {"name": side_names[side], "faction": str((setup[side] as Dictionary).get("faction", "")), "color": side_colors[side]}
	# Le résultat est appliqué dès la fin, pour montrer ses suites (captifs, rançons,
	# expérience) sur l'écran de fin ; « Retour à la campagne » ne fait plus que rendre la main.
	var side_setup: Dictionary = setup[player_side]
	var general: Variant = side_setup.get("general", null)
	var general_id := str(general.get("character", "")) if general is Dictionary else ""
	var before := BattleAftermath.snapshot(campaign_sim, str(side_setup.get("army", "")), general_id)
	_resolve_now()
	var aftermath := {}
	if bool(_resolution.get("ok", false)):
		aftermath = BattleAftermath.diff(before, BattleAftermath.snapshot(campaign_sim, str(side_setup.get("army", "")), general_id))
		aftermath["campaign_outcome"] = _resolution.get("outcome", {})  # Classe du résultat
	# A toast shown in the last seconds (garrison sortie) was drawn over the result table (Q3).
	if hud.toast_label != null:
		hud.toast_label.get_parent().visible = false
	result_screen = BattleResultScreen.new()
	hud.root.add_child(result_screen)
	result_screen.return_pressed.connect(_on_return)
	result_screen.show_result(hud.title_label.text, player_side, sides, battle.call("get_units"), outcome, aftermath)
	# La bataille est enregistrée ; « Revoir la bataille » la rejoue ici même.
	replay.save()
	if result_screen.replay_button != null:
		result_screen.replay_button.visible = true
		result_screen.replay_pressed.connect(func() -> void: start_replay_in_place())


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
	audio.restore_campaign()  # La carte retrouve sa musique de contexte
	if not standalone:
		# Retour vers la carte (pas de changement de scène) sous le voile noir parchemin.
		await SceneFader.cover()
	returned.emit(result)
	if not standalone:
		SceneFader.reveal()
	if standalone:
		SceneFader.go("res://scenes/start_menu.tscn")


# --- Entrées --------------------------------------------------------------------------


## Entrées déplacées vers `input` (`BattleInput`). Délégations fines gardées ici pour
## `game/tests/smoke.gd` (`scene.handle_group_key`, `scene.issue`).
func handle_group_key(number: int, save: bool) -> void:
	input.handle_group_key(number, save)


func _toggle_pause() -> void:
	paused = not paused


var _paused_before_quit_menu := false


## Échap sans sélection : ouvre (bataille en pause) ou ferme « Quitter la bataille ? ».
func toggle_quit_menu() -> void:
	if battle == null or replay_mode or finished_shown or battle.call("is_finished"):
		return
	if hud.confirm_panel.visible:
		hud.confirm_panel.visible = false
		return
	if hud.quit_panel.visible:
		hud.quit_panel.visible = false
		paused = _paused_before_quit_menu
		return
	_paused_before_quit_menu = paused
	paused = true
	hud.ask_quit(not standalone)


## « Quitter la bataille » : l'armée quitte le champ (règle `concede` du cœur, déploiement
## compris) ; l'écran de fin s'ouvre aussitôt et son bouton rend la main à la carte (ou au menu).
func quit_battle() -> void:
	if battle == null or replay_mode or battle.call("is_finished"):
		return
	var result: Dictionary = battle.call("issue_command", {"type": "concede"})
	if not bool(result.get("ok", false)):
		hud.show_toast("Impossible de quitter : %s." % str(result.get("error", "?")))
		paused = _paused_before_quit_menu
		return
	if deployment != null:
		deployment.dismiss()
	selected.clear()
	paused = false
	_end_wait = _victory_hold()


## Boutons de vitesse du HUD (F5b) : -1 = pause (bascule), 0..2 = SPEEDS[index] et reprise.
func _on_speed_pressed(index: int) -> void:
	if index < 0:
		_toggle_pause()
	else:
		speed = SPEEDS[index]
		paused = false
	hud.set_clock(float(battle.call("get_elapsed")), speed, paused)


## `BattleInput.speed_step` (touches + / −) : +1/-1 cran de vitesse, rejeu ou partie normale.
func _on_speed_step(delta: int) -> void:
	if replay_mode:
		if delta > 0:
			replay_set_speed(REPLAY_SPEEDS[mini(REPLAY_SPEEDS.find(speed) + 1, REPLAY_SPEEDS.size() - 1)])
		else:
			replay_set_speed(REPLAY_SPEEDS[maxi(REPLAY_SPEEDS.find(speed) - 1, 0)])
	elif delta > 0:
		_on_speed_pressed(mini(SPEEDS.find(speed) + 1, SPEEDS.size() - 1))
	else:
		_on_speed_pressed(maxi(SPEEDS.find(speed) - 1, 0))


func _on_input_command(command: Dictionary) -> void:
	issue(command)


func _on_input_selection_changed(ids: Array) -> void:
	selected = ids
	_hover_dirty = true  # Le curseur dépend de la sélection


func _on_input_camera_focus(point: Vector3) -> void:
	camera_rig.glide_to(point, camera_rig.distance, camera_rig.yaw)  # Glissement 0,4 s


func _on_input_markers_toggled() -> void:
	if markers != null:
		markers.toggle()


## Touche Tab.
func _on_input_tactical_view_toggled() -> void:
	if tactical_view != null:
		tactical_view.toggle()


func _on_input_screenshot_requested() -> void:
	capture.take_screenshot(ProjectSettings.globalize_path("res://").path_join("../docs/img/godot-battle-%d.png" % Time.get_unix_time_from_system()).simplify_path(), false)


## Cadre de la caméra au sol (x, z) : les quatre coins de l'écran projetés sur le terrain.
func camera_frame() -> PackedVector2Array:
	var frame := PackedVector2Array()
	var screen := get_viewport().get_visible_rect().size
	for corner in [Vector2(0, 0), Vector2(screen.x, 0), screen, Vector2(0, screen.y)]:
		var point := ground_point(corner)
		frame.append(Vector2(point.x, point.z))
	return frame


## Alertes qui déclenchent la pause automatique (réglage `battle/auto_pause_on_alert`).
const AUTO_PAUSE_KINDS := ["rout", "general_down"]


## Pause automatique : déroute d'une troupe du joueur ou chute de son général (désactivée par
## défaut, jamais pendant un rejeu). Reprise : Espace.
func _auto_pause_on_alerts(alerts: Array) -> void:
	var settings := get_node_or_null("/root/Settings")
	if replay_mode or paused or settings == null or not bool(settings.call("get_value", "battle/auto_pause_on_alert")):
		return
	for alert in alerts:
		var side := str(alert.get("side", ""))
		var kind := str(alert.get("kind", ""))
		if AUTO_PAUSE_KINDS.has(kind) and (side == "" or side == player_side):
			paused = true
			hud.show_toast("Pause : %s (Espace pour reprendre)." % BattleAlertsColumn.LABELS.get(kind, kind).to_lower())
			return


func _on_minimap_clicked(world: Vector2) -> void:
	camera_rig.look_at_point(Vector3(world.x, 0, world.y), camera_rig.distance, camera_rig.yaw)


## Clic sur une alerte de la colonne = caméra sur le lieu + repère pulsé sur la minicarte.
func _on_alert_pinged(x: float, z: float) -> void:
	camera_rig.look_at_point(Vector3(x, 0, z), camera_rig.distance, camera_rig.yaw)
	hud.minimap.ping(Vector2(x, z))


## Envoie une commande à la simulation ; les refus s'affichent au journal.
func _exit_tree() -> void:
	if cursor != null:
		cursor.reset()  # Rendre la flèche du système hors de la bataille


## Position de la souris notée par `BattleInput` ; le curseur est recalculé au plus une
## fois par image, et seulement si la case de 2 m visée, le régiment survolé ou la sélection
## changent.
## NA (ADR 0219) : bulle codex différée au survol d'un arbre ou d'un buisson de la bataille.
var decor_hover: DecorHover = null
const DECOR_MIN_RADIUS_PX := 12.0
const DECOR_REACH_M := 40.0  # grand arbre : la tête se projette loin du pied sous une caméra rasante


func _setup_decor_hover() -> void:
	decor_hover = DecorHover.new()
	decor_hover.name = "DecorHover"
	decor_hover.provider = pick_decor
	decor_hover.blocker = func(screen_position: Vector2) -> bool:
		if markers != null and markers.world_hover >= 0:
			return true  # une troupe est visée : elle prime
		return pick_unit(screen_position, "attacker") >= 0 or pick_unit(screen_position, "defender") >= 0
	add_child(decor_hover)


## NA : arbre ou buisson sous un point écran : {kind: "battle_tree", species} ou {}. Point du sol
## visé, puis arbres plantés alentour (`BattleTerrain.decor_candidates`) départagés en espace
## écran (centre du houppier, rayon projeté, au moins `DECOR_MIN_RADIUS_PX`).
func pick_decor(screen_position: Vector2) -> Dictionary:
	var camera := camera_rig.camera
	if terrain == null or camera == null:
		return {}
	var ground := ground_point(screen_position)
	var reach := DECOR_REACH_M
	var right := camera.global_transform.basis.x
	var best := {}
	var best_score := 1.0
	for tree: Dictionary in terrain.decor_candidates(Vector2(ground.x, ground.z), reach):
		var centre: Vector3 = (tree["position"] as Vector3) + Vector3.UP * float(tree["height"]) * 0.55
		if camera.is_position_behind(centre):
			continue
		var at := camera.unproject_position(centre)
		var radius := maxf(at.distance_to(camera.unproject_position(centre + right * float(tree["radius"]))), DECOR_MIN_RADIUS_PX)
		var score := at.distance_to(screen_position) / radius
		if score < best_score:
			best_score = score
			best = {"kind": "battle_tree", "species": tree["species"]}
	return best


func note_mouse(position: Vector2) -> void:
	_hover_mouse = position
	_hover_dirty = true


func _update_hover_cursor() -> void:
	if cursor == null or not _hover_dirty or _hover_mouse.x < 0.0:
		return
	_hover_dirty = false
	if replay_mode:
		cursor.apply("none")
		_show_terrain_tip(false)
		return
	var target: int = markers.world_hover if markers != null else -1
	var point := Vector3.ZERO
	if target >= 0:
		for unit in units:
			if int(unit["id"]) == target:
				point = Vector3(float(unit["x"]), 0.0, float(unit["z"]))
				break
	else:
		point = ground_point(_hover_mouse)
	var key := "%d,%d,%d,%s" % [floori(point.x / 2.0), floori(point.z / 2.0), target, str(selected)]
	if key != _hover_key:
		_hover_key = key
		hover_calls += 1
		last_hover = battle.call("hover_context", point.x, point.z, PackedInt32Array(selected))
	var context := str(last_hover.get("context", "none"))
	if path_preview != null and path_preview.live_active and not path_preview.reachable():
		context = "forbidden"  # destination sans chemin : l'ordre ne partira pas
	# Maj tenue et file pleine : l'ordre en file serait refusé.
	var full := Input.is_key_pressed(KEY_SHIFT) and not selected.is_empty() and BattlePathPreview.queue_full(units, selected)
	if full:
		context = "forbidden"
	_show_queue_tip(full)
	_show_terrain_tip(target < 0 and not full)
	cursor.apply(context)


func _show_terrain_tip(allowed: bool) -> void:
	var decor: Dictionary = last_hover.get("decor", {}) if allowed else {}
	if decor.is_empty():
		if terrain_tip != null:
			terrain_tip.hide_tip()
		return
	if terrain_tip == null and hud != null:
		terrain_tip = _TerrainTip.new()
		hud.root.add_child(terrain_tip)
	if terrain_tip != null:
		terrain_tip.show_at(_hover_mouse, decor)


func _show_queue_tip(full: bool) -> void:
	if not full:
		if queue_tip != null:
			queue_tip.hide_tip()
		return
	if queue_tip == null and hud != null:
		queue_tip = BattleQueueTip.new()
		hud.root.add_child(queue_tip)
	if queue_tip != null:
		queue_tip.show_at(_hover_mouse, BattleInput.queue_full_text())


func issue(command: Dictionary) -> Dictionary:
	if log_orders_for_test or OS.has_feature("test"):
		issued_log.append(command.duplicate(true))
	if replay_mode:
		return {"ok": false, "error": "rejeu"}  # On regarde, on ne commande pas
	var result: Dictionary = battle.call("issue_command", command)
	UiSounds.play_order_result(result)  # Ordre donné ou refusé
	if voices != null:
		voices.on_order(command, result)  # Réplique du régiment
	if bool(result.get("ok", false)):
		_flash_order(command)  # TW bfeel : anneau d'ordre au sol
	if not result.get("ok", false):
		hud.add_events([{"time": battle.call("get_elapsed"), "text_fr": "Ordre refusé : %s" % result.get("error", "?")}])
	return result


## TW bfeel (top 1) : anneau au sol à la destination d'un ordre de marche, autour de l'ennemi
## visé pour une attaque.
func _flash_order(command: Dictionary) -> void:
	if path_preview == null:
		return
	var kind := str(command.get("type", ""))
	if kind == "move":
		var x := float(command.get("x", 0.0))
		var z := float(command.get("z", 0.0))
		path_preview.flash_order(Vector3(x, terrain.surface_height(x, z), z), "move")
	elif kind == "attack":
		for unit in units:
			if int(unit["id"]) == int(command.get("target", -1)):
				path_preview.flash_order(Vector3(float(unit["x"]), float(unit["y"]), float(unit["z"])), "attack")
				return


func _on_card_clicked(unit_id: int, additive: bool) -> void:
	if not additive:
		selected.clear()
	if not selected.has(unit_id):
		selected.append(unit_id)


## Clic sur le sceau du chef = sélectionner sa garde ; double clic = y centrer la caméra.
func _on_leader_clicked(double: bool) -> void:
	var id := hud.leader_unit_id()
	if id < 0:
		return
	if double:
		_on_card_double_clicked(id)
	else:
		_on_card_clicked(id, false)


## Double-clic sur une carte d'unité = centrer la caméra sur ce régiment (comme TW).
func _on_card_double_clicked(unit_id: int) -> void:
	input.select_same_type_of(unit_id)  # Sélection rapide, même `type`
	for unit in units:
		if int(unit["id"]) == unit_id and bool(unit["present"]):
			camera_rig.glide_to(Vector3(float(unit["x"]), 0.0, float(unit["z"])), camera_rig.distance, camera_rig.yaw)  # PO5
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
		var banner_top := camera.unproject_position(Vector3(float(unit["x"]), float(unit["y"]) + BattleBannerLayer.BANNER_HEIGHT * _banner_scale(), float(unit["z"])))
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
	var args := CmdArgs.args().duplicate()
	args.append_array(demo_args)
	demo_args = PackedStringArray()
	for arg in args:
		if capture.parse_arg(arg):
			autoplay = autoplay or capture.screenshot_path != ""
		elif arg.begins_with("--units="):
			_pad_units = int(arg.trim_prefix("--units="))
		elif arg.begins_with("--seed="):
			_seed_arg = int(arg.trim_prefix("--seed="))
		elif arg.begins_with("--scale="):
			_scale_tier = arg.trim_prefix("--scale=")
		elif arg.begins_with("--replay="):
			_replay_path = arg.trim_prefix("--replay=")  # EP13
		elif arg == "--autoplay":
			autoplay = true
		elif arg == "--siege":
			siege_demo = true
		elif arg.begins_with("--siege-province="):
			siege_demo = true
			siege_province = arg.trim_prefix("--siege-province=")
		elif arg.begins_with("--siege-landmark="):
			siege_demo = true
			siege_landmark = arg.trim_prefix("--siege-landmark=")
		elif arg.begins_with("--siege-attacker="):
			siege_attacker = arg.trim_prefix("--siege-attacker=")
		elif arg.begins_with("--siege-engines="):
			_siege_engines = arg.trim_prefix("--siege-engines=")
		elif arg.begins_with("--unit-size="):
			_unit_size_override = float(arg.trim_prefix("--unit-size="))
		elif arg.begins_with("--blood="):
			var value := arg.trim_prefix("--blood=")
			_blood_override = ["off", "moderate", "full"].find(value) if not value.is_valid_int() else clampi(int(value), 0, 2)
		elif arg.begins_with("--weather="):
			_weather_override = arg.trim_prefix("--weather=")
		elif arg.begins_with("--hour="):
			_hour_override = arg.trim_prefix("--hour=")
		elif arg.begins_with("--historical="):
			_historical = arg.trim_prefix("--historical=")
		elif arg.begins_with("--historical-side="):
			_historical_side = arg.trim_prefix("--historical-side=")
		elif arg == "--custom-battle":
			_custom = CustomBattleScreen.saved_config()  # NT2
	if capture.screenshot_path != "":
		capture.run.call_deferred()


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
	if replay_mode:
		return  # Le déploiement enregistré est rejoué par le cœur
	if autoplay and not capture.deploy_shot:
		return
	if not _prologue_data.is_empty():
		return  # Le prologue commence en bataille, armées déjà rangées
	if not historical.is_empty() and not bool(historical.get("site_only", false)):
		return  # Déploiement historique imposé

	deployment = DeploymentController.new()
	deployment.name = "Deployment"
	add_child(deployment)
	if not deployment.open(self):
		deployment.queue_free()
		deployment = null


## Sortie de la garnison (F5a) : message éphémère une fois, et mention dans la ligne du siège.
func _check_sortie(siege: Dictionary) -> void:
	if _sortie_shown or not bool(siege.get("sortie", false)):
		return
	_sortie_shown = true
	hud.show_toast("La garnison ouvre ses portes et fait une sortie !", player_side == "attacker")


