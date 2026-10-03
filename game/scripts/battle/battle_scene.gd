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
## `--benchmark` (mesure les FPS sur `BENCH_FRAMES` images puis quitte, vsync désactivé, sortie
## `BENCH_JSON {...}` systématique, code de sortie ≠ 0 en cas d'échec ou de dépassement du budget
## `--bench-timeout=<s>` (défaut 120) ; `--bench-at=<s>` avance d'abord la bataille,
## `--bench-repeat=<n>` répète la fenêtre de mesure, T8), `--autoplay` (IA des deux camps),
## `--siege` (démo autonome : assaut français de la Guyenne, bataille de siège M8),
## `--closeup` (capture : caméra rapprochée sur la mêlée, à `--closeup-distance=<m>`, 26 par
## défaut ; avec `--benchmark` : banc rapproché, FG5), `--weather=<clear|fog|rain|snow>`
## (rendu seulement : force l'aspect de la météo, la simulation garde la sienne),
## `--camera=x,z,distance,lacet` (capture : position de caméra imposée), `--deploy-shot` (avec
## `--screenshot=` : capture de la phase de déploiement, F5c), `--result-shot` (avec
## `--screenshot=` : bataille jouée jusqu'au bout, capture de l'écran de fin, B2),
## `--no-effects` (sans poussière ni traits, B4 : captures « avant », mesures A/B),
## `--no-bv1` (volées, sang, mottes et taille d'unité du lot BV1 coupés : mesures A/B),
## `--shot-at=<s>` (capture : à cet instant de la bataille plutôt qu'au premier contact, B4),
## `--standard-shot=<foot|mounted|line|fallen|captured>` (capture EP5 : gros plan d'un
## porte-étendard à pied ou à cheval, ligne de bataille et ses étendards au loin, étendard tombé,
## étendard pris porté par le vainqueur) ; `--standard-side=<attacker|defender>` choisit le camp
## cadré (DA1b : étendards aux armes de la maison du général ennemi).
## `--ep12-shot=<wounded|rout>` (capture EP12 : blessés au sol 2,5 s après leur chute, ou
## régiment à pied en déroute qui a jeté ses armes) ; `--no-ep12` coupe le lot (A/B).
## `--no-da6` (végétation de bataille DA6 coupée : herbe, lisières, arbres, sol de près ; captures
## « avant »), `--bench-ab=da6,no-da6` (banc : les deux végétations alternées, DA6).
## `--no-horizon` (relief réel lointain, panorama et silhouettes EP2 coupés : mesures A/B),
## `--horizon-province=<id>`, `--panorama=<id>` (captures EP2).
## EP8 (mise en scène, `BattleStaging`) : `--hour=<dawn|morning|midday|afternoon|dusk|night|h>`
## (heure de début : phase ou heure décimale, règle du cœur), `--no-daytime`,
## `--no-cloud-shadows`, `--no-staging-dust`, `--no-smoke`, `--no-birds`, `--no-cinematic`,
## `--no-ep8` (mesures A/B), `--cinematic` (plan cinématique même en capture ou IA contre IA),
## `--birds-shot` (capture : envol des volées), `--cinematic-shot` (capture au milieu du plan).

signal returned(result: Dictionary)

## T8 : images mesurées par répétition du banc d'essai (`--benchmark`).
const BENCH_FRAMES := 600
const KINDS := ["infantry", "archer", "cavalry", "siege"]
## CB3 : ralenti ×0,5 ajouté (le − descend jusque-là) ; index par défaut sur ×1 (`speed = 1.0`).
const SPEEDS := [0.5, 1.0, 2.0, 4.0]
## EP13 : vitesses du rejeu (barre de rejeu, + / −).
const REPLAY_SPEEDS := [1.0, 2.0, 4.0, 8.0]
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
var _side_houses: Dictionary = {}  # DA1 / DA1b : maison du général par camp (écus, étendards)
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
## AN1b : secondes d'acclamation du vainqueur montrées avant l'écran de fin (0 sans affichage,
## en banc d'essai) ; `_end_wait` les compte.
const VICTORY_HOLD := 3.0
var _end_wait: float = 0.0
var resolved: bool = false
var _returned: bool = false  # UB1 : « Retour à la campagne » déjà émis
var standalone: bool = false
var siege_view: BattleSiege = null  # batailles de siège (M8)
var engines_fx: SiegeEnginesFx = null  # SG2 : engins de siège animés
var assault_fx: SiegeAssaultFx = null  # SG1 : engins, échelles, porte, huile (événements du cœur)
var _siege_engines := ""  # SG1 : `--siege-engines=` (captures, banc d'essai)
var siege_demo: bool = false
## L3 : province de la démo de siège (`--siege-province=prov_ile_de_france` : Paris).
var siege_province: String = "prov_guyenne"
## SG2 : ville assiégée dans son plan (`--siege-landmark=avignon`, guerre déclarée au détenteur si
## besoin) et faction de l'assiégeant (`--siege-attacker=fac_england`).
var siege_landmark: String = ""
var siege_attacker: String = "fac_france"
## SG2 : options d'une démo lancée depuis le menu (`BattleDemosMenu`), lues comme la ligne de
## commande puis oubliées.
static var demo_args: PackedStringArray = []
## NT2 : bataille personnalisée composée dans `CustomBattleScreen` (configuration de
## `BattleSim.setup_custom`), lue au `_ready` puis oubliée. `--custom-battle` rejoue la dernière
## composition gardée dans les réglages (captures, bancs).
static var custom_config: Dictionary = {}
var _custom: Dictionary = {}
## NT4 : bataille-prologue guidée (`BattlePrologue.launch`), lue au `_ready` avec
## `custom_config` puis oubliée ; pas de déploiement, de discours ni de conseiller spontané.
static var prologue_data: Dictionary = {}
var _prologue_data: Dictionary = {}
var prologue: BattlePrologue = null
var landmark_town: LandmarkSiegeTown = null

var _mm: Dictionary = {}  # unit id -> MultiMeshInstance3D (BattleSoldiers.layers)
var soldiers: BattleSoldiers = null
var effects: BattleEffects = null  # B4 : poussière, traits, fumée des bombardes, gués
var _weather_key: String = "clear"
var blood: BattleBlood = null  # BV1 : sang au sol (réglage « Sang »)
var grass_flatten: BattleGrassFlatten = null  # BV3 : herbe couchée et tachée de sang
var standards: BattleStandards = null  # BV3 : vent, porte-étendards
var duels: BattleDuels = null  # BV3 : duels appariés cosmétiques
var speech: BattleSpeech = null  # BV3 : discours du général avant la bataille
var enemy_speech: BattleSpeech = null  # NT6a : discours du général adverse, après celui du joueur
var _no_speech: bool = false  # `--no-speech`
var _speech_shot: String = ""  # `--speech-shot=<png>` (avec `--speech-at=<s>`)
var _speech_at: float = 6.0
var _no_bv3: bool = false  # `--no-bv3` : finitions BV3 coupées (mesures A/B)
var _no_impostors: bool = false  # `--no-impostors` : imposteurs lointains seuls coupés (A/B)
var _unit_size_override: float = -1.0  # `--unit-size=<k>` (banc d'essai BV1)
var _blood_override: int = -1  # `--blood=<0|1|2>`
var _no_bv1: bool = false  # `--no-bv1` : volées, sang et mottes du lot BV1 coupés (mesures A/B)
var _no_effects: bool = false  # `--no-effects` : captures « avant » et mesures A/B
var _banners: Dictionary = {}  # id -> {node, flag_mat, routing}
var markers: BattleUnitMarkers = null  # B2 : bannières flottantes (repères 2D)
var result_screen: BattleResultScreen = null  # B2 : écran de fin
var _result_shot: bool = false
## CB-M1 : contours de formation (décales), remplacent l'anneau jaune de sélection.
var outlines: BattleFormationOutline = null
var _hovered_ids: Array[int] = []  # régiments survolés (terrain, repère), réutilisé
## CB-M2 : aperçu du trajet, curseur contextuel, carte du HUD survolée (-1 : aucune).
var path_preview: BattlePathPreview = null
var cursor: BattleCursor = null
## CB-M4 : portée au sol des tireurs sélectionnés ou survolés, comparaison au survol d'un ennemi.
var range_arc: BattleRangeArc = null
var compare_panel: BattleComparePanel = null
## CB6 : sélecteur de formation de groupe (bas droite, au-dessus du bandeau des cartes).
var formation_picker: BattleFormationPicker = null
var card_hover := -1
## CB-M3 : infobulle « file d'ordres pleine » (Maj tenue).
var queue_tip: BattleQueueTip = null
## Dernier `hover_context` du cœur et nombre d'appels (tests : au plus un par image).
var last_hover: Dictionary = {}
var hover_calls := 0
var _hover_mouse := Vector2(-1, -1)
var _hover_dirty := false
var _hover_key := ""
var _drag_rect: ColorRect  # CB0 : rectangle de sélection, lu et positionné par `input`
var _hud_timer: float = 0.0
var _screenshot_path: String = ""
var _benchmark: bool = false
var _bench_frames: int = 0
var _bench_time: float = 0.0
var _bench_at: float = -1.0  # `--bench-at=<s>` : avance rapide avant la mesure
var _bench_start_elapsed: float = 0.0
## T8 : répétitions (`--bench-repeat=`), temps d'image collectés (ms, toutes répétitions
## confondues, pour médiane/p95), budget de temps réel (`--bench-timeout=`, défaut 120 s) et
## drapeau d'échec (pour ne conclure qu'une fois).
var _bench_repeat: int = 1
var _bench_repeat_done: int = 0
var _bench_frame_ms: PackedFloat64Array = PackedFloat64Array()
var _bench_wall_start_ms: int = -1
var _bench_timeout_s: float = 120.0
var _bench_failed: bool = false
var _bench_gpu_ms: float = 0.0  # V3 : temps de rendu GPU cumulé
var _bench_cpu_ms: float = 0.0
## PB3c : temps de `_process` (Performance.TIME_PROCESS) et de `soldiers.update` par image mesurée.
var _bench_process_ms: Array = []
## PB3e : durée de `_process` (scripts + pont) des images avec un pas de simulation et des autres.
var _bench_step_frame_ms: Array = []
var _bench_plain_frame_ms: Array = []
var _bench_plain_soldiers_ms: Array = []  # RJ-b : figurines, images sans pas de simulation
var _bench_proc_start_us: int = 0
var _bench_ticks_before: int = -1
var _bench_soldiers_ms: Array = []
var _bench_soldiers_last_ms: float = 0.0
var _bench_tick_ms: Array = []  # PB3c : durée de `BattleSim.tick` (pas de simulation) par image
var _bench_tick_last_ms: float = 0.0
## Compteur d'images mesurées (GPU/CPU/A-B) qui ne repart pas à zéro entre répétitions
## (`--bench-repeat=`), contrairement à `_bench_frames` (fenêtre de mesure courante).
var _bench_measured: int = 0
var _bench_gpu_samples: int = 0
## V3 : `--bench-ab=<niveau>,<niveau>` alterne deux niveaux de `RenderQuality` toutes les 30 images
## pendant la mesure (même charge machine pour les deux), temps GPU médian par niveau.
var _bench_ab: PackedStringArray = []
var _bench_ab_ms: Dictionary = {}
var _pad_units: int = 0
var _scale_tier: String = ""  # EP1 : palier d'échelle forcé (`--scale=`), sinon selon l'effectif
var _closeup: bool = false
var _closeup_distance: float = 26.0  # FG5 : `--closeup-distance=<m>` (captures du LOD0 par soldat)
var _shot_at: float = -1.0  # B4 : `--shot-at=<s>`
var _standard_side: String = ""  # DA1b : `--standard-side=` (camp cadré par `--standard-shot`)
var _standard_shot: String = ""  # EP5 : `--standard-shot=<foot|mounted|line|fallen|captured>`
var _ep12_shot: String = ""  # EP12 : `--ep12-shot=<wounded|rout>`
var _ep12_focus: Variant = null  # EP12 : point cadré (blessé) ou id du régiment en déroute
var _ep12_ticks: int = -1
var _weather_override: String = ""
var _camera_override: String = ""
var deployment: DeploymentController = null  # F5c : phase de déploiement du joueur
var _deploy_shot: bool = false
## Q4 : `--open-shot` capture la vue d'ouverture (caméra de `_frame_camera`), sans rien jouer.
var _open_shot: bool = false
var _sortie_shown: bool = false
var music: BattleMusicDirector = null  # B3 : musique dynamique par intensité
var battle_audio: BattleAudio = null  # AU1 : sons spatialisés (mêlée, volées, siège, météo)
var voices: BattleVoices = null  # VO1 : répliques des régiments
var _siege_audio_timer: float = 0.0
var _audio_director: Node = null  # B3 : mis en veille pendant la bataille, réveillé au retour
var staging: BattleStaging = null  # EP8 : heure, nuages, fumées, oiseaux, plan cinématique
var _hour_override: String = ""  # EP8 : `--hour=`
var _ep8_disabled: Dictionary = {}  # EP8 : `--no-<effet>`
var _force_cinematic: bool = false  # EP8 : `--cinematic`
var _birds_shot: bool = false  # EP8 : `--birds-shot`
var _cinematic_shot: bool = false  # EP8 : `--cinematic-shot`
var _dust_shot: bool = false  # EP8 : `--dust-shot` (charge de cavalerie et sa poussière)
var _dust_unit: int = -1
var _dust_since: float = -1.0
var _title_text: String = ""
## EP7 : bataille historique (`--historical=<id>`, `--historical-side=<attacker|defender>`, vide :
## IA contre IA) ; `historical` = `BattleSim.get_historical()` (vide hors carte historique).
var _historical: String = ""
var _historical_side: String = ""
var historical: Dictionary = {}
var _sim_weather: String = ""
var _weather_poll: float = 0.0
var _weather_text: String = ""
var _tod_key: String = ""
var _tod_clock: String = ""  # EP8b : dernière heure affichée au bandeau (« Midi, 11 h 00 »)
## EP13 : rejeu d'après bataille. `--replay=<fichier>` (menu « Rejeux ») ou « Revoir la bataille »
## sur l'écran de fin : le cœur re-simule la bataille enregistrée, la scène la montre sans ordre.
var replay_mode: bool = false
## PB3e : pas de simulation calculé sur un fil (`--no-pb3e` : synchrone, mesures A/B).
var pb3e_enabled: bool = not OS.get_cmdline_user_args().has("--no-pb3e")
var _step_thread_on: bool = false
## RJ-b : figurines et régiments interpolés entre deux pas de simulation (démarche continue) ;
## `--no-pose-lerp` après `--` : poses du pas courant (mesures A/B). Coupé en headless (tests).
var pose_lerp_enabled: bool = not OS.get_cmdline_user_args().has("--no-pose-lerp")
var _pose_lerp_on: bool = false
var replay_bar: BattleReplayBar = null
var replay_error: String = ""
var replay_saved_path: String = ""  # fichier écrit à la fin de la bataille (vide : non enregistré)
var _replay_path: String = ""
var _leader_bar: CanvasLayer = null
## CB0 : dictionnaires passés à `issue()`, pour le test d'équivalence des entrées
## (`cb0_input_equivalence_test.gd`). Rempli seulement si `log_orders_for_test` est vrai (mis par
## le test) ou sous la fonctionnalité moteur « test ».
var issued_log: Array = []
var log_orders_for_test: bool = false

@onready var terrain: BattleTerrain = $Terrain
@onready var camera_rig: BattleCamera = $CameraRig
@onready var hud: BattleHud = $HUD
@onready var world_env: WorldEnvironment = $WorldEnvironment
@onready var sun: DirectionalLight3D = $Sun
## CB0 : entrées (clics, glisser, touches, groupes), nœud enfant créé au premier `_ready`.
var input: BattleInput = null
## CB3 : vue tactique (Tab), nœud enfant créé au premier `_ready` comme `input`.
var tactical_view: BattleTacticalView = null


## À appeler avant `add_child` quand la bataille vient de la campagne.
func configure(p_campaign_sim: Object, index: int, seed: int) -> void:
	campaign_sim = p_campaign_sim
	battle_index = index
	battle_seed = seed


func _ready() -> void:
	_parse_cmdline()
	# CB0 : nœud d'entrées, connecté aux appels encore portés par la scène (ordres, rendu, pont).
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
	# CB3 : vue tactique (Tab).
	tactical_view = BattleTacticalView.new()
	tactical_view.name = "BattleTacticalView"
	tactical_view.scene = self
	add_child(tactical_view)
	hud.card_clicked.connect(_on_card_clicked)
	hud.card_double_clicked.connect(_on_card_double_clicked)
	hud.card_hovered.connect(func(id: int) -> void: card_hover = id)  # CB-M2 : contour au survol
	hud.command_pressed.connect(input._on_command)
	hud.ability_pressed.connect(input.use_card_ability)  # CB4 : bouton de capacité d'une carte
	hud.speed_pressed.connect(_on_speed_pressed)
	hud.minimap_clicked.connect(_on_minimap_clicked)
	hud.leader_clicked.connect(_on_leader_clicked)  # UB1 : sceau du chef
	hud.alerts_column.pinged.connect(_on_alert_pinged)  # CB5
	_drag_rect = ColorRect.new()
	_drag_rect.color = Color(0.95, 0.8, 0.3, 0.18)
	_drag_rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_drag_rect.visible = false
	hud.add_child(_drag_rect)
	if _replay_path != "":
		# EP13 : rejeu d'un fichier (menu « Rejeux »), hors campagne.
		standalone = true
		if not begin_replay(_replay_path):
			push_error("BattleScene: replay %s failed: %s" % [_replay_path, replay_error])
			_replay_failed()
		return
	if campaign_sim == null and not custom_config.is_empty():
		_custom = custom_config
	custom_config = {}
	if campaign_sim == null and not prologue_data.is_empty() and not _custom.is_empty():
		_prologue_data = prologue_data
	prologue_data = {}
	if campaign_sim == null and not _custom.is_empty():
		# NT2 : bataille personnalisée (menu « Bataille personnalisée »), hors campagne.
		standalone = true
		if not begin_custom(_custom):
			push_error("BattleScene: custom battle setup failed")
			if _benchmark:
				_bench_fail("custom battle setup failed")
		elif not _prologue_data.is_empty():
			_start_prologue()
		return
	if campaign_sim == null and _historical != "":
		# EP7 : carte historique jouée hors campagne (menu « Batailles historiques »).
		standalone = true
		if not begin_historical():
			push_error("BattleScene: historical battle %s failed" % _historical)
			if _benchmark:
				_bench_fail("historical battle setup failed")
		return
	if campaign_sim == null:
		standalone = true
		if not _stage_standalone():
			push_error("BattleScene: cannot stage a demo battle")
			# T8 : un banc d'essai qui ne peut pas se lancer doit échouer bruyamment (JSON +
			# code de sortie ≠ 0) plutôt que laisser une fenêtre ouverte sans jamais quitter
			# (l'une des causes des exécutions « sans résultat », cf. docs/wip/t2-perf.md).
			if _benchmark:
				_bench_fail("cannot stage a demo battle")
			return
	if not begin():
		push_error("BattleScene: battle setup failed")
		if _benchmark:
			_bench_fail("battle setup failed")


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
	if siege_demo and siege_landmark != "" and sim.has_method("debug_stage_landmark_siege"):
		var besieger: String = armies[1] if siege_attacker == "fac_england" else armies[0]
		index = sim.call("debug_stage_landmark_siege", besieger, siege_landmark)
	elif siege_demo and sim.has_method("debug_stage_siege"):
		index = sim.call("debug_stage_siege", armies[0], siege_province)
		if index < 0:
			# L3 : une ville française (Paris, Rouen) est assiégée par l'armée anglaise.
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
	BattleTerrain.apply_site_overrides(setup)  # B5 : --terrain= --season= --village --coast
	# B3 : la musique de campagne cède la place à la musique de bataille (réveillée au retour).
	_audio_director = get_node_or_null("/root/AudioDirector")
	if _audio_director != null:
		_audio_director.call("stop_all")
	if _pad_units > 0:
		_pad_setup(_pad_units)
	if _siege_engines != "" and setup.get("siege") != null:
		SiegeAssaultFx.add_engines(setup, _siege_engines.split(",", false))  # SG1 : captures, banc
		padded = true  # régiments hors campagne : pas de résultat à rapporter
	battle = ClassDB.instantiate("BattleSim")
	if _scale_tier != "" and battle.has_method("set_scale_tier"):
		battle.call("set_scale_tier", _scale_tier)  # EP1 : --scale=<skirmish|large|epic>
	if not battle.call("setup", setup, battle_seed):
		return false
	if _hour_override != "" and battle.has_method("set_start_hour"):
		# EP8 : heure de début imposée (bataille rapide, captures) ; la règle reste au cœur.
		if _hour_override.is_valid_float():
			battle.call("set_start_hour", float(_hour_override))
		elif not battle.call("set_start_phase", _hour_override):
			push_warning("BattleScene: unknown --hour=%s" % _hour_override)
	return _build_scene()


## EP7 : bataille historique `_historical` (site réel, ordres de bataille, météo, heure) ; le
## résultat n'est pas conservé (hors campagne).
func begin_historical() -> bool:
	if not ClassDB.class_exists("BattleSim"):
		return false
	_audio_director = get_node_or_null("/root/AudioDirector")
	if _audio_director != null:
		_audio_director.call("stop_all")
	battle = ClassDB.instantiate("BattleSim")
	if not battle.has_method("setup_historical"):
		return false
	var paths := get_node_or_null("/root/MapPaths")
	var data_dir: String = paths.data_dir if paths != null else ProjectSettings.globalize_path("res://").path_join("../data").simplify_path()
	if not battle.call("setup_historical", data_dir, _historical, _historical_side, battle_seed):
		return false
	setup = battle.call("get_setup")
	padded = true  # hors campagne : pas de résultat à rapporter
	if _hour_override != "":
		if _hour_override.is_valid_float():
			battle.call("set_start_hour", float(_hour_override))
		else:
			battle.call("set_start_phase", _hour_override)
	return _build_scene()


## NT2 : bataille personnalisée `config` (armées achetées, champ, siège) ; le résultat n'est pas
## conservé (hors campagne).
func begin_custom(config: Dictionary) -> bool:
	if not ClassDB.class_exists("BattleSim"):
		return false
	_audio_director = get_node_or_null("/root/AudioDirector")
	if _audio_director != null:
		_audio_director.call("stop_all")
	battle = ClassDB.instantiate("BattleSim")
	if not battle.has_method("setup_custom"):
		return false
	# Un champ différent à chaque bataille, sauf graine imposée (`seed` : tests, captures).
	battle_seed = int(config.get("seed", randi() % 1000000))
	if not battle.call("setup_custom", CustomBattleScreen.data_dir(), config, battle_seed):
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
	var attacker := str((setup.get("attacker", {}) as Dictionary).get("faction_name", ""))
	var defender := str((setup.get("defender", {}) as Dictionary).get("faction_name", ""))
	_title_text = "Bataille personnalisée : %s contre %s" % [attacker, defender]
	if not _prologue_data.is_empty():
		_title_text = str(_prologue_data.get("title", _title_text))  # NT4
	hud.set_title(_title_text, _weather_text, [side_colors[player_side], side_colors[enemy_side]])
	return true


## NT4 : branche le guide de la bataille-prologue (étapes, ennemi passif au début).
func _start_prologue() -> void:
	prologue = BattlePrologue.new()
	prologue.name = "BattlePrologue"
	prologue.setup(_prologue_data)
	add_child(prologue)
	prologue.attach(self)


## EP13 : rejeu du fichier `path` ; `replay_error` dit pourquoi en cas d'échec (autre format,
## bataille impossible à reconstruire).
func begin_replay(path: String) -> bool:
	if not ClassDB.class_exists("BattleSim"):
		replay_error = "extension absente"
		return false
	_audio_director = get_node_or_null("/root/AudioDirector")
	if _audio_director != null:
		_audio_director.call("stop_all")
	battle = ClassDB.instantiate("BattleSim")
	if not battle.has_method("load_replay"):
		replay_error = "extension trop ancienne"
		return false
	var result: Dictionary = battle.call("load_replay", path)
	if not bool(result.get("ok", false)):
		replay_error = str(result.get("error", "?"))
		battle = null
		return false
	setup = battle.call("get_setup")
	padded = true  # hors campagne : rien à rapporter
	replay_mode = true
	if not _build_scene():
		return false
	var info: Dictionary = battle.call("get_replay")
	if str(info.get("title", "")) != "":
		_title_text = str(info["title"])
		hud.set_title(_title_text, _weather_text, [side_colors[player_side], side_colors[enemy_side]])
	_enter_replay()
	return true


## EP13 : le rejeu ne peut être lu : message, puis retour au menu principal.
func _replay_failed() -> void:
	var dialog := AcceptDialog.new()
	dialog.title = "Rejeu"
	dialog.dialog_text = "Ce rejeu ne peut être revu : %s." % replay_error
	dialog.confirmed.connect(func() -> void: SceneFader.go("res://scenes/start_menu.tscn"))
	dialog.canceled.connect(func() -> void: SceneFader.go("res://scenes/start_menu.tscn"))
	hud.add_child(dialog)
	dialog.popup_centered()


## EP13 : passe la scène en rejeu (barre de rejeu, ordres, déploiement et vitesses de combat cachés).
func _enter_replay() -> void:
	replay_mode = true
	paused = false
	speed = 1.0
	selected.clear()
	if deployment != null:
		deployment.queue_free()
		deployment = null
	if _leader_bar != null:
		_leader_bar.queue_free()  # ordres du chef : rien à ordonner pendant un rejeu
		_leader_bar = null
	if hud.withdraw_all_button != null:
		hud.withdraw_all_button.get_parent().visible = false  # ordres et retraite générale
	if not hud._speed_buttons.is_empty():
		hud._speed_buttons[0].get_parent().visible = false  # la barre de rejeu a ses vitesses
	if hud.toast_label != null:
		hud.toast_label.get_parent().visible = true
	replay_bar = BattleReplayBar.new()
	hud.root.add_child(replay_bar)
	var info: Dictionary = battle.call("get_replay")
	replay_bar.setup(_title_text, float(info.get("duration", 0.0)))
	replay_bar.play_toggled.connect(replay_toggle_play)
	replay_bar.speed_chosen.connect(replay_set_speed)
	replay_bar.seek_requested.connect(replay_seek)
	replay_bar.quit_pressed.connect(_on_return)
	hud.add_events([{"time": float(battle.call("get_elapsed")), "text_fr": "Rejeu de la bataille : on regarde, on ne commande pas."}])
	var divergence: Dictionary = info.get("divergence", {})
	if not divergence.is_empty():
		hud.add_events([{"time": 0.0, "text_fr": str(divergence.get("message", ""))}])
	_update_replay()


## EP13 : état de la barre de rejeu ; pause d'elle-même à la fin de l'enregistrement.
func _update_replay() -> void:
	if replay_bar == null or battle == null:
		return
	var info: Dictionary = battle.call("get_replay")
	if bool(info.get("at_end", false)) and not paused:
		paused = true
	replay_bar.show_state(float(battle.call("get_elapsed")), paused, speed, info.get("divergence", {}))


## EP13 : lecture / pause ; « Lecture » à la fin repart du début.
func replay_toggle_play() -> void:
	if paused and bool((battle.call("get_replay") as Dictionary).get("at_end", false)):
		replay_seek(0.0)
	paused = not paused
	_update_replay()


func replay_set_speed(value: float) -> void:
	speed = value
	paused = false
	_update_replay()


## EP13 : saut dans la barre de temps (le cœur repart de l'instantané le plus proche). Un saut en
## arrière reconstruit les figurines et effets (sang, traits, corps) pour ne pas montrer l'avenir.
func replay_seek(seconds: float) -> void:
	if battle == null or not replay_mode:
		return
	var before := float(battle.call("get_elapsed"))
	battle.call("replay_seek", seconds)
	var after := float(battle.call("get_elapsed"))
	if after < before - 0.05:
		_reset_battle_visuals()
	_refresh_view(true)
	hud.add_events([{"time": after, "text_fr": "Rejeu : saut à %s." % BattleReplayBar.clock(after)}])
	_update_replay()


## EP13 : figurines, étendards, effets, sang et herbe couchée refaits à neuf (après un saut en
## arrière ou au début d'un rejeu lancé depuis l'écran de fin).
func _reset_battle_visuals() -> void:
	units = battle.call("get_units")
	# Les imposteurs survivent au saut : atlas déjà cuits gardés, et une cuisson en cours (coroutine
	# sur ce nœud) ne reprend jamais sur une instance libérée.
	var kept_impostors: BattleImpostors = null
	if soldiers != null and is_instance_valid(soldiers) and soldiers.impostors != null and is_instance_valid(soldiers.impostors):
		kept_impostors = soldiers.impostors
		soldiers.remove_child(kept_impostors)
		soldiers.impostors = null
	for node in [soldiers, standards, duels, effects, engines_fx, assault_fx]:
		if node != null and is_instance_valid(node):
			(node as Node).get_parent().remove_child(node)
			(node as Node).queue_free()
	standards = null
	duels = null
	effects = null
	blood = null
	engines_fx = null
	assault_fx = null
	grass_flatten = null
	_build_soldier_layers(kept_impostors)


## EP13 : « Revoir la bataille » depuis l'écran de fin (résultat déjà appliqué à la campagne).
func start_replay_in_place() -> bool:
	if battle == null or not battle.has_method("start_replay"):
		return false
	var result: Dictionary = battle.call("start_replay")
	if not bool(result.get("ok", false)):
		push_warning("BattleScene: start_replay: %s" % result.get("error", "?"))
		return false
	if result_screen != null:
		result_screen.queue_free()
		result_screen = null
	_reset_battle_visuals()
	_enter_replay()
	_refresh_view(true)
	return true


## EP13 : enregistre la bataille (dossier utilisateur, N derniers gardés par le cœur). Pas pendant
## un rejeu, ni en banc d'essai ou capture ; en mode sans affichage (tests) seulement si un dossier
## de test est imposé (`ReplaysMenu.dir_override`).
func _save_replay() -> void:
	if replay_mode or _benchmark or _screenshot_path != "" or battle == null or not battle.has_method("save_replay"):
		return
	if DisplayServer.get_name() == "headless" and ReplaysMenu.dir_override == "":
		return
	replay_saved_path = str(battle.call("save_replay", ReplaysMenu.replays_dir(), _title_text))


## Scène de bataille (terrain, soldats, interface) une fois `battle` et `setup` prêts.
func _build_scene() -> bool:
	var setup_side: Variant = setup.get("player_side", "attacker")
	player_side = str(setup_side) if setup_side != null else ""
	if player_side == "" and standalone and siege_landmark != "":
		# SG2 : démo d'Avignon ou de Bruges, sans le joueur de la campagne : il mène l'assaut.
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
		# DA1 : maison du général (armes du HUD et des figurines nobles), rendu seulement.
		var general: Variant = side_setup.get("general", null)
		if general is Dictionary and not (general as Dictionary).has("house"):
			(general as Dictionary)["house"] = HouseArms.house_of(str((general as Dictionary).get("character", "")), campaign_sim)
	var weather: Dictionary = battle.call("get_weather")
	var weather_key := _weather_override if _weather_override != "" else str(weather.get("key", "clear"))
	# EP7 : une carte historique est rendue sous son ciel final ; l'averse du début tombe de ce
	# ciel et cesse quand la simulation change de météo (Crécy).
	historical = battle.call("get_historical") if battle.has_method("get_historical") else {}
	_sim_weather = str(weather.get("key", "clear"))
	if not historical.is_empty() and _weather_override == "":
		weather_key = str(historical.get("weather_end", weather_key))
	_weather_key = weather_key
	var terrain_data: Dictionary = battle.call("get_terrain")
	terrain.province_id = str(setup.get("province", ""))  # EP2 : relief réel et panorama du lieu
	# EP7 : tuile d'horizon du site historique (campagne sur le site ou carte du menu).
	terrain.horizon_site = str(historical.get("horizon", setup.get("historical_horizon", "")))
	terrain.build(terrain_data, weather_key)
	if terrain.decor_view != null:
		terrain.decor_view.bind(battle)  # EP6 : pillage des camps
	if terrain_data.has("siege"):
		siege_view = BattleSiege.new()
		siege_view.name = "Siege"
		siege_view.side_colors = side_colors  # SB : barres de vie à la couleur des camps
		add_child(siege_view)
		siege_view.build(terrain_data["siege"], func(x: float, z: float) -> float: return terrain.height_at(x, z))
		# L1 : ville emblématique (Paris) en toile de fond derrière la ville assiégée.
		var backdrop := LandmarkBackdrop.create(setup, terrain_data["siege"], func(x: float, z: float) -> float: return terrain.height_at(x, z))
		if backdrop != null:
			add_child(backdrop)
		# L3 : ville assiégée tirée du plan (rues pavées ; murailles et maisons viennent du cœur).
		if battle.has_method("get_siege_landmark"):
			landmark_town = LandmarkSiegeTown.create(battle.call("get_siege_landmark"), func(x: float, z: float) -> float: return terrain.height_at(x, z))
			if landmark_town != null:
				add_child(landmark_town)
	# PO4 : heure de rendu (phase du cœur, sinon graine) ; EP8 anime la lumière si l'heure avance.
	var tod: Dictionary = battle.call("get_time_of_day") if battle.has_method("get_time_of_day") else {}
	var time_key := BattleAtmosphere.time_key_for(tod, battle_seed, weather_key)
	BattleAtmosphere.apply(world_env, sun, weather_key, camera_rig.camera, terrain.season_key, time_key, not tod.is_empty() and not _ep8_disabled.has("daytime"))
	if _sim_weather == "rain" and weather_key != "rain":
		BattleAtmosphere.add_shower(camera_rig.camera)  # EP7 : averse qui cessera
	if terrain.horizon != null:
		terrain.horizon.apply_atmosphere(world_env.environment, sun, weather_key)  # EP2
	var field_center := terrain.field_center()  # EP1 : (600, 400) au palier standard
	BattleAtmosphere.add_ground_mist(self, weather_key, Vector3(field_center.x, terrain.height_at(field_center.x, field_center.y), field_center.y), Vector2(terrain.FIELD_W + 300.0, terrain.FIELD_D + 300.0))
	# BV1 (ADR 0016) : taille des unités = figurines par homme simulé (rendu seulement).
	if not _no_bv1:
		battle.call("set_figure_scale", _figure_scale())
	_open_deployment()
	units = battle.call("get_units")
	_build_soldier_layers()
	for unit in units:
		_make_banner(unit)
	outlines = BattleFormationOutline.new()
	add_child(outlines)
	outlines.setup(side_colors, player_side)
	path_preview = BattlePathPreview.new()
	add_child(path_preview)
	path_preview.setup(battle, func(x: float, z: float) -> float: return terrain.height_at(x, z), side_colors.get(player_side, Color(0.9, 0.8, 0.3)))
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
		# EP7 : « Bataille de Crécy (26 août 1346) » ; sur le site en campagne : « … , champ de Crécy ».
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
		weather_label = str(historical["weather_label"])  # EP7 : « Averse d'orage, puis… »
	elif weather_key != str(weather.get("key", "clear")):
		weather_label += " (rendu forcé : %s)" % weather_key
		print("BattleScene: --weather=%s overrides rendering only; simulated weather is %s" % [weather_key, weather.get("key", "?")])
	_title_text = title
	_weather_text = weather_label
	hud.set_title(title, weather_label, [side_colors[player_side], side_colors[enemy_side]])
	hud.set_site(str(terrain_data.get("site_label", "")))
	if battle.has_method("get_opening"):
		hud.set_opening(battle.call("get_opening"), player_side)  # CV3-2
	hud.player_faction = str((setup[player_side] as Dictionary).get("faction", ""))
	hud.set_leader((setup[player_side] as Dictionary).get("general", null), hud.player_faction)
	# CB4 : textes des capacités pour les infobulles des boutons de carte.
	if battle.has_method("get_ability_catalog"):
		hud.ability_catalog = battle.call("get_ability_catalog")
	# RJ-a : formations de régiment (menu du bouton « Formation », infobulles).
	if battle.has_method("unit_formations"):
		hud.setup_formations(battle.call("unit_formations"), battle.call("formation_reform_rules"))
	camera_rig.height_at = func(x: float, z: float) -> float: return terrain.world_height(x, z)
	camera_rig.bounds = Rect2(-150, -150, terrain.FIELD_W + 300.0, terrain.FIELD_D + 300.0)  # EP1
	# EP1 : recul maximal selon la largeur du champ (900 m au standard, 1350 m à 2400 m).
	camera_rig.max_distance = 900.0 * (0.5 + 0.5 * maxf(terrain.field_scale_x(), 1.0))
	_frame_camera()
	_setup_staging(terrain_data)
	hud.minimap.flipped = player_side == "attacker"
	hud.minimap.setup(terrain_data, side_colors)
	hud.add_events(battle.call("get_events"))
	_leader_bar = LEADER_ORDERS_BAR.new(self)
	add_child(_leader_bar)
	if compare_panel != null:
		compare_panel.above = _leader_bar.panel
	music = BATTLE_MUSIC.new()
	music.name = "Music"
	add_child(music)
	music.setup(self)
	battle_audio = BattleAudio.new()
	add_child(battle_audio)
	battle_audio.setup(_weather_key, camera_rig.camera)
	hud.alerts_column.battle_audio = battle_audio  # CB5 : cris (déroute, général tombé)
	voices = BattleVoices.new()  # VO1
	add_child(voices)
	voices.setup(self)
	_refresh_view(true)
	if not replay_mode:  # EP13 : pas de discours ni de conseil pendant un rejeu
		_start_speech()
		_advise_first_battle()
	return true


## EP8 : mise en scène (heure, nuages, poussière, fumées, oiseaux, plan cinématique).
func _setup_staging(terrain_data: Dictionary) -> void:
	staging = BattleStaging.new()
	add_child(staging)
	var decor_view: BattleDecor = terrain.decor_view
	if decor_view != null and decor_view.has_camps():
		staging.auto_campfires = false  # EP6 : les camps du décor portent leurs feux
	staging.setup(self, battle, world_env.environment, sun, _weather_key, terrain_data, _ep8_disabled)
	staging.configure_effects(effects, str(terrain_data.get("terrain", "plains")), terrain.season_key)
	if decor_view != null:
		decor_view.attach_smoke(self, staging, _weather_key)
	if effects != null:
		effects.cannon_fired.connect(staging.on_cannon_fired)
	if staging.cinematic != null and not _force_cinematic and (autoplay or _benchmark or _screenshot_path != ""):
		# Rien d'imposé : jamais en banc d'essai, en capture ni quand l'IA joue les deux camps.
		staging.cinematic.enabled = false
	elif staging.cinematic != null and _force_cinematic:
		staging.cinematic.enabled = true
	_update_time_label()


## EP8 : captures `--birds-shot` (caméra basse tournée vers les volées) et `--cinematic-shot`
## (plan cinématique figé à mi-course, bandes noires comprises).
func _apply_staging_shot() -> void:
	if staging == null:
		return
	if _dust_shot and _dust_unit >= 0:
		for unit in units:
			if int(unit["id"]) == _dust_unit:
				var facing := float(unit["facing"])
				# De trois quarts avant, un peu à l'écart de la trajectoire.
				camera_rig.look_at_point(Vector3(float(unit["x"]), 0.0, float(unit["z"])), 55.0, facing + 0.9)
	if _birds_shot and staging.birds != null:
		var sky := staging.birds.flying_center()
		if sky != Vector3.ZERO:
			camera_rig.set_process(false)
			var ground := Vector3(sky.x, 0.0, sky.z)
			var back := Vector3(float(units[0]["x"]), 0.0, float(units[0]["z"])) - ground if not units.is_empty() else Vector3(0, 0, -1)
			back.y = 0.0
			var eye := ground + back.normalized() * 50.0
			eye.y = terrain.world_height(eye.x, eye.z) + 3.0
			camera_rig.camera.global_position = eye
			camera_rig.camera.look_at(Vector3(sky.x, terrain.world_height(sky.x, sky.z) + 14.0, sky.z), Vector3.UP)
	if _cinematic_shot and staging.cinematic != null:
		selected.clear()
		_refresh_view(true)
		var shot := _closeup_shot(units)
		staging.cinematic.start(shot["focus"], float(shot["yaw"]))
		staging.cinematic.pose_at(float(staging.cinematic.cfg.get("duration_s", 6.0)) * 0.5)


## EP8 : aucun point d'eau (rivière, gué, ruisseau) à moins de `radius` m (capture de poussière).
func _dry_around(x: float, z: float, radius: float) -> bool:
	for k in 9:
		var a := TAU * k / 8.0
		var r := 0.0 if k == 8 else radius
		if terrain.in_water(x + cos(a) * r, z + sin(a) * r):
			return false
	return true


## EP8 : pose une source de fumée durable (EP6 : feux des camps) ; -1 si coupée ou hors budget.
func add_smoke_source(position: Vector3, intensity: float = 1.0, kind: String = "campfire") -> int:
	return staging.add_smoke_source(position, intensity, kind) if staging != null else -1


## EP8 : l'heure au bandeau (« Temps clair · Crépuscule, 18 h 40 (portée des tireurs −30 %) ») ;
## EP8b (ADR 0055) : l'heure elle-même s'affiche (`BattleTimeOfDay.clock_label`), pas seulement le
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


## NT6a : le conseiller parle après le discours adverse s'il est en cours.
func _advise_after_speeches(trigger: String) -> void:
	if enemy_speech != null and is_instance_valid(enemy_speech) and enemy_speech.active:
		enemy_speech.finished.connect(func() -> void: Advisor.say_trigger(trigger), CONNECT_ONE_SHOT)
	else:
		Advisor.say_trigger(trigger)


## VO1 : le conseiller commente la première bataille (ou le premier assaut), après le discours.
func _advise_first_battle() -> void:
	if autoplay or _benchmark or not _prologue_data.is_empty():  # NT4 : le guide parle déjà
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
	if not _no_bv3 and not _no_impostors:
		# BV3 : imposteurs lointains, cuits au début de la bataille (ADR 0024).
		soldiers.impostors = kept_impostors if kept_impostors != null else BattleImpostors.new()
		soldiers.impostors.name = "Impostors"
		soldiers.add_child(soldiers.impostors)
	elif kept_impostors != null:
		kept_impostors.queue_free()
	var factions := {}
	var houses := {}  # DA1 : maison du général par camp
	for side in ["attacker", "defender"]:
		factions[side] = str((setup[side] as Dictionary).get("faction", ""))
		var general: Variant = (setup[side] as Dictionary).get("general", null)
		houses[side] = str((general as Dictionary).get("house", "")) if general is Dictionary else ""
	_side_houses = houses
	# AN1a : vent de la bataille (le même que celui des drapeaux) pour le mouvement secondaire.
	BattleSecondaryMotion.set_wind(BattleStandards.wind_for(_weather_key, battle_seed))
	soldiers.setup(units, side_colors, factions, houses)
	_mm = soldiers.layers
	_setup_standards()
	BattleAudio.auto_volley = true  # BV1 : repris ci-dessous par les tirs du cœur (effets actifs)
	if _no_effects:
		return
	effects = BattleEffects.new()
	effects.name = "Effects"
	add_child(effects)
	# EP3 : l'eau à la largeur locale de la rivière, et les ruisseaux.
	effects.setup(_weather_key, func(x: float, z: float) -> float: return terrain.world_height(x, z), func(x: float, z: float) -> int: return 1 if terrain.in_water(x, z) else 0)
	if not _no_bv1:
		effects.configure_ground(str(terrain.terrain.get("ground", "dry")), _weather_key)
	effects.volleys.figure_scale = float(battle.call("get_figure_scale"))
	effects.volleys.sound_event.connect(_on_sound_event)
	effects.sound_event.connect(_on_sound_event)
	BattleAudio.auto_volley = _no_bv1
	blood = BattleBlood.new()
	blood.name = "Blood"
	effects.add_child(blood)
	blood.figure_scale = effects.volleys.figure_scale
	blood.setup(func(x: float, z: float) -> float: return terrain.world_height(x, z), BattleBlood.OFF if _no_bv1 else _blood_level(), func(x: float, z: float) -> int: return 1 if terrain.in_water(x, z) else 0)
	effects.hit_landed.connect(func(pos: Vector3, time: float) -> void: blood.add_hit(pos, time, _camera_position()))
	# Fusion BV1/BV2 : les morts de BV2 portent la flaque au sol (BV1) et les traits fichés dans
	# les corps ; la gerbe reste à BV2 (`BattleGore`), une seule source par événement.
	if soldiers.bv2_enabled and not _no_bv1:
		blood.corpse_driven = true
		soldiers.corpse_fallen.connect(func(pos: Vector3, side: String, kind: String, cause: String) -> void:
			blood.on_corpse(pos, side, kind, cause, _camera_position())
			effects.volleys.on_corpse(pos, side, kind, cause))
	# SG2 : engins animés (trébuchet, mangonneau, bombarde, roues du bélier et du beffroi).
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


## BV3 : vent de la météo (drapeaux, herbe) et porte-étendards des régiments.
func _setup_standards() -> void:
	if _no_bv3:
		return
	var wind := BattleStandards.wind_for(_weather_key, battle_seed)
	standards = BattleStandards.new()
	standards.name = "Standards"
	add_child(standards)
	var factions := {}
	for side in ["attacker", "defender"]:
		factions[side] = str((setup.get(side, {}) as Dictionary).get("faction", ""))
	standards.setup(units, side_colors, func(unit: Dictionary) -> Dictionary: return _banner_cloth(unit, str((setup[str(unit["side"])] as Dictionary).get("faction", ""))), wind, factions, battle, _side_houses)
	for id in _banners:
		standards.apply_wind((_banners[id] as Dictionary)["flag_mat"])
	if terrain.vegetation != null:
		terrain.vegetation.set_wind(wind["dir"], float(wind["strength"]) * float(wind["grass_scale"]))
	if soldiers.bv2_enabled:
		duels = BattleDuels.new()
		duels.name = "Duels"
		add_child(duels)
		duels.setup()


## BV3 : discours du général du joueur, au début du déploiement (ou de la bataille), en jeu
## seulement (pas en `--autoplay`, captures ni bancs), sauf `--speech-shot`.
func _start_speech() -> void:
	if _no_bv3 or _no_speech or (autoplay and _speech_shot == "") or not _prologue_data.is_empty():
		return
	var ours := float(battle.call("get_strength", player_side))
	var theirs := maxf(float(battle.call("get_strength", enemy_side)), 1.0)
	var text := BattleSpeech.compose(setup, player_side, ours / theirs, terrain.terrain_key, _weather_key, battle_seed)
	speech = BattleSpeech.new()
	speech.name = "Speech"
	speech.shot_path = _speech_shot
	speech.shot_at = _speech_at
	add_child(speech)
	if not speech.start(self, text, units, player_side):
		speech.queue_free()
		speech = null
		return
	if _speech_shot == "":
		speech.finished.connect(_start_enemy_speech, CONNECT_ONE_SHOT)


## NT6a : discours du général adverse, joué juste après celui du joueur (même réglage
## `--no-speech`) ; un « passer » du joueur écarte aussi celui de l'adversaire.
func _start_enemy_speech() -> void:
	if speech == null or speech.skipped or _no_speech:
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


## BV3 : herbe couchée par les troupes et sous les corps, sang lisible en prairie ; pavois du
## dos masqué quand la rangée de BV1 est plantée.
func _setup_grass_flatten() -> void:
	if _no_bv3:
		return
	soldiers.hide_planted_pavise = not _no_bv1  # BV1 plante les rangées de pavois
	if terrain.vegetation == null:
		return
	grass_flatten = BattleGrassFlatten.new()
	grass_flatten.setup(Vector2(terrain.FIELD_W, terrain.FIELD_D))
	terrain.vegetation.set_flatten(grass_flatten)
	var blood_amount: float = 0.0 if blood == null else [0.0, 0.6, 1.0][blood.level]
	if soldiers.bv2_enabled:
		soldiers.corpse_fallen.connect(func(pos: Vector3, _side: String, kind: String, cause: String) -> void:
			grass_flatten.on_corpse(pos, kind, 0.0 if cause == "fire" else blood_amount))


## Réglages du joueur lus par la bataille (BV1) : `--unit-size=` / `--blood=` les forcent.
func _unit_size() -> float:
	if _unit_size_override > 0.0:
		return _unit_size_override
	var settings := get_node_or_null("/root/Settings")
	return float(settings.call("get_value", "battle/unit_size")) if settings != null else 1.0


## FB1 : taille des unités effective. Le multiplicateur choisi est abaissé pour que le total de
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


## BV1 : sons des tirs du cœur (lâcher, sifflement, impact) joués par l'API du lot AU1
## (`BattleAudio.play_at` / `play_at_delayed`, bus et banque sonore d'AU1). `delay` en temps de
## bataille ; `BattleAudio.auto_volley` est coupé pour ne pas doubler ses volées déduites des
## munitions.
func _on_sound_event(event: StringName, position: Vector3, delay: float) -> void:
	if delay > 0.0:
		BattleAudio.play_at_delayed(str(event), position, delay)
	else:
		BattleAudio.play_at(str(event), position)


## B4 : effets (poussière, traits…) d'après l'état des régiments ; `dt` = temps simulé écoulé.
func _update_effects(dt: float) -> void:
	terrain.update_trample(units, dt)  # B7 : neige piétinée (sans effet hors neige au sol)
	if duels != null:
		duels.update(units, soldiers, soldiers.anim_time, _camera_position())
	if grass_flatten != null:
		grass_flatten.update(units, dt)
	if effects == null:
		return
	var camera := get_viewport().get_camera_3d()
	var camera_pos := camera.global_position if camera != null else Vector3.ZERO
	var shots: Variant = battle.call("get_shots")
	if engines_fx != null:
		engines_fx.update(units, shots, soldiers.anim_time, dt)
	effects.update(units, soldiers, soldiers.anim_time, dt, camera_pos, null if _no_bv1 else shots)
	if blood != null:
		blood.tick_time(soldiers.anim_time)
		blood.update(units, camera_pos)
	if assault_fx != null:
		assault_fx.update(battle.call("get_siege_events"), units, soldiers.anim_time, dt)


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


## Étoffe d'un drapeau de régiment : bannière peinte de la faction (`heraldry/banners/`,
## 256×512, tissu dans le haut, bas transparent) pour la noblesse, fanion à queue d'aronde (4:1)
## pour les autres, étendards royaux pour le général de France / d'Angleterre ; à défaut, centre
## de l'écu de la faction (repli). DA1b : le général et les unités nobles de sa retenue portent
## la bannière de sa maison (`heraldry/banners/houses/`) quand elle existe.
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
	var house := HouseArms.id_of(str(_side_houses.get(str(unit.get("side", "")), "")))
	if house != "" and (bool(unit.get("is_general", false)) or BattleStandards.is_house_retinue(unit)):
		var house_banner: Array = [dir + "houses/%s_banner.png" % house, Vector2(1.3, 2.6)]
		var no_quarter_first := not candidates.is_empty() and (str(candidates[0][0]).ends_with("oriflamme.png") or str(candidates[0][0]).ends_with("dragon.png"))
		candidates.insert(1 if no_quarter_first else 0, house_banner)
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
	if siege_view != null and player_side == "attacker" and n > 0 and _frame_siege_camera(center):
		return
	var yaw := PI if player_side == "attacker" else 0.0
	# Regarder un peu devant sa propre ligne, vers l'ennemi.
	center.z += 70.0 if player_side == "attacker" else -70.0
	# A1-06 : vue d'ouverture plus basse et plus proche (on voit des hommes, pas des points).
	camera_rig.look_at_point(center, 170.0, yaw)


## Q4 (Q3 : caméra d'assaut cadrant un bélier sur une plaine vide) : l'assaillant ouvre derrière
## son armée, face à la porte, murailles et régiments dans le même plan.
func _frame_siege_camera(army: Vector3) -> bool:
	var siege: Dictionary = battle.call("get_siege") if battle.has_method("get_siege") else {}
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


# --- Boucle ---------------------------------------------------------------------------


func _process(delta: float) -> void:
	if battle == null:
		return
	# EP8 : ralenti du plan cinématique (temps de bataille et animations).
	var slow := staging.time_scale() if staging != null else 1.0
	var running: bool = not paused and not battle.call("is_finished")
	var tick_start := Time.get_ticks_usec()
	if _benchmark:
		_bench_proc_start_us = tick_start
		_bench_ticks_before = int(battle.call("get_ticks"))
	if running:
		_configure_step_thread()
		battle.call("tick", delta * speed * slow)
	_bench_tick_last_ms = float(Time.get_ticks_usec() - tick_start) / 1000.0
	_refresh_view(false, delta * slow)
	_update_hover_cursor()
	if music != null:
		music.update(delta, units)
	if staging != null:
		staging.update(units, delta * speed * slow if running else 0.0, delta, bool(battle.call("is_finished")))
		_update_time_label()
	_update_audio(delta)
	_poll_weather(delta)
	if replay_mode:
		_update_replay()  # EP13 : pas d'écran de fin pendant un rejeu
	elif battle.call("is_finished") and not finished_shown:
		_end_wait += delta
		if _end_wait >= _victory_hold():
			_show_end()
	if _benchmark:
		_run_benchmark_frame(delta)
		if _bench_measured > 10 and battle != null:
			var proc_ms := float(Time.get_ticks_usec() - _bench_proc_start_us) / 1000.0
			if int(battle.call("get_ticks")) != _bench_ticks_before:
				_bench_step_frame_ms.append(proc_ms)
			else:
				_bench_plain_frame_ms.append(proc_ms)
				_bench_plain_soldiers_ms.append(_bench_soldiers_last_ms)


## PB3e (ADR 0090) : le pas de simulation suivant se calcule sur un fil pendant que l'image
## montre le pas courant (même bataille, au bit près). Synchrone en headless (tests), pendant un
## rejeu et avec `--no-pb3e` après `--` (mesures A/B).
func _configure_step_thread() -> void:
	_configure_pose_lerp()
	var wanted := pb3e_enabled and not replay_mode and DisplayServer.get_name() != "headless"
	if wanted == _step_thread_on or not battle.has_method("set_step_thread"):
		return
	_step_thread_on = wanted
	battle.call("set_step_thread", wanted)


## RJ-b : poses interpolées entre deux pas (fraction du pas en cours, `step_fraction` du cœur).
## `--pose-lerp` force l'interpolation en headless (test, banc).
func _configure_pose_lerp() -> void:
	var forced := OS.get_cmdline_user_args().has("--pose-lerp")
	var wanted := pose_lerp_enabled and (forced or DisplayServer.get_name() != "headless")
	if wanted == _pose_lerp_on or not battle.has_method("set_pose_lerp"):
		return
	_pose_lerp_on = wanted
	battle.call("set_pose_lerp", wanted)


## EP7 : la météo d'une carte historique change pendant la bataille (averse de Crécy) : la pluie
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


## T8 : une image du banc d'essai (`--benchmark`). Fenêtre de `BENCH_FRAMES` images mesurées,
## répétée `_bench_repeat` fois (`--bench-repeat=`) ; les temps d'image de toutes les
## répétitions sont regroupés pour la médiane / p95 finales. Un budget de temps réel
## (`--bench-timeout=`, `_bench_wall_start_ms`) fait échouer proprement le banc (JSON + code de
## sortie ≠ 0) au lieu de bloquer indéfiniment si la simulation n'avance pas (120 régiments, cf.
## `docs/wip/t2-perf.md`). V3 : temps GPU/CPU mesurés et banc A/B (`--bench-ab=`, `_bench_ab_step`)
## sur un compteur dédié `_bench_measured` qui ne repart pas à zéro entre répétitions.
func _run_benchmark_frame(delta: float) -> void:
	if _bench_failed:
		return
	if _bench_wall_start_ms < 0:
		_bench_wall_start_ms = Time.get_ticks_msec()
	if _bench_frames == 0:
		if _bench_repeat_done == 0:
			# BV1 : l'autoload `Settings` réimpose la synchro verticale du joueur (60 Hz) ; le banc
			# d'essai la coupe pour que les FPS départagent enfin les variantes.
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
			Engine.max_fps = 0
			# V3 : temps GPU/CPU de rendu mesurés (l'écran plafonne souvent les FPS à 60).
			RenderingServer.viewport_set_measure_render_time(get_viewport().get_viewport_rid(), true)
		if _bench_at > 0.0:
			_fast_forward(_bench_at)
			if _bench_failed:
				return  # `_bench_fail` a déjà conclu (timeout pendant l'avance rapide)
			if _bench_timed_out():
				_bench_fail("timeout advancing to --bench-at=%.0f (%.0f s elapsed of %.0f s wall budget)" % [_bench_at, _bench_wall_elapsed_s(), _bench_timeout_s])
				return
		_bench_start_elapsed = float(battle.call("get_elapsed"))
		soldiers.start_timing()
		_apply_camera_override()  # banc d'essai rapproché (lot B1)
	_bench_frames += 1
	_bench_time += delta
	_bench_frame_ms.append(delta * 1000.0)
	_bench_measured += 1
	if _bench_measured > 10:
		var viewport_rid := get_viewport().get_viewport_rid()
		var gpu_ms := RenderingServer.viewport_get_measured_render_time_gpu(viewport_rid)
		_bench_gpu_ms += gpu_ms
		_bench_cpu_ms += RenderingServer.viewport_get_measured_render_time_cpu(viewport_rid)
		_bench_process_ms.append(Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0)
		_bench_soldiers_ms.append(_bench_soldiers_last_ms)
		_bench_tick_ms.append(_bench_tick_last_ms)
		_bench_gpu_samples += 1
		# DA6 : sous Metal le temps GPU mesuré vaut 0 : durée de l'image à la place (vsync coupée).
		_bench_ab_step(gpu_ms if gpu_ms > 0.0 else delta * 1000.0)
	if _bench_timed_out():
		_bench_fail("timeout after %d frames of repeat %d/%d (%.0f s wall budget)" % [_bench_frames, _bench_repeat_done + 1, _bench_repeat, _bench_timeout_s])
		return
	if _bench_frames >= BENCH_FRAMES:
		_bench_repeat_done += 1
		if _bench_repeat_done < _bench_repeat:
			_bench_frames = 0
			_bench_time = 0.0
			return
		_bench_finish()


## Résultat JSON systématique (T8) : imprimé sur une seule ligne préfixée `BENCH_JSON `
## (facile à extraire d'une sortie bruyante), avec code de sortie 0. V3 : `gpu_ms`/`cpu_ms`
## (moyenne par image mesurée) et `ab` (médiane GPU par niveau, si `--bench-ab=`).
func _bench_finish() -> void:
	var sorted_ms := _bench_frame_ms.duplicate()
	sorted_ms.sort()
	var soldier_count := 0
	for unit in units:
		soldier_count += int(unit["soldiers"])
	var total_frames := sorted_ms.size()
	var total_s := 0.0
	for ms in sorted_ms:
		total_s += ms / 1000.0
	var ab_result := {}
	var ab_mean := {}  # DA6 : moyenne aussi (la médiane colle aux paliers de la cadence d'affichage)
	for level in _bench_ab_ms:
		var samples: Array = _bench_ab_ms[level]
		samples.sort()
		ab_result[level] = samples[samples.size() / 2] if not samples.is_empty() else 0.0
		var sum := 0.0
		for v in samples:
			sum += float(v)
		ab_mean[level] = sum / maxf(float(samples.size()), 1.0)
	var result := {
		"ok": true,
		"units": units.size(),
		"soldiers": soldier_count,
		"frames": total_frames,
		"repeats": _bench_repeat,
		"bench_at_s": _bench_start_elapsed,
		"fps_avg": total_frames / maxf(total_s, 0.001),
		"frame_ms_median": _percentile(sorted_ms, 0.5),
		"frame_ms_p95": _percentile(sorted_ms, 0.95),
		"engine_fps": Engine.get_frames_per_second(),
		"gpu_ms": _bench_gpu_ms / maxf(_bench_gpu_samples, 1),
		"cpu_ms": _bench_cpu_ms / maxf(_bench_gpu_samples, 1),
		"process_ms_median": _median(_bench_process_ms),
		"soldiers_ms_median": _median(_bench_soldiers_ms),
		"tick_ms_median": _median(_bench_tick_ms),
		# PB3e : les pics d'image (pas de simulation) sont l'objectif ; pire image et p99.
		"frame_ms_p99": _percentile(sorted_ms, 0.99),
		"frame_ms_max": sorted_ms[sorted_ms.size() - 1] if not sorted_ms.is_empty() else 0.0,
		"tick_ms_p99": _percentile(_sorted(_bench_tick_ms), 0.99),
		"tick_ms_max": _sorted(_bench_tick_ms)[-1] if not _bench_tick_ms.is_empty() else 0.0,
		"step_thread": _step_thread_on,
		"proc_step_ms_median": _median(_bench_step_frame_ms),
		"proc_step_ms_p99": _percentile(_sorted(_bench_step_frame_ms), 0.99),
		"proc_plain_ms_median": _median(_bench_plain_frame_ms),
		"soldiers_plain_ms_median": _median(_bench_plain_soldiers_ms),
		"pose_lerp": _pose_lerp_on,
		"proc_plain_ms_p99": _percentile(_sorted(_bench_plain_frame_ms), 0.99),
		"step_stats": battle.call("get_step_stats") if battle.has_method("get_step_stats") else {},
		"quality": RenderQuality.current(),
		# PF1 : géométrie de la dernière image mesurée (compare les préréglages).
		"primitives": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_PRIMITIVES_IN_FRAME),
		"lod_counts": soldiers.call("lod_counts"),  # FG5 : régiments par LOD, soldats en LOD0 fin
		"draw_calls": RenderingServer.get_rendering_info(RenderingServer.RENDERING_INFO_TOTAL_DRAW_CALLS_IN_FRAME),
		"missiles_launched": effects.launched if effects != null else 0,
		"wall_s": _bench_wall_elapsed_s(),
	}
	if not ab_result.is_empty():
		result["ab"] = ab_result
		result["ab_mean"] = ab_mean
	# EP1 : palier d'échelle, champ, soldats présents, figurines, primitives, budget d'animation.
	var on_field := 0
	var figures := 0
	for unit in units:
		if bool(unit.get("present", true)):
			on_field += int(unit["soldiers"])
			figures += int(unit.get("figures", unit["soldiers"]))
	var scale_info: Dictionary = battle.call("get_scale") if battle.has_method("get_scale") else {}
	result["scale"] = str(scale_info.get("key", ""))
	result["field_w"] = float(scale_info.get("width", 0.0))
	result["on_field_soldiers"] = on_field
	result["figures"] = figures
	result["primitives_m"] = Performance.get_monitor(Performance.RENDER_TOTAL_PRIMITIVES_IN_FRAME) / 1.0e6
	result["draw_calls"] = Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
	result["skipped_updates"] = self.soldiers.skipped_updates
	if self.soldiers.pb3e_verify:
		result["pb3e_buffer_mismatches"] = self.soldiers.pb3e_mismatches
	# EP12 : blessés au sol, régiments désarmés, armes au sol (plafonnées).
	result["wounded"] = self.soldiers.wounded_count
	result["disarmed_units"] = self.soldiers.disarmed_units.size()
	result["dropped_arms"] = self.soldiers.dropped_arms.shown_count() if self.soldiers.dropped_arms != null else 0
	if effects != null and effects.volleys != null:
		# BV1 : volées, traits fichés et échelle des figurines.
		result["volley_arrows"] = effects.volleys.launched
		result["arrows_stuck"] = effects.volleys.stuck_count
		result["figure_scale"] = effects.volleys.figure_scale
	if self.soldiers.impostors != null:
		# BV3 : atlas d'imposteurs cuits, régiments dessinés en imposteurs à la fin du banc.
		result["impostor_atlases"] = self.soldiers.impostors.baked_count
		result["impostor_regiments"] = self.soldiers.impostor_regiments()
	print("BENCH_JSON " + JSON.stringify(result))
	print("BattleScene benchmark: %d units, %d soldiers, %.1f FPS average over %d frames (%d repeats)%s" % [units.size(), soldier_count, result["fps_avg"], total_frames, _bench_repeat, self.soldiers.timing_report()])
	print("BattleScene benchmark: measured from %.0f s, %d missiles launched" % [_bench_start_elapsed, effects.launched if effects != null else 0])
	print("BattleScene benchmark: render %.2f ms GPU, %.2f ms CPU per frame (quality %s)" % [result["gpu_ms"], result["cpu_ms"], result["quality"]])
	for level in ab_result:
		print("BattleScene benchmark A/B: %s median %.2f ms GPU" % [level, ab_result[level]])
	get_tree().quit(0)


## Échec du banc (setup impossible, ou budget de temps réel dépassé) : sortie JSON aussi,
## `"ok": false`, code de sortie 1 (T8 : plus jamais de code 0 sans résultat).
func _bench_fail(reason: String) -> void:
	if _bench_failed:
		return
	_bench_failed = true
	var result := {"ok": false, "error": reason, "wall_s": _bench_wall_elapsed_s()}
	push_error("BattleScene benchmark failed: %s" % reason)
	print("BENCH_JSON " + JSON.stringify(result))
	get_tree().quit(1)


func _bench_wall_elapsed_s() -> float:
	if _bench_wall_start_ms < 0:
		return 0.0
	return float(Time.get_ticks_msec() - _bench_wall_start_ms) / 1000.0


func _bench_timed_out() -> bool:
	return _bench_timeout_s > 0.0 and _bench_wall_elapsed_s() > _bench_timeout_s


static func _sorted(values: Array) -> PackedFloat64Array:
	var out := PackedFloat64Array(values)
	out.sort()
	return out


static func _median(values: Array) -> float:
	if values.is_empty():
		return 0.0
	var sorted := values.duplicate()
	sorted.sort()
	return float(sorted[sorted.size() / 2])


static func _percentile(sorted_values: PackedFloat64Array, ratio: float) -> float:
	if sorted_values.is_empty():
		return 0.0
	var idx := int(clampf(ratio * float(sorted_values.size() - 1), 0.0, float(sorted_values.size() - 1)))
	return float(sorted_values[idx])


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
			battle_audio.update_siege(_frame_siege, elapsed)
	if voices != null:
		voices.update(delta)


## Banc A/B (V3) : range le temps GPU de l'image dans le niveau actif, change de niveau toutes les
## 30 images sur `_bench_measured` (les 4 premières après un changement sont ignorées : mesure en
## retard d'une image, ressources réallouées) — indépendant de `_bench_frames` pour continuer à
## cycler correctement à travers plusieurs répétitions (`--bench-repeat=`).
func _bench_ab_step(gpu_ms: float) -> void:
	if _bench_ab.size() < 2:
		return
	var slot := (_bench_measured - 11) / 30
	var phase := (_bench_measured - 11) % 30
	var level := _bench_ab[slot % _bench_ab.size()]
	if phase == 0:
		if level in ["da6", "no-da6"]:
			terrain.set_da6_view(level == "da6")  # DA6 : végétation de bataille A/B
		elif level in ["fa-grass", "no-fa-grass"]:
			terrain.vegetation.set_fa_view(level == "fa-grass")  # FA7 : herbe en vrais brins A/B
		elif level == "off" or level.contains(":"):
			# PB3b : mise à l'échelle 3D (`off`, `metalfx_s:0.75`, `metalfx_t:0.67`, `bilinear:0.75`).
			RenderQuality.upscale_override = level
			RenderQuality.reapply(get_tree())
		else:
			RenderQuality.override_level = level
			RenderQuality.reapply(get_tree())
	elif phase >= 4:
		if not _bench_ab_ms.has(level):
			_bench_ab_ms[level] = []
		(_bench_ab_ms[level] as Array).append(gpu_ms)


## Avance la simulation (pas de 0,1 s) jusqu'à `seconds`, cadavres et effets compris. Abandonne
## (T8 : `_bench_fail`) si le budget de temps réel du banc est dépassé pendant l'avance rapide,
## pour ne jamais bloquer indéfiniment (120 régiments en lib debug : jadis sans résultat après
## 98-220 s, cf. `docs/wip/t2-perf.md`).
func _fast_forward(seconds: float) -> void:
	while float(battle.call("get_elapsed")) < seconds and not battle.call("is_finished"):
		if _benchmark and _bench_timed_out():
			_bench_fail("timeout advancing to --bench-at=%.0f (stopped at %.1f s simulated)" % [seconds, battle.call("get_elapsed")])
			return
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
	# AN1b : une fois la bataille finie, le camp vainqueur acclame (son horloge d'animation
	# continue ; les autres régiments restent figés, cf. BattleSoldiers.victor_side).
	if finished and soldiers.victor_side == "":
		var winner := str((battle.call("get_outcome") as Dictionary).get("winner", ""))
		soldiers.victor_side = winner if winner != "" else "-"
	elif not finished:
		soldiers.victor_side = ""
	var anim_dt := delta * speed if running else (delta if finished and not paused else 0.0)
	var soldiers_start := Time.get_ticks_usec()
	soldiers.update(battle, units, anim_dt, selected)
	_bench_soldiers_last_ms = float(Time.get_ticks_usec() - soldiers_start) / 1000.0
	_update_effects(delta * speed if running else 0.0)
	if siege_view != null:
		# Lu une seule fois par image (gros dictionnaire construit par le cœur) : HUD, sortie
		# et sons du siège reprennent cette lecture.
		_frame_siege = battle.call("get_siege")
		siege_view.update(_frame_siege, units)
	var banner_scale := _banner_scale()
	# Les drapeaux se présentent de trois quarts à la caméra (lisibles sans être des panneaux).
	var cam_yaw := camera_rig.yaw + PI * 0.5 + 0.35
	for unit in units:
		var id := int(unit["id"])
		var banner: Dictionary = _banners[id]
		var node: Node3D = banner["node"]
		var present: bool = unit["present"]
		node.visible = present
		if not present:
			continue
		var pos := Vector3(float(unit["x"]), float(unit["y"]), float(unit["z"]))
		node.position = pos + Vector3(0, 0, 0)
		node.scale = Vector3.ONE * banner_scale
		node.rotation.y = cam_yaw
		if standards != null:
			# BV3 : de près, le drapeau-repère flotte dans le vent, et s'efface devant l'étendard
			# porté quand celui-ci est affiché.
			node.rotation.y = lerp_angle(standards.downwind_yaw(), cam_yaw, smoothstep(1.0, 2.5, banner_scale))
			node.visible = not (banner_scale <= standards.hide_scale() and standards.handles(id))
		var routing := str(unit["state"]) == "routing"
		if routing != bool(banner["routing"]):
			banner["routing"] = routing
			(banner["flag_mat"] as ShaderMaterial).set_shader_parameter("routing", routing)
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


## CB-M1 : contours de formation. Survol = troupe sous la souris sur le terrain (`world_hover`,
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


## B2 : repères 2D au-dessus des troupes, sous les panneaux du HUD (premier enfant de sa racine).
func _build_markers() -> void:
	markers = BattleUnitMarkers.new()
	hud.root.add_child(markers)
	hud.root.move_child(markers, 0)
	markers.setup(side_colors, player_side)
	markers.marker_clicked.connect(_on_card_clicked)
	markers.marker_right_clicked.connect(_on_marker_right_clicked)


## Ancre écran de chaque repère : au-dessus du drapeau 3D du régiment.
## CB3 : en vue tactique, les ennemis non `spotted` sont retirés et les pastilles sont forcées
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
			var top := Vector3(float(unit["x"]), float(unit["y"]) + (BANNER_HEIGHT + 0.6) * banner_scale, float(unit["z"]))
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


## AN1b : durée d'acclamation avant l'écran de fin (aucune sans affichage ni en banc d'essai).
func _victory_hold() -> float:
	if _benchmark or DisplayServer.get_name() == "headless":
		return 0.0
	return VICTORY_HOLD


## B2 / T2 : écran de fin mis en scène (verdict, écus, pertes par régiment, mentions).
func _show_end() -> void:
	finished_shown = true
	var outcome: Dictionary = battle.call("get_outcome")
	if not autoplay and not _benchmark and _prologue_data.is_empty():  # VO1 : conseiller
		Advisor.say_trigger("first_victory" if str(outcome.get("winner", "")) == player_side else "first_defeat")
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
		aftermath["campaign_outcome"] = _resolution.get("outcome", {})  # CV3-4 : classe du résultat
	# A toast shown in the last seconds (garrison sortie) was drawn over the result table (Q3).
	if hud.toast_label != null:
		hud.toast_label.get_parent().visible = false
	result_screen = BattleResultScreen.new()
	hud.root.add_child(result_screen)
	result_screen.return_pressed.connect(_on_return)
	result_screen.show_result(hud.title_label.text, player_side, sides, battle.call("get_units"), outcome, aftermath)
	# EP13 : la bataille est enregistrée ; « Revoir la bataille » la rejoue ici même.
	_save_replay()
	if battle.has_method("start_replay") and not _benchmark and result_screen.replay_button != null:
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
	if _audio_director != null:  # B3 : la carte retrouve sa musique de contexte
		_audio_director.call("refresh_context")
	if not standalone:
		# PO5 : retour vers la carte (pas de changement de scène) sous le voile noir parchemin.
		await SceneFader.cover()
	returned.emit(result)
	if not standalone:
		SceneFader.reveal()
	if standalone:
		SceneFader.go("res://scenes/start_menu.tscn")


# --- Entrées --------------------------------------------------------------------------


## CB0 : entrées déplacées vers `input` (`BattleInput`). Délégations fines gardées ici pour
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
	_hover_dirty = true  # CB-M2 : le curseur dépend de la sélection


func _on_input_camera_focus(point: Vector3) -> void:
	camera_rig.glide_to(point, camera_rig.distance, camera_rig.yaw)  # PO5 : glissement 0,4 s


func _on_input_markers_toggled() -> void:
	if markers != null:
		markers.toggle()


## CB3 : touche Tab.
func _on_input_tactical_view_toggled() -> void:
	if tactical_view != null:
		tactical_view.toggle()


func _on_input_screenshot_requested() -> void:
	_take_screenshot(ProjectSettings.globalize_path("res://").path_join("../docs/img/godot-battle-%d.png" % Time.get_unix_time_from_system()).simplify_path(), false)


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


## CB5 : clic sur une alerte de la colonne = caméra sur le lieu + repère pulsé sur la minicarte.
func _on_alert_pinged(x: float, z: float) -> void:
	camera_rig.look_at_point(Vector3(x, 0, z), camera_rig.distance, camera_rig.yaw)
	hud.minimap.ping(Vector2(x, z))


## Envoie une commande à la simulation ; les refus s'affichent au journal.
func _exit_tree() -> void:
	if cursor != null:
		cursor.reset()  # CB-M2 : rendre la flèche du système hors de la bataille


## CB-M2 : position de la souris notée par `BattleInput` ; le curseur est recalculé au plus une
## fois par image, et seulement si la case de 2 m visée, le régiment survolé ou la sélection
## changent.
func note_mouse(position: Vector2) -> void:
	_hover_mouse = position
	_hover_dirty = true


func _update_hover_cursor() -> void:
	if cursor == null or not _hover_dirty or _hover_mouse.x < 0.0:
		return
	_hover_dirty = false
	if replay_mode:
		cursor.apply("none")
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
	# CB-M3 : Maj tenue et file pleine : l'ordre en file serait refusé.
	var full := Input.is_key_pressed(KEY_SHIFT) and not selected.is_empty() and BattlePathPreview.queue_full(units, selected)
	if full:
		context = "forbidden"
	_show_queue_tip(full)
	cursor.apply(context)


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
		return {"ok": false, "error": "rejeu"}  # EP13 : on regarde, on ne commande pas
	var result: Dictionary = battle.call("issue_command", command)
	UiSounds.play_order_result(result)  # UB1 / U13 : ordre donné ou refusé
	if voices != null:
		voices.on_order(command, result)  # VO1 : réplique du régiment
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
	input.select_same_type_of(unit_id)  # CB0 : sélection rapide, même `type`
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
	var args := OS.get_cmdline_user_args()
	args.append_array(demo_args)
	demo_args = PackedStringArray()
	for arg in args:
		if arg.begins_with("--screenshot="):
			_screenshot_path = arg.trim_prefix("--screenshot=")
			autoplay = true
		elif arg.begins_with("--units="):
			_pad_units = int(arg.trim_prefix("--units="))
		elif arg.begins_with("--scale="):
			_scale_tier = arg.trim_prefix("--scale=")
		elif arg.begins_with("--bench-at="):
			_bench_at = float(arg.trim_prefix("--bench-at="))
		elif arg.begins_with("--bench-repeat="):
			_bench_repeat = maxi(1, int(arg.trim_prefix("--bench-repeat=")))
		elif arg.begins_with("--bench-timeout="):
			_bench_timeout_s = float(arg.trim_prefix("--bench-timeout="))
		elif arg.begins_with("--bench-ab="):
			_bench_ab = arg.trim_prefix("--bench-ab=").split(",", false)
		elif arg.begins_with("--replay="):
			_replay_path = arg.trim_prefix("--replay=")  # EP13
		elif arg == "--benchmark":
			_benchmark = true
			autoplay = true
			# T8 : vsync fausse les i/s (plafond à 60) et rend le banc peu comparable d'une
			# machine à l'autre ; le désactiver systématiquement pendant le banc.
			DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
			Engine.max_fps = 0
		elif arg == "--autoplay":
			autoplay = true
		elif arg == "--deploy-shot":
			_deploy_shot = true
		elif arg == "--open-shot":
			_open_shot = true
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
		elif arg.begins_with("--standard-side="):
			_standard_side = arg.trim_prefix("--standard-side=")
		elif arg.begins_with("--camera="):
			_camera_override = arg.trim_prefix("--camera=")
		elif arg == "--result-shot":
			_result_shot = true
		elif arg.begins_with("--shot-at="):
			_shot_at = float(arg.trim_prefix("--shot-at="))
		elif arg.begins_with("--standard-shot="):
			_standard_shot = arg.trim_prefix("--standard-shot=")
		elif arg.begins_with("--ep12-shot="):
			_ep12_shot = arg.trim_prefix("--ep12-shot=")
		elif arg == "--no-effects":
			_no_effects = true
		elif arg == "--no-bv1":
			_no_bv1 = true
		elif arg == "--no-bv3":
			_no_bv3 = true
		elif arg == "--no-impostors":
			_no_impostors = true
		elif arg == "--no-speech":
			_no_speech = true
		elif arg.begins_with("--speech-shot="):
			_speech_shot = arg.trim_prefix("--speech-shot=")
		elif arg.begins_with("--speech-at="):
			_speech_at = float(arg.trim_prefix("--speech-at="))
		elif arg.begins_with("--unit-size="):
			_unit_size_override = float(arg.trim_prefix("--unit-size="))
		elif arg.begins_with("--blood="):
			var value := arg.trim_prefix("--blood=")
			_blood_override = ["off", "moderate", "full"].find(value) if not value.is_valid_int() else clampi(int(value), 0, 2)
		elif arg == "--closeup":
			_closeup = true
		elif arg.begins_with("--closeup-distance="):
			_closeup_distance = float(arg.trim_prefix("--closeup-distance="))
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
		elif arg == "--cinematic":
			_force_cinematic = true
		elif arg == "--birds-shot":
			_birds_shot = true
		elif arg == "--dust-shot":
			_dust_shot = true
		elif arg == "--cinematic-shot":
			_cinematic_shot = true
			_force_cinematic = true
	_ep8_disabled = BattleStaging.disabled_from_args(args)
	if _screenshot_path != "":
		call_deferred("_stage_screenshot")


## Capture : IA des deux camps jusqu'au premier contact (+ 12 s), sélection de deux régiments
## du joueur, caméra sur la mêlée, capture après quelques images.
func _stage_screenshot() -> void:
	if battle == null:
		get_tree().quit(1)
		return
	camera_rig.edge_pan_enabled = false
	if _open_shot:
		for _i in 90:
			await get_tree().process_frame
		_take_screenshot(_screenshot_path, true)
		return
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
	# EP7 : sur une carte historique, les batailles françaises montent longtemps avant le choc.
	for _i in (9000 if not historical.is_empty() else 3000):
		battle.call("tick", 0.1)
		# Les soldats tombés pendant l'avance rapide laissent aussi leurs cadavres.
		units = battle.call("get_units")
		soldiers.update(battle, units, 0.1, [])
		_update_effects(0.1)
		if staging != null:
			staging.update(units, 0.1, 0.1, bool(battle.call("is_finished")))
		if _dust_shot:
			# EP8 : une charge de cavalerie lancée depuis 2,5 s (la poussière s'est levée).
			var charging := -1
			for unit in units:
				if bool(unit["present"]) and str(unit["state"]) == "charging" and str(unit["render"]) == "cavalry" and _dry_around(float(unit["x"]), float(unit["z"]), 40.0):
					charging = int(unit["id"])
					break
			if charging < 0:
				_dust_since = -1.0  # au sec seulement (au gué, ce sont des gerbes d'eau)
			elif _dust_since < 0.0:
				_dust_since = float(battle.call("get_elapsed"))
				_dust_unit = charging
			if _dust_since >= 0.0 and float(battle.call("get_elapsed")) - _dust_since >= 2.5:
				break
			continue
		if (_birds_shot or _cinematic_shot) and contact_time >= 0.0:
			# EP8 : envol des oiseaux (6 s après le choc) ou plan cinématique (dès le choc).
			if float(battle.call("get_elapsed")) - contact_time >= (6.0 if _birds_shot else 0.5):
				break
		if _ep12_shot != "":
			if _ep12_shot_ready(units):
				break
			continue
		if _standard_shot == "fallen" or _standard_shot == "captured":
			# EP5 : dès qu'un étendard gît depuis 2 s (le porte-étendard a fini de tomber).
			if standards != null:
				standards.update(units, soldiers, _camera_position())
			var down := false
			for unit in units:
				if _standard_shot == "fallen" and str(unit.get("standard", "")) == "fallen" and float(unit.get("standard_timer", 99.0)) < 3.0:
					down = true
				elif _standard_shot == "captured" and int(unit.get("standard_by", -1)) >= 0:
					down = true
			if down:
				break
			continue
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
	if effects != null and blood != null:
		print("BattleScene: BV1 %d volley arrows, %d stuck, %d blood decals (level %d), last at %s" % [effects.volleys.launched, effects.volleys.stuck_count, blood.decal_count, blood.level, blood.last_pos])
	units = battle.call("get_units")
	if grass_flatten != null:
		print("BattleScene: BV3 %d corpses marked on the grass, last at %s" % [grass_flatten.corpse_marks, grass_flatten.last_corpse])
		for unit in units:
			if str(unit["state"]) == "melee":
				print("BattleScene: BV3 melee at (%.0f, %.0f), grass flattened %.2f, blood %.2f" % [float(unit["x"]), float(unit["z"]), grass_flatten.flatten_at(float(unit["x"]), float(unit["z"])), grass_flatten.blood_at(float(unit["x"]), float(unit["z"]))])
				break
	var focus := Vector3.ZERO
	var n := 0
	for unit in units:
		if str(unit["state"]) == "melee":
			focus += Vector3(float(unit["x"]), 0, float(unit["z"]))
			n += 1
	if _closeup:
		var shot := _closeup_shot(units)
		camera_rig.look_at_point(shot["focus"], _closeup_distance, float(shot["yaw"]))
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
	_apply_standard_shot()
	_apply_ep12_shot()
	_apply_staging_shot()
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


## Capture EP5 (`--standard-shot=`) : cadre un porte-étendard (à pied, à cheval), la ligne de
## bataille et ses étendards au loin, ou un étendard tombé.
func _apply_standard_shot() -> void:
	if _standard_shot == "" or standards == null:
		return
	selected.clear()
	var best: Dictionary = {}
	for unit in units:
		if not bool(unit["present"]) or not unit.has("bearer_slots"):
			continue
		var render := str(unit["render"])
		var ok := false
		match _standard_shot:
			"foot":
				ok = render == "infantry" and str(unit.get("standard", "")) == "carried"
			"mounted":
				ok = render == "cavalry" and str(unit.get("standard", "")) == "carried"
			"fallen":
				ok = str(unit.get("standard", "")) in ["fallen", "lost"]
			"line":
				ok = str(unit["side"]) == player_side
			"captured":
				for other in units:
					if int(other.get("standard_by", -1)) == int(unit["id"]):
						ok = true
		if ok and (best.is_empty() or _standard_shot_score(unit) > _standard_shot_score(best)):
			best = unit
	if best.is_empty():
		push_warning("BattleScene: no regiment for --standard-shot=%s" % _standard_shot)
		return
	var facing := float(best["facing"])
	var ahead := Vector3(sin(facing), 0, cos(facing))
	if _standard_shot == "line":
		# Derrière la ligne du joueur, haut et loin : les étendards ennemis à 300-600 m.
		camera_rig.look_at_point(Vector3(float(best["x"]), 0, float(best["z"])) + ahead * 25.0, 45.0, atan2(-ahead.x, -ahead.z) + 0.35)
		print("BattleScene: EP5 line shot, %d standards shown, %d figures" % [standards.shown_count, standards.figure_count])
		return
	var point := Vector3(float(best.get("standard_x", best["x"])), 0, float(best.get("standard_z", best["z"])))
	if _standard_shot != "fallen":
		var frame: Variant = soldiers.figure_at(int(best["id"]), int((best["bearer_slots"] as PackedInt32Array)[0]))
		if frame != null:
			point = (frame as Transform3D).origin
	# De trois quarts, devant le porte-étendard.
	var yaw := atan2(ahead.x, ahead.z) + 0.6
	camera_rig.look_at_point(point, 8.0 if _standard_shot == "foot" else 12.0, yaw)
	if _standard_shot == "fallen" and camera_rig.camera != null:
		# Vue plongeante : l'étendard gît dans la mêlée, caché par les hommes debout.
		camera_rig.set_process(false)
		var ground := terrain.height_at(point.x, point.z)
		var at := Vector3(point.x, ground, point.z)
		camera_rig.camera.global_position = at + Vector3(sin(yaw), 0, cos(yaw)) * 6.0 + Vector3(0, 7.5, 0)
		camera_rig.camera.look_at(at, Vector3.UP)
	print("BattleScene: EP5 %s shot on %s (%s) at %s, standard %s, %d standards, %d figures, %d fallen" % [_standard_shot, str(best["name"]), str(best["type"]), point, str(best.get("standard", "")), standards.shown_count, standards.figure_count, standards.fallen_count])


## Capture EP12 (`--ep12-shot=`) : vrai quand l'instant est venu. Blessés : 25 pas (2,5 s)
## après le 30e blessé, le dernier tombé cadré (il rampe, s'assoit ou s'agenouille). Déroute :
## un régiment à pied débandé depuis 2 s (armes au sol, course de fuite).
func _ep12_shot_ready(p_units: Array) -> bool:
	if _ep12_ticks >= 0:
		_ep12_ticks -= 1
		return _ep12_ticks < 0
	if _ep12_shot == "wounded":
		if soldiers.wounded_count >= 30 and soldiers.last_wounded_pos != null:
			_ep12_focus = soldiers.last_wounded_pos
			_ep12_ticks = 25
	elif _ep12_shot == "rout":
		for unit in p_units:
			var id := int(unit["id"])
			if bool(unit["present"]) and str(unit["render"]) != "cavalry" and soldiers.disarmed_units.has(id) and soldiers.anim_time - float(soldiers.disarmed_units[id]) >= 2.0:
				_ep12_focus = id
				return true
	return false


func _apply_ep12_shot() -> void:
	if _ep12_shot == "" or _ep12_focus == null:
		if _ep12_shot != "":
			push_warning("BattleScene: nothing to frame for --ep12-shot=%s" % _ep12_shot)
		return
	selected.clear()
	var dropped: int = soldiers.dropped_arms.shown_count() if soldiers.dropped_arms != null else 0
	if _ep12_shot == "wounded":
		var at: Vector3 = _ep12_focus
		camera_rig.look_at_point(at, 7.0, 0.7)
		if camera_rig.camera != null:
			# Vue plongeante : le blessé est au sol, souvent derrière les hommes debout.
			camera_rig.set_process(false)
			var ground := Vector3(at.x, terrain.height_at(at.x, at.z), at.z)
			camera_rig.camera.global_position = ground + Vector3(sin(0.7), 0, cos(0.7)) * 5.5 + Vector3(0, 5.0, 0)
			camera_rig.camera.look_at(ground, Vector3.UP)
		print("BattleScene: EP12 wounded shot at %s, %d wounded, %d arms on the ground" % [at, soldiers.wounded_count, dropped])
		return
	for unit in units:
		if int(unit["id"]) != int(_ep12_focus):
			continue
		# Devant les fuyards (ils courent vers la caméra), de trois quarts.
		var facing := float(unit["facing"])
		var at := Vector3(float(unit["x"]), 0, float(unit["z"]))
		var yaw := atan2(sin(facing), cos(facing)) + 0.5
		camera_rig.look_at_point(at, 16.0, yaw)
		if camera_rig.camera != null:
			camera_rig.set_process(false)
			var ground := Vector3(at.x, terrain.height_at(at.x, at.z), at.z)
			camera_rig.camera.global_position = ground + Vector3(sin(yaw), 0, cos(yaw)) * 11.0 + Vector3(0, 4.5, 0)
			camera_rig.camera.look_at(ground + Vector3(0, 0.8, 0), Vector3.UP)
		print("BattleScene: EP12 rout shot on %s (%s), %d arms on the ground, %d wounded" % [str(unit["name"]), str(unit["type"]), dropped, soldiers.wounded_count])


## Préférence de `--standard-shot` : le camp voulu d'abord, puis le général et sa retenue noble
## (DA1b : leurs étendards portent les armes de la maison du général).
func _standard_shot_score(unit: Dictionary) -> int:
	var wanted := _standard_side if _standard_side != "" else player_side
	var score := 4 if str(unit["side"]) == wanted else 0
	if bool(unit.get("is_general", false)):
		score += 2
	elif BattleStandards.is_house_retinue(unit):
		score += 1
	return score


## Capture : `--camera=x,z,distance,lacet_en_degrés` place la caméra (réglage du rendu).
func _apply_camera_override() -> void:
	# FG5 : `--benchmark --closeup` : banc rapproché (caméra de la capture `--closeup`, 26 m de la
	# mêlée) où les figurines passent par leurs LOD0 et LOD1.
	if _benchmark and _closeup and _camera_override == "":
		# Régiment du joueur le plus proche de l'ennemi (pas de mêlée garantie à `--bench-at`).
		var best := INF
		var focus := Vector3.ZERO
		var yaw := 0.0
		for unit in units:
			if str(unit["side"]) != player_side or not bool(unit["present"]):
				continue
			for other in units:
				if str(other["side"]) == player_side or not bool(other["present"]):
					continue
				var a := Vector2(float(unit["x"]), float(unit["z"]))
				var b := Vector2(float(other["x"]), float(other["z"]))
				if a.distance_to(b) < best:
					best = a.distance_to(b)
					focus = Vector3(a.x, 0.0, a.y)
					yaw = atan2(a.x - b.x, a.y - b.y) + 1.05
		camera_rig.look_at_point(focus, 26.0, yaw)
		return
	if _camera_override == "":
		return
	var parts := _camera_override.split(",")
	if parts.size() < 4:
		return
	var cam_z := float(parts[1])
	if parts[1] == "river":
		cam_z = terrain.river_center_z(float(parts[0]))  # VN : cadrage sur la rivière (captures)
	camera_rig.look_at_point(Vector3(float(parts[0]), 0, cam_z), float(parts[2]), deg_to_rad(float(parts[3])))


func _take_screenshot(path: String, quit_after: bool) -> void:
	if OS.get_cmdline_user_args().has("--no-hud"):
		# EP2 : captures de décor sans interface.
		for layer in find_children("*", "CanvasLayer", true, false):
			(layer as CanvasLayer).visible = false
		for control in find_children("*", "Control", true, false):
			if not (control.get_parent() is Control):
				(control as Control).visible = false
		# CR1 : l'interface 3D aussi (contours, trajets, arcs de tir), sinon elle fuit dans les
		# captures « sans interface ».
		for overlay: Node3D in [outlines, path_preview, range_arc]:
			if overlay != null:
				overlay.visible = false
		await get_tree().process_frame
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
	if replay_mode:
		return  # EP13 : le déploiement enregistré est rejoué par le cœur
	if autoplay and not _deploy_shot:
		return
	if not _prologue_data.is_empty():
		return  # NT4 : le prologue commence en bataille, armées déjà rangées
	if not historical.is_empty() and not bool(historical.get("site_only", false)):
		return  # EP7 : déploiement historique imposé

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
