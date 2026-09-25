class_name StartMenu
extends Control

## Écran titre et menu principal (F3, refait par MM1) : décor 3D vivant (`MenuBackdrop3D` : Paris
## au crépuscule, l'ost et ses bannières, plans de caméra lents enchaînés en fondu), titre enluminé
## (`IlluminatedTitle`), colonne de boutons à gauche (Nouvelle partie, Continuer, Charger une
## partie, Prologue, Codex, Réglages, Crédits, Quitter), légende du plan en bas à droite.
## « Nouvelle partie » ouvre le choix de faction (`FactionSelect`) en fondu ; « Commencer »
## passe par l'écran de chargement (`LoadingScreen`). Le prologue (`IntroCards`) est joué une fois
## au premier lancement, puis depuis le menu. Sans rendu (headless) ou avec `--no-menu-3d` : fond
## illustré 2D (`MenuBackground`).
##
## Options (après `--`) : `--screenshot=<png>` capture puis quitte ; `--menu-stage=settings`,
## `credits`, `faction`, `intro` ou `loading` ouvre l'écran correspondant avant la capture ;
## `--autostart[=fac_x]` démarre directement une campagne (jeu exporté).

const CAMPAIGN_SCENE := "res://scenes/campaign_map.tscn"
const FADE_SECONDS := 0.6
const SWITCH_SECONDS := 0.45

@onready var save_load_dialog: SaveLoadDialog = %SaveLoadDialog
@onready var fade: ColorRect = %Fade

var backdrop: Node = null  # MenuBackdrop3D ou MenuBackground
var faction_select: FactionSelect
var main_column: Control
var continue_button: Button
var new_game_button: Button
var load_button: Button
var intro_button: Button
var codex_button: Button
var settings_button: Button
var credits_button: Button
var quit_button: Button
var status_label: Label
var latest_save: Dictionary = {}
## Bouton « Commencer » du choix de faction (les tests et captures y accèdent ici).
var start_button: Button:
	get:
		return faction_select.start_button if faction_select != null else null
var selected_faction: String:
	get:
		return faction_select.selected_faction if faction_select != null else "fac_france"

var _continue_detail: Label
var _caption: Label
var _leaving := false
var _overlay: Control = null  # réglages, crédits ou prologue ouverts


func _ready() -> void:
	_build_backdrop()
	_build_shading()
	_build_main_column()
	_build_caption()
	if backdrop is MenuBackdrop3D:
		var scene := backdrop as MenuBackdrop3D
		if scene.shot_count() > 0:
			_on_shot_changed(scene.shot_index, str((scene.config.get("shots", [])[scene.shot_index] as Dictionary).get("label", "")))
	faction_select = FactionSelect.new()
	faction_select.name = "FactionSelect"
	faction_select.visible = false
	add_child(faction_select)
	faction_select.back_requested.connect(show_main)
	faction_select.start_requested.connect(_on_start_requested)
	var facade := get_node_or_null("/root/SimFacade")
	if facade != null:
		var pending := str(facade.get("pending_faction"))
		faction_select.select(pending if pending != "" else "fac_france")
	move_child(save_load_dialog, -1)
	move_child(fade, -1)
	save_load_dialog.load_confirmed.connect(_on_load)
	save_load_dialog.dialog_closed.connect(_refresh_saves)
	_refresh_saves()
	var audio := get_node_or_null("/root/AudioDirector")
	if audio != null:
		audio.enter_menu()
	_play_intro_fade()
	# RL1 : `-- --journey` (parcours de vérification du jeu exporté, `scripts/dev/release_journey.gd`).
	if ReleaseJourney.maybe_start(get_tree()):
		return
	var args := OS.get_cmdline_user_args()
	# `-- --autostart[=fac_x]` : démarre directement une campagne (tests du jeu exporté, où la
	# scène ne peut pas être passée en argument) ; les autres options vont à la carte.
	for arg in args:
		# NV1 : `-- --naval-scenario=sluys` lance directement une bataille navale historique.
		if arg.begins_with("--naval-scenario="):
			get_tree().change_scene_to_file.call_deferred("res://scenes/naval/naval_battle.tscn")
			return
		if arg.begins_with("--autostart"):
			var faction := arg.trim_prefix("--autostart").trim_prefix("=")
			if faction != "":
				faction_select.select(faction)
			_autostart.call_deferred()
			return
	var staged := false
	for arg in args:
		if arg == "--menu-stage=settings":
			open_settings()
			staged = true
		elif arg == "--menu-stage=credits":
			open_credits()
			staged = true
		elif arg == "--menu-stage=faction":
			show_faction_select(true)
			staged = true
		elif arg == "--menu-stage=intro":
			open_intro()
			staged = true
		elif arg == "--menu-stage=loading":
			_on_start_requested.call_deferred(faction_select.selected_faction, 1337, "")
			return
	for arg in args:
		if arg.begins_with("--screenshot="):
			_screenshot_then_quit(arg.trim_prefix("--screenshot="))
			staged = true
	# Premier lancement (sans option de ligne de commande) : le prologue.
	var settings := get_node_or_null("/root/Settings")
	if not staged and args.is_empty() and not _headless() and settings != null and not bool(settings.call("get_value", "interface/intro_seen")):
		settings.call("set_value", "interface/intro_seen", true)
		open_intro()
	else:
		new_game_button.grab_focus.call_deferred()


static func _headless() -> bool:
	return DisplayServer.get_name() == "headless"


# --- Construction ------------------------------------------------------------------------------


func _build_backdrop() -> void:
	var use_3d := not _headless() and not OS.get_cmdline_user_args().has("--no-menu-3d")
	if use_3d:
		var scene := MenuBackdrop3D.new()
		scene.name = "Backdrop3D"
		add_child(scene)
		move_child(scene, 0)
		scene.shot_changed.connect(_on_shot_changed)
		backdrop = scene
	else:
		var flat := MenuBackground.new()
		flat.name = "Background"
		flat.dim = 0.25
		add_child(flat)
		move_child(flat, 0)
		backdrop = flat


## Dégradé sombre à gauche (lisibilité de la colonne), vignette en bas.
func _build_shading() -> void:
	var left := TextureRect.new()
	var gradient := Gradient.new()
	gradient.set_color(0, Color(0.02, 0.015, 0.01, 0.82))
	gradient.set_color(1, Color(0.02, 0.015, 0.01, 0.0))
	gradient.add_point(0.55, Color(0.02, 0.015, 0.01, 0.45))
	var texture := GradientTexture2D.new()
	texture.gradient = gradient
	texture.fill_from = Vector2(0, 0)
	texture.fill_to = Vector2(1, 0)
	texture.width = 256
	texture.height = 4
	left.texture = texture
	left.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	left.stretch_mode = TextureRect.STRETCH_SCALE
	left.mouse_filter = Control.MOUSE_FILTER_IGNORE
	left.anchor_right = 0.55
	left.anchor_bottom = 1.0
	add_child(left)
	var bottom := TextureRect.new()
	var bottom_gradient := Gradient.new()
	bottom_gradient.set_color(0, Color(0, 0, 0, 0.0))
	bottom_gradient.set_color(1, Color(0.02, 0.015, 0.01, 0.6))
	var bottom_texture := GradientTexture2D.new()
	bottom_texture.gradient = bottom_gradient
	bottom_texture.fill_from = Vector2(0, 0)
	bottom_texture.fill_to = Vector2(0, 1)
	bottom_texture.width = 4
	bottom_texture.height = 128
	bottom.texture = bottom_texture
	bottom.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	bottom.stretch_mode = TextureRect.STRETCH_SCALE
	bottom.mouse_filter = Control.MOUSE_FILTER_IGNORE
	bottom.anchor_top = 0.78
	bottom.anchor_right = 1.0
	bottom.anchor_bottom = 1.0
	add_child(bottom)


func _build_main_column() -> void:
	main_column = MarginContainer.new()
	main_column.name = "MainColumn"
	main_column.anchor_bottom = 1.0
	main_column.offset_right = 640
	main_column.add_theme_constant_override("margin_left", 84)
	main_column.add_theme_constant_override("margin_top", 64)
	main_column.add_theme_constant_override("margin_bottom", 40)
	add_child(main_column)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 6)
	main_column.add_child(column)
	column.add_child(IlluminatedTitle.new())
	var gap := Control.new()
	gap.custom_minimum_size = Vector2(0, 44)
	column.add_child(gap)

	new_game_button = _menu_button(column, "Nouvelle partie", func() -> void: show_faction_select())
	continue_button = _menu_button(column, "Continuer", _on_continue)
	_continue_detail = FrontEndStyle.label("", 16, Color(0.85, 0.78, 0.62), FrontEndStyle.body_italic(), 4)
	var detail_margin := MarginContainer.new()
	detail_margin.add_theme_constant_override("margin_left", 25)
	detail_margin.add_theme_constant_override("margin_top", -8)
	detail_margin.add_child(_continue_detail)
	column.add_child(detail_margin)
	load_button = _menu_button(column, "Charger une partie", func() -> void: save_load_dialog.open_load())
	intro_button = _menu_button(column, "Prologue : 1328-1337", open_intro)
	codex_button = _menu_button(column, "Codex", open_codex)
	settings_button = _menu_button(column, "Réglages", open_settings)
	credits_button = _menu_button(column, "Crédits", open_credits)
	quit_button = _menu_button(column, "Quitter", func() -> void: get_tree().quit())

	status_label = FrontEndStyle.label("", 15, Color(1.0, 0.75, 0.6), FrontEndStyle.body_italic(), 4)
	column.add_child(status_label)


func _menu_button(parent: Control, text: String, action: Callable) -> Button:
	var button := Button.new()
	button.text = text
	FrontEndStyle.style_menu_button(button)
	button.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	button.custom_minimum_size = Vector2(360, 0)
	button.pressed.connect(action)
	parent.add_child(button)
	return button


func _build_caption() -> void:
	_caption = FrontEndStyle.label("", 18, Color(0.92, 0.86, 0.72), FrontEndStyle.title_italic(), 5)
	_caption.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	_caption.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_RIGHT)
	_caption.offset_left = -620
	_caption.offset_top = -52
	_caption.offset_right = -36
	_caption.offset_bottom = -22
	_caption.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_caption)


func _on_shot_changed(_index: int, label: String) -> void:
	if _caption == null:
		return
	var tween := create_tween()
	tween.tween_property(_caption, "modulate:a", 0.0, 0.3)
	tween.tween_callback(func() -> void: _caption.text = "%s — printemps 1337" % label if label != "" else "")
	tween.tween_property(_caption, "modulate:a", 1.0, 1.2)


func _play_intro_fade() -> void:
	if Accessibility.reduce_motion():  # U12 (UI3) : pas de fondu
		fade.color.a = 0.0
		main_column.modulate.a = 1.0
		return
	fade.color.a = 1.0
	main_column.modulate.a = 0.0
	var tween := create_tween()
	tween.tween_property(fade, "color:a", 0.0, FADE_SECONDS * 1.5)
	tween.tween_property(main_column, "modulate:a", 1.0, FADE_SECONDS)


func _refresh_saves() -> void:
	latest_save = SaveSlots.latest()
	continue_button.visible = not latest_save.is_empty()
	_continue_detail.get_parent().visible = not latest_save.is_empty()
	if not latest_save.is_empty():
		var facade := get_node_or_null("/root/SimFacade")
		var faction := str(facade.call("faction_short_name", str(latest_save.get("faction", "")))) if facade != null else ""
		_continue_detail.text = "%s — %s" % [faction, latest_save.get("date", "")]
		continue_button.tooltip_text = str(latest_save.get("label", ""))
	# Audit A3 M3 : plus de ligne de débogage ; un message seulement si le jeu est incomplet.
	var facade_node := get_node_or_null("/root/SimFacade")
	var loaded: bool = facade_node != null and bool(facade_node.call("store_loaded"))
	status_label.text = "" if loaded else "Données de jeu introuvables : le jeu est incomplet, réinstallez-le."
	status_label.visible = status_label.text != ""


## Capture de l'écran (`-- --screenshot=<chemin.png>`) puis sortie.
func _screenshot_then_quit(path: String) -> void:
	for _i in 90:
		await get_tree().process_frame
	# Audit A3 M1 : la capture ne doit pas tomber au milieu d'un fondu.
	while fade.color.a > 0.01 or (main_column.visible and main_column.modulate.a < 0.99) or (faction_select.visible and faction_select.modulate.a < 0.99):
		await get_tree().process_frame
	for _i in 30:
		await get_tree().process_frame
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var err := image.save_png(path)
	print("StartMenu: screenshot %s (%s)" % [path, error_string(err)])
	get_tree().quit(0 if err == OK else 1)


func card_count() -> int:
	return faction_select.card_count() if faction_select != null else 0


# --- Écrans ------------------------------------------------------------------------------------


func show_faction_select(immediate: bool = false) -> void:
	_switch(main_column, faction_select, immediate)
	_caption.visible = false
	faction_select.start_button.grab_focus.call_deferred()


func show_main() -> void:
	_switch(faction_select, main_column, false)
	_caption.visible = true
	new_game_button.grab_focus.call_deferred()


func _switch(from: Control, to: Control, immediate: bool) -> void:
	if immediate:
		from.visible = false
		to.visible = true
		to.modulate.a = 1.0
		return
	var tween := create_tween()
	tween.tween_property(from, "modulate:a", 0.0, SWITCH_SECONDS * 0.6)
	tween.tween_callback(func() -> void:
		from.visible = false
		to.modulate.a = 0.0
		to.visible = true)
	tween.tween_property(to, "modulate:a", 1.0, SWITCH_SECONDS)


func open_settings() -> void:
	_open_overlay(SettingsMenu.new())


func open_credits() -> void:
	_open_overlay((load("res://scenes/ui/credits_screen.tscn") as PackedScene).instantiate())


func open_intro() -> void:
	var intro := IntroCards.new()
	intro.finished.connect(func() -> void:
		_overlay = null
		new_game_button.grab_focus.call_deferred())
	_open_overlay(intro, false)


func open_codex() -> void:
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles != null:
		bubbles.call("open_entry", "")


func _open_overlay(overlay: Control, closable: bool = true) -> void:
	if _overlay != null and is_instance_valid(_overlay):
		_overlay.queue_free()
	_overlay = overlay
	if closable:
		overlay.connect("closed", func() -> void: _overlay = null)
	add_child(overlay)
	move_child(fade, -1)


func overlay_open() -> bool:
	return _overlay != null and is_instance_valid(_overlay)


# --- Départ ------------------------------------------------------------------------------------


func _on_start_requested(faction_id: String, seed_value: int, _start_date: String) -> void:
	var facade := get_node_or_null("/root/SimFacade")
	if facade != null:
		facade.set("pending_faction", faction_id)
		facade.set("pending_seed", seed_value)
		facade.set("pending_load_path", "")
	_leave()


## Démarrage direct (jeu exporté, tests) : pas de fondu ni d'écran de chargement, pour que les
## captures `--screenshot` de la carte gardent leur délai habituel.
func _autostart() -> void:
	var facade := get_node_or_null("/root/SimFacade")
	if facade != null:
		facade.set("pending_faction", faction_select.selected_faction)
		facade.set("pending_seed", faction_select.seed_value())
		facade.set("pending_load_path", "")
	get_tree().change_scene_to_file(CAMPAIGN_SCENE)


func _on_continue() -> void:
	if latest_save.is_empty():
		return
	_on_load(str(latest_save["path"]))


func _on_load(path: String) -> void:
	var facade := get_node_or_null("/root/SimFacade")
	if facade != null:
		facade.set("pending_load_path", path)
	_leave()


## Fondu au noir puis écran de chargement (qui remplace ce menu par la carte).
func _leave() -> void:
	if _leaving:
		return
	_leaving = true
	var tween := create_tween()
	tween.tween_property(fade, "color:a", 1.0, FADE_SECONDS * 0.8)
	var audio := get_node_or_null("/root/AudioDirector")
	if audio != null and audio.has_method("duck_music"):
		audio.call("duck_music", -8.0, FADE_SECONDS)
	await tween.finished
	LoadingScreen.start(get_tree(), CAMPAIGN_SCENE)
