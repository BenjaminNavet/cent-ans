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


func _ready() -> void:
	process_mode = Node.PROCESS_MODE_ALWAYS
	apply_display_enabled = DisplayServer.get_name() != "headless"
	load_settings()
	apply_display()


## Smoke test : valeurs par défaut, fichier dédié (le fichier du joueur n'est pas touché).
func use_test_file(test_path: String = TEST_SETTINGS_PATH) -> void:
	path = test_path
	values = DEFAULTS.duplicate()
	apply_display_enabled = false


func get_value(key: String) -> Variant:
	return values.get(key, DEFAULTS.get(key))


func set_value(key: String, value: Variant, persist: bool = true) -> void:
	if not DEFAULTS.has(key):
		push_warning("Settings: unknown key %s" % key)
		return
	values[key] = _coerce(key, value)
	if key.begins_with("video/") or key == "interface/ui_scale":
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
