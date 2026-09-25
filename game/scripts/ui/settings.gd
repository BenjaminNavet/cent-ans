extends Node

## F3 — autoload `Settings` : réglages du joueur persistés dans `user://settings.cfg`
## (même fichier que les volumes d'`AudioDirector`, section `audio`, conservée telle quelle).
##
## Clés « section/nom » ; `get_value` / `set_value` (persisté, signal `changed`). Les réglages
## d'affichage sont appliqués ici (fenêtre, vsync, échelle d'interface) ; ceux de la carte
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
	"video/quality": "high",
	"interface/ui_scale": 1.0,
	"interface/season_report": true,
	"interface/confirm_end_turn": false,
	"camera/edge_pan": true,
	"camera/speed": 1.0,
	# C1 : brouillard de guerre (provinces hors de vue voilées, armées ennemies masquées).
	"map/fog_of_war": true,
	"game/autosave_interval": 4,
	"game/interactive_battles": true,
	# F8 : tutoriel des premiers tours (désactivable, progression persistée).
	"tutorial/enabled": true,
	"tutorial/step": 0,
	"tutorial/done": false,
	# MM1 : prologue (cartons 1328-1337) joué une fois au premier lancement.
	"interface/intro_seen": false,
}

## Choix proposés par le menu (texte d'interface, pas des données de jeu).
const RESOLUTIONS: Array[Vector2i] = [
	Vector2i(1280, 720), Vector2i(1440, 900), Vector2i(1600, 900), Vector2i(1920, 1080),
	Vector2i(2560, 1440),
]
const UI_SCALES: Array[float] = [0.8, 0.9, 1.0, 1.1, 1.25, 1.5]
const AUTOSAVE_CHOICES: Array[int] = [0, 1, 2, 4, 8]

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
	apply_display()
	RenderQuality.apply_global(get_tree().root)


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
	if key == "video/quality":
		if is_inside_tree():
			RenderQuality.reapply(get_tree())
	elif key.begins_with("video/") or key == "interface/ui_scale":
		apply_display()
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
	apply_display()
	RenderQuality.reapply(get_tree())
	save_settings()
	for key in DEFAULTS:
		changed.emit(key)


# --- Affichage -------------------------------------------------------------------


func apply_display() -> void:
	if not is_inside_tree():
		return
	get_tree().root.content_scale_factor = float(get_value("interface/ui_scale"))
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
