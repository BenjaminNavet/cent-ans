extends Node

## F3 — autoload `Settings` : réglages du joueur persistés dans `user://settings.cfg`
## (même fichier que les volumes d'`AudioDirector`, section `audio`, conservée telle quelle).
##
## Clés « section/nom » ; `get_value` / `set_value` (persisté, signal `changed`). Les réglages
## d'affichage sont appliqués ici (fenêtre, vsync, échelle d'interface, taille du texte) ; ceux de la carte
## (caméra, sauvegarde auto, batailles, rapport de saison) sont lus par `FlowController`.
## Volumes : délégués à `AudioDirector` (`music_volume` / `sfx_volume`).
##
## Accès depuis un script à `class_name` : `get_node_or_null("/root/Settings")` (le smoke test
## compile certains scripts avant l'enregistrement des autoloads).

signal changed(key: String)

const SETTINGS_PATH := "user://settings.cfg"
const TEST_SETTINGS_PATH := "user://settings_smoke.cfg"

const DEFAULTS := {
	"video/fullscreen": false,
	"video/resolution": Vector2i(1440, 900),
	"video/vsync": true,
	# V3 (A1-14) : préréglage de qualité du rendu (low, medium, high, ultra), voir `RenderQuality`.
	# RL1 : "auto" (défaut) suit le GPU détecté ; un choix enregistré par le joueur est gardé.
	"video/quality": "auto",
	# PB3b (ADR 0080) : mise à l'échelle 3D : "auto" suit le préréglage de qualité, "off",
	# "quality", "performance" (voir `RenderQuality.UPSCALE_PLAYER`).
	"video/upscale": "auto",
	# Lot U4 (audit A3) : échelle automatique (hauteur de la fenêtre / 900, bornée entre 0,9 et
	# 1,6) multipliée par « Taille de l'interface » ; « Taille du texte » agit sur les polices seules.
	"interface/ui_size": 1.0,
	"interface/text_size": 1.0,
	"interface/season_report": true,
	"interface/confirm_end_turn": false,
	# Lot U5 (audit A3, T2) : portée des lettres et du bandeau (`NewsInterest.MODES`).
	"interface/news_filter": "interest",
	# Lot U7 : disposition du clavier (« azerty » / « qwerty ») pour les libellés des touches.
	"input/layout": "auto",
	# Lot NT6d : touches réaffectées (action → liste de touches, voir `KeyBindings`).
	"input/bindings": {},
	# Lot U12 : accessibilité.
	"access/colorblind": false,
	"access/reduce_motion": false,
	"access/high_contrast": false,
	"camera/edge_pan": true,
	"camera/speed": 1.0,
	# C1 : brouillard de guerre (provinces hors de vue voilées, armées ennemies masquées).
	"map/fog_of_war": true,
	# CT1 : mouvements des armées IA en fin de tour (« follow » Suivre, « show » Montrer, « hide »
	# Masquer, voir `AiTurnReplay`) et leur vitesse (×1, ×2, ×4).
	"map/ai_moves": "follow",
	"map/ai_moves_speed": 1.0,
	"game/autosave_interval": 4,
	"game/interactive_battles": true,
	# F8 : tutoriel des premiers tours (désactivable, progression persistée).
	"tutorial/enabled": true,
	"tutorial/step": 0,
	"tutorial/done": false,
	# UX2 : guide rangé par « Plus tard » (repris à `tutorial/step`) ; conseil « que faire
	# maintenant » de la carte (encart haut gauche).
	"tutorial/postponed": false,
	# FE6 : guide de la féodalité (trois étapes) vu une fois.
	"feudal_tutorial/done": false,
	"interface/next_hint": true,
	# BV1/BV2 : sang en bataille (0 désactivé, 1 modéré, 2 complet : démembrements) ; taille des unités (figurines
	# par homme simulé, ADR 0016 : 0,5 petite, 1 normale, 1,5 grande, 2,5 ultra ; EP1 : 4 épique).
	"battle/blood": 1,
	"battle/unit_size": 1.0,
	# FB1 : plafond de figurines dessinées sur tout le champ de bataille ; la taille des unités est
	# abaissée pour le respecter (rendu seulement, voir ADR 0016).
	"battle/max_figures": 15000,
	# EP8 : plan cinématique facultatif au premier choc, et son ralenti.
	"battle/cinematic": true,
	"battle/cinematic_slowmo": true,
	# NT2 : dernière composition de la bataille personnalisée (JSON, "" : aucune).
	"custom_battle/last": "",
	# NT4 : didacticiel de bataille terminé, ou « Ne plus demander » à l'invite du premier lancement.
	"battle_prologue/done": false,
	"battle_prologue/never_ask": false,
	# MM1 : prologue (cartons 1328-1337) joué une fois au premier lancement.
	"interface/intro_seen": false,
	# VO1 : conseiller parlé (chroniqueur), répliques des unités, interventions déjà faites
	# (« premières fois », liste séparée par des virgules).
	"voice/advisor": true,
	"voice/barks": true,
	"voice/advisor_seen": "",
	# PO (ADR 0097) : mode développeur, textes d'outil visibles (relief incomplet, commandes).
	# Aussi vrai avec l'argument `--dev` (voir `is_dev`), sans être enregistré.
	"dev/mode": false,
}

## Choix proposés par le menu (texte d'interface, pas des données de jeu).
const RESOLUTIONS: Array[Vector2i] = [
	Vector2i(1280, 720), Vector2i(1440, 900), Vector2i(1600, 900), Vector2i(1920, 1080),
	Vector2i(2560, 1440),
]
const UI_SIZES: Array[float] = [0.8, 0.9, 1.0, 1.1, 1.25]
const TEXT_SIZES: Array[float] = [0.9, 1.0, 1.15, 1.3]
## Échelle automatique : hauteur de la fenêtre / 900, bornée (lot U4).
const AUTO_SCALE_REFERENCE_HEIGHT := 900.0
const AUTO_SCALE_MIN := 0.9
const AUTO_SCALE_MAX := 1.6
## Bornes de l'échelle finale (automatique × taille choisie).
const UI_SCALE_MIN := 0.7
const UI_SCALE_MAX := 2.0
## Propriétés de thème de taille de police mises à l'échelle par « Taille du texte ».
const FONT_SIZE_KEYS := ["font_size", "normal_font_size", "bold_font_size", "italics_font_size", "bold_italics_font_size", "mono_font_size"]
const PARCHMENT_THEME := "res://scenes/ui/parchment_theme.tres"
const AUTOSAVE_CHOICES: Array[int] = [0, 1, 2, 4, 8]
const BLOOD_CHOICES: Array[int] = [0, 1, 2]
const UNIT_SIZES: Array[float] = [0.5, 1.0, 1.5, 2.5, 4.0]  # EP1 : 4 = Épique
# EP1 : 20 000 à 30 000 pour les batailles rangées (mesuré ≥ 30 i/s à 28 600 figurines, ADR 0076).
const MAX_FIGURES_CHOICES: Array[int] = [1000, 2000, 3000, 4000, 6000, 8000, 10000, 12000, 15000, 20000, 25000, 30000]

var path: String = SETTINGS_PATH
var values: Dictionary = {}
## Désactivé en headless et en mode test : pas de changement de fenêtre.
var apply_display_enabled: bool = true
## `--resolution`, `--fullscreen` ou `--windowed` passé au moteur : la fenêtre de la ligne de
## commande l'emporte sur le réglage enregistré, jusqu'à ce que le joueur change la vidéo en jeu.
var window_overridden_by_cmdline: bool = false


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	apply_display_enabled = DisplayServer.get_name() != "headless"
	if apply_display_enabled:
		window_overridden_by_cmdline = window_differs_from_project(
			DisplayServer.window_get_size(), DisplayServer.window_get_mode()
		)
	load_settings()
	KeyBindings.apply_saved(get_value(KeyBindings.SETTING_KEY))
	apply_display()
	get_tree().root.size_changed.connect(_apply_ui_scale)
	get_tree().node_added.connect(_on_node_added)
	RenderQuality.apply_global(get_tree().root)


## PO (ADR 0097, bible DA § 12.5) : vrai en mode développeur (réglage `dev/mode` ou `-- --dev`).
## Hors mode dev, aucun texte d'outil (commande, chemin, identifiant brut) n'est affiché.
func is_dev() -> bool:
	return bool(get_value("dev/mode")) or OS.get_cmdline_user_args().has("--dev")


## Smoke test : valeurs par défaut, fichier dédié (le fichier du joueur n'est pas touché).
func use_test_file(test_path: String = TEST_SETTINGS_PATH) -> void:
	path = test_path
	values = DEFAULTS.duplicate()
	apply_display_enabled = false


## Vrai si la fenêtre de départ diffère de celle du projet : le moteur a reçu `--resolution`,
## `--fullscreen` ou `--maximized` (arguments qu'il consomme et que `OS.get_cmdline_args()` ne
## rend pas). La fenêtre est alors celle de la ligne de commande.
static func window_differs_from_project(window_size: Vector2i, window_mode: int) -> bool:
	if window_mode != DisplayServer.WINDOW_MODE_WINDOWED:
		return true
	var project_size := Vector2i(
		int(ProjectSettings.get_setting("display/window/size/viewport_width", 1440)),
		int(ProjectSettings.get_setting("display/window/size/viewport_height", 900))
	)
	var override_size := Vector2i(
		int(ProjectSettings.get_setting("display/window/size/window_width_override", 0)),
		int(ProjectSettings.get_setting("display/window/size/window_height_override", 0))
	)
	if override_size.x > 0 and override_size.y > 0:
		project_size = override_size
	return window_size != project_size


func get_value(key: String) -> Variant:
	return values.get(key, DEFAULTS.get(key))


func set_value(key: String, value: Variant, persist: bool = true) -> void:
	if not DEFAULTS.has(key):
		push_warning("Settings: unknown key %s" % key)
		return
	values[key] = _coerce(key, value)
	if key == "video/resolution" or key == "video/fullscreen":
		window_overridden_by_cmdline = false
	if key == "video/quality" or key == "video/upscale":
		if is_inside_tree():
			RenderQuality.reapply(get_tree())
	elif key.begins_with("video/") or key == "interface/ui_size" or key == "interface/text_size":
		apply_display()
	elif key == "access/high_contrast":
		Accessibility.apply_contrast(load(PARCHMENT_THEME) as Theme, bool(get_value(key)))
	if persist:
		save_settings()
	changed.emit(key)


func _coerce(key: String, value: Variant) -> Variant:
	var default: Variant = DEFAULTS[key]
	match typeof(default):
		TYPE_BOOL:
			return bool(value)
		TYPE_INT:
			return int(value)
		TYPE_FLOAT:
			return float(value)
		TYPE_VECTOR2I:
			return value if value is Vector2i else default
		TYPE_STRING:
			return str(value)
	return value


func load_settings(from_path: String = "") -> void:
	var source := from_path if from_path != "" else path
	values = DEFAULTS.duplicate()
	var config := ConfigFile.new()
	if config.load(source) != OK:
		return
	for key in DEFAULTS:
		var parts: PackedStringArray = (key as String).split("/")
		if config.has_section_key(parts[0], parts[1]):
			values[key] = _coerce(key, config.get_value(parts[0], parts[1]))


## Écrit les sections de réglages en conservant les autres (volumes `audio`).
func save_settings(to_path: String = "") -> Error:
	var target := to_path if to_path != "" else path
	var config := ConfigFile.new()
	config.load(target)
	for key in DEFAULTS:
		var parts: PackedStringArray = (key as String).split("/")
		config.set_value(parts[0], parts[1], get_value(key))
	return config.save(target)


func reset_to_defaults() -> void:
	values = DEFAULTS.duplicate()
	KeyBindings.reset()
	apply_display()
	RenderQuality.reapply(get_tree())
	save_settings()
	for key in DEFAULTS:
		changed.emit(key)


# --- Affichage -------------------------------------------------------------------


func apply_display() -> void:
	if not is_inside_tree():
		return
	_apply_ui_scale()
	apply_text_size()
	Accessibility.apply_contrast(load(PARCHMENT_THEME) as Theme, bool(get_value("access/high_contrast")))
	if not apply_display_enabled:
		return
	DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_ENABLED if get_value("video/vsync") else DisplayServer.VSYNC_DISABLED)
	if window_overridden_by_cmdline:
		return
	if get_value("video/fullscreen"):
		DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_FULLSCREEN)
	else:
		if DisplayServer.window_get_mode() == DisplayServer.WINDOW_MODE_FULLSCREEN:
			DisplayServer.window_set_mode(DisplayServer.WINDOW_MODE_WINDOWED)
		var size: Vector2i = get_value("video/resolution")
		var screen := DisplayServer.window_get_current_screen()
		var usable := DisplayServer.screen_get_usable_rect(screen)
		size = Vector2i(mini(size.x, usable.size.x), mini(size.y, usable.size.y))
		if DisplayServer.window_get_size() != size:
			DisplayServer.window_set_size(size)
			DisplayServer.window_set_position(usable.position + (usable.size - size) / 2)


# --- Échelle de l'interface et taille du texte (lot U4) ------------------------------


## Échelle automatique pour une fenêtre de `height` pixels : 1 à 900 px, 0,9 au moins, 1,6 au plus.
static func auto_ui_scale(height: float) -> float:
	if height < 300.0:
		return 1.0  # fenêtre factice (headless 64 × 64) : pas d'échelle automatique
	return clampf(height / AUTO_SCALE_REFERENCE_HEIGHT, AUTO_SCALE_MIN, AUTO_SCALE_MAX)


## Échelle appliquée à la fenêtre : automatique × « Taille de l'interface ».
func effective_ui_scale() -> float:
	var height := float(get_tree().root.size.y) if is_inside_tree() else AUTO_SCALE_REFERENCE_HEIGHT
	return clampf(auto_ui_scale(height) * float(get_value("interface/ui_size")), UI_SCALE_MIN, UI_SCALE_MAX)


func _apply_ui_scale() -> void:
	if not is_inside_tree():
		return
	var scale := effective_ui_scale()
	if not is_equal_approx(get_tree().root.content_scale_factor, scale):
		get_tree().root.content_scale_factor = scale


## « Taille du texte » : thème parchemin, thème par défaut et tailles de police posées en code
## (surcharges `font_size`…) de chaque contrôle, recalculées depuis leur valeur d'origine.
func apply_text_size() -> void:
	if not is_inside_tree():
		return
	var factor := float(get_value("interface/text_size"))
	_scale_theme(load(PARCHMENT_THEME) as Theme, factor)
	_scale_theme(ThemeDB.get_default_theme(), factor)
	for node in get_tree().root.find_children("*", "Control", true, false):
		_scale_control_text(node as Control, factor)


func _scale_theme(theme: Theme, factor: float) -> void:
	if theme == null:
		return
	if not theme.has_meta("text_base_size"):
		theme.set_meta("text_base_size", theme.default_font_size if theme.default_font_size > 0 else ThemeDB.fallback_font_size)
	var size := roundi(int(theme.get_meta("text_base_size")) * factor)
	if theme.default_font_size != size:
		theme.default_font_size = size


func _on_node_added(node: Node) -> void:
	if node is Control and not is_equal_approx(float(get_value("interface/text_size")), 1.0):
		# Les surcharges sont souvent posées juste après l'ajout (dans `_ready`).
		_scale_control_text.call_deferred(node, float(get_value("interface/text_size")))


## Met à l'échelle les surcharges de taille de police de `control` ; la taille d'origine est
## gardée en méta (`text_base_sizes` : clé → [base, appliquée]) et reprise si le code l'a changée.
func _scale_control_text(control: Control, factor: float) -> void:
	if not is_instance_valid(control):
		return
	var bases: Dictionary = control.get_meta("text_base_sizes", {})
	var changed_any := false
	for key: String in FONT_SIZE_KEYS:
		if not control.has_theme_font_size_override(key):
			continue
		var current := control.get_theme_font_size(key)
		var entry: Array = bases.get(key, [])
		if entry.is_empty() or int(entry[1]) != current:
			entry = [current, current]  # nouvelle valeur posée par le code : c'est la base
		var target := maxi(6, roundi(int(entry[0]) * factor))
		if target != current:
			control.add_theme_font_size_override(key, target)
		entry[1] = target
		bases[key] = entry
		changed_any = true
	if changed_any:
		control.set_meta("text_base_sizes", bases)


# --- Volumes (AudioDirector) --------------------------------------------------------


func _audio() -> Node:
	return get_node_or_null("/root/AudioDirector")


func music_volume() -> float:
	var audio := _audio()
	return float(audio.get("music_volume")) if audio != null else 0.0


func sfx_volume() -> float:
	var audio := _audio()
	return float(audio.get("sfx_volume")) if audio != null else 0.0


func set_music_volume(linear: float) -> void:
	var audio := _audio()
	if audio != null:
		audio.call("set_music_volume", linear, path == SETTINGS_PATH)


func set_sfx_volume(linear: float) -> void:
	var audio := _audio()
	if audio != null:
		audio.call("set_sfx_volume", linear, path == SETTINGS_PATH)


## AU1 : volume d'un bus réglable (`AudioBuses.PLAYER_BUSES`).
func bus_volume(bus_name: String) -> float:
	var audio := _audio()
	return float(audio.call("bus_volume", bus_name)) if audio != null else 0.0


func set_bus_volume(bus_name: String, linear: float) -> void:
	var audio := _audio()
	if audio != null:
		audio.call("set_bus_volume", bus_name, linear, path == SETTINGS_PATH)
