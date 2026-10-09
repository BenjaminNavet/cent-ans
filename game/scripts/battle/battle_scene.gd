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
## `--units=<n>` (complète chaque camp à n régiments, essai sans retour campagne), `--autoplay` (IA des deux camps),
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
var speech: BattleSpeech = null  # BV3 : discours du général avant la bataille
var enemy_speech: BattleSpeech = null  # NT6a : discours du général adverse, après celui du joueur
var _unit_size_override: float = -1.0  # `--unit-size=<k>` (banc d'essai BV1)
var _blood_override: int = -1  # `--blood=<0|1|2>`
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
var _pad_units: int = 0
var _scale_tier: String = ""  # EP1 : palier d'échelle forcé (`--scale=`), sinon selon l'effectif
var _closeup: bool = false
var _closeup_distance: float = 26.0  # FG5 : `--closeup-distance=<m>` (captures du LOD0 par soldat)
var _shot_at: float = -1.0  # B4 : `--shot-at=<s>`
var _standard_side: String = ""  # DA1b : `--standard-side=` (camp cadré par `--standard-shot`)
var _standard_shot: String = ""  # EP5 : `--standard-shot=<foot|mounted|line|fallen|captured>`
var _weather_override: String = ""
var _camera_override: String = ""
var deployment: DeploymentController = null  # F5c : phase de déploiement du joueur
var _deploy_shot: bool = false
var _sortie_shown: bool = false
var music: BattleMusicDirector = null  # B3 : musique dynamique par intensité
var battle_audio: BattleAudio = null  # AU1 : sons spatialisés (mêlée, volées, siège, météo)
var voices: BattleVoices = null  # VO1 : répliques des régiments
var _siege_audio_timer: float = 0.0
var _audio_director: Node = null  # B3 : mis en veille pendant la bataille, réveillé au retour
var staging: BattleStaging = null  # EP8 : heure, nuages, fumées, oiseaux, plan cinématique
var _hour_override: String = ""  # EP8 : `--hour=`
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
var _step_thread_on: bool = false
## RJ-b : figurines et régiments interpolés entre deux pas de simulation (démarche continue) ;
## coupé en headless (tests).
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
	_setup_decor_hover()
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
		elif not _prologue_data.is_empty():
			_start_prologue()
		return
	if campaign_sim == null and _historical != "":
		# EP7 : carte historique jouée hors campagne (menu « Batailles historiques »).
		standalone = true
		if not begin_historical():
			push_error("BattleScene: historical battle %s failed" % _historical)
		return
	if campaign_sim == null:
		standalone = true
		if not _stage_standalone():
			push_error("BattleScene: cannot stage a demo battle")
			# T8 : un banc d'essai qui ne peut pas se lancer doit échouer bruyamment (JSON +
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
	BattleTerrain.apply_site_overrides(setup)  # B5 : --terrain= --season= --coast
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
	if _scale_tier != "":
		battle.call("set_scale_tier", _scale_tier)  # EP1 : --scale=<skirmish|large|epic>
	if not battle.call("setup", setup, battle_seed):
		return false
	if _hour_override != "":
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


## NT2 : bataille personnalisée `config` (armées achetées, champ, siège) ; le résultat n'est pas
## conservé (hors campagne).
func begin_custom(config: Dictionary) -> bool:
	if not ClassDB.class_exists("BattleSim"):
		return false
	_audio_director = get_node_or_null("/root/AudioDirector")
	if _audio_director != null:
		_audio_director.call("stop_all")
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
	for node in [soldiers, standards, effects, engines_fx, assault_fx]:
		if node != null and is_instance_valid(node):
			(node as Node).get_parent().remove_child(node)
			(node as Node).queue_free()
	standards = null
	effects = null
	blood = null
	engines_fx = null
	assault_fx = null
	grass_flatten = null
	_build_soldier_layers(kept_impostors)


## EP13 : « Revoir la bataille » depuis l'écran de fin (résultat déjà appliqué à la campagne).
func start_replay_in_place() -> bool:
	if battle == null:
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
	if replay_mode or _screenshot_path != "" or battle == null:
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
	historical = battle.call("get_historical")
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
		landmark_town = LandmarkSiegeTown.create(battle.call("get_siege_landmark"), func(x: float, z: float) -> float: return terrain.height_at(x, z))
		if landmark_town != null:
			add_child(landmark_town)
	# PO4 : heure de rendu (phase du cœur, sinon graine) ; EP8 anime la lumière si l'heure avance.
	var tod: Dictionary = battle.call("get_time_of_day")
	var time_key := BattleAtmosphere.time_key_for(tod, battle_seed, weather_key)
	BattleAtmosphere.apply(world_env, sun, weather_key, camera_rig.camera, terrain.season_key, time_key, not tod.is_empty())
	if _sim_weather == "rain" and weather_key != "rain":
		BattleAtmosphere.add_shower(camera_rig.camera)  # EP7 : averse qui cessera
	if terrain.horizon != null:
		terrain.horizon.apply_atmosphere(world_env.environment, sun, weather_key)  # EP2
	var field_center := terrain.field_center()  # EP1 : (600, 400) au palier standard
	BattleAtmosphere.add_ground_mist(self, weather_key, Vector3(field_center.x, terrain.height_at(field_center.x, field_center.y), field_center.y), Vector2(terrain.FIELD_W + 300.0, terrain.FIELD_D + 300.0))
	# BV1 (ADR 0016) : taille des unités = figurines par homme simulé (rendu seulement).
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
	hud.set_opening(battle.call("get_opening"), player_side)  # CV3-2
	hud.player_faction = str((setup[player_side] as Dictionary).get("faction", ""))
	hud.set_leader((setup[player_side] as Dictionary).get("general", null), hud.player_faction)
	# CB4 : textes des capacités pour les infobulles des boutons de carte.
	hud.ability_catalog = battle.call("get_ability_catalog")
	# RJ-a : formations de régiment (menu du bouton « Formation », infobulles).
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
	staging.setup(self, battle, world_env.environment, sun, _weather_key, terrain_data)
	staging.configure_effects(effects, str(terrain_data.get("terrain", "plains")), terrain.season_key)
	if decor_view != null:
		decor_view.attach_smoke(self, staging, _weather_key)
	if effects != null:
		effects.cannon_fired.connect(staging.on_cannon_fired)
	if staging.cinematic != null and (autoplay or _screenshot_path != ""):
		# Jamais en banc d'essai, en capture ni quand l'IA joue les deux camps.
		staging.cinematic.enabled = false
	_update_time_label()






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
	if autoplay or not _prologue_data.is_empty():  # NT4 : le guide parle déjà
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
	var houses := {}  # DA1 : maison du général par camp
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
	# EP3 : l'eau à la largeur locale de la rivière, et les ruisseaux.
	effects.setup(_weather_key, func(x: float, z: float) -> float: return terrain.world_height(x, z), func(x: float, z: float) -> int: return 1 if terrain.in_water(x, z) else 0)
	effects.configure_ground(str(terrain.terrain.get("ground", "dry")), _weather_key)
	effects.volleys.figure_scale = float(battle.call("get_figure_scale"))
	effects.volleys.sound_event.connect(_on_sound_event)
	effects.sound_event.connect(_on_sound_event)
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


## BV3 : discours du général du joueur, au début du déploiement (ou de la bataille), en jeu
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


## NT6a : discours du général adverse, joué juste après celui du joueur ; un « passer » du joueur écarte aussi celui de l'adversaire.
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


## BV3 : herbe couchée par les troupes et sous les corps, sang lisible en prairie ; pavois du
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
		var no_quarter: bool = battle != null and bool(battle.call("get_no_quarter", side))
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
	# B3 : cadrage sur le centre de l'armée du joueur (léger décalage vers l'ennemi), à ~66 m de
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


# --- Boucle ---------------------------------------------------------------------------


func _process(delta: float) -> void:
	if battle == null:
		return
	# EP8 : ralenti du plan cinématique (temps de bataille et animations).
	var slow := staging.time_scale() if staging != null else 1.0
	var running: bool = not paused and not battle.call("is_finished")
	if running:
		_configure_step_thread()
		battle.call("tick", delta * speed * slow)
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


## PB3e (ADR 0090) : le pas de simulation suivant se calcule sur un fil pendant que l'image
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
	# AN1b : une fois la bataille finie, le camp vainqueur acclame (son horloge d'animation
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


## AN1b : durée d'acclamation avant l'écran de fin (aucune sans affichage).
func _victory_hold() -> float:
	if DisplayServer.get_name() == "headless":
		return 0.0
	return VICTORY_HOLD


## B2 / T2 : écran de fin mis en scène (verdict, écus, pertes par régiment, mentions).
func _show_end() -> void:
	finished_shown = true
	var outcome: Dictionary = battle.call("get_outcome")
	if not autoplay and _prologue_data.is_empty():  # VO1 : conseiller
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
	var args := CmdArgs.args().duplicate()
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
		elif arg.begins_with("--replay="):
			_replay_path = arg.trim_prefix("--replay=")  # EP13
		elif arg == "--autoplay":
			autoplay = true
		elif arg == "--deploy-shot":
			_deploy_shot = true
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
	if CmdArgs.has("--no-hud"):
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
