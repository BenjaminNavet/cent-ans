class_name StartMenu
extends Control

## Écran de démarrage (F3) : carte ancienne illustrée en fond (`MenuBackground`, cartouche
## « Cent Ans »), trois cartes de faction (écu, nom, blason, accroche, objectifs historiques),
## graine, « Continuer » (sauvegarde la plus récente), « Commencer », « Charger une partie »,
## « Réglages », « Crédits », « Quitter ». Fondu à l'ouverture, fondu vers l'écran de
## chargement (`LoadingScreen`) qui construit la carte.
##
## Options (après `--`) : `--screenshot=<png>` capture puis quitte ; `--menu-stage=settings`
## ou `credits` ouvre la fenêtre correspondante avant la capture ; `--autostart[=fac_x]`
## démarre directement une campagne (jeu exporté).

const CAMPAIGN_SCENE := "res://scenes/campaign_map.tscn"
const FADE_SECONDS := 0.6

## Textes d'interface (pas des données de jeu) : deux lignes d'accroche par faction jouable.
const PLAYABLE_FACTIONS := [
	{
		"id": "fac_france",
		"fallback_name": "Royaume de France",
		"description": "Le plus riche royaume d'Occident, mais un roi contesté et des vassaux turbulents. Défendez la Guyenne, tenez la Flandre et repoussez l'Anglais.",
	},
	{
		"id": "fac_england",
		"fallback_name": "Royaume d'Angleterre",
		"description": "Un royaume insulaire aux finances tendues, mais aux archers redoutés. Revendiquez la couronne de France depuis la Guyenne et la Flandre.",
	},
	{
		"id": "fac_burgundy",
		"fallback_name": "Duché de Bourgogne",
		"description": "Un duché prospère, vassal du roi de France et son beau-frère. Grandissez dans l'ombre des deux couronnes sans vous faire dévorer.",
	},
]

@onready var background: MenuBackground = %Background
@onready var title_label: Label = %Title
@onready var subtitle_label: Label = %Subtitle
@onready var cards: HBoxContainer = %Cards
@onready var seed_edit: LineEdit = %SeedEdit
@onready var continue_button: Button = %ContinueButton
@onready var start_button: Button = %StartButton
@onready var load_button: Button = %LoadButton
@onready var settings_button: Button = %SettingsButton
@onready var credits_button: Button = %CreditsButton
@onready var quit_button: Button = %QuitButton
@onready var status_label: Label = %StatusLabel
@onready var save_load_dialog: SaveLoadDialog = %SaveLoadDialog
@onready var fade: ColorRect = %Fade

var selected_faction: String = "fac_france"
var latest_save: Dictionary = {}
var _card_buttons: Dictionary = {}  # faction_id → Button (toggle)
var _card_panels: Dictionary = {}  # faction_id → PanelContainer
var _leaving := false
var _overlay: Control = null  # réglages ou crédits ouverts


func _ready() -> void:
	_build_cards()
	# Le cartouche de l'illustration porte le titre : le libellé ne sert que sans image.
	title_label.visible = not background.has_art()
	# Bandeau parchemin sous les textes posés sur l'illustration (les noms de villes de la carte
	# ne doivent pas se lire à travers).
	for label: Label in [subtitle_label, status_label]:
		_give_backing(label)
	background.layout_changed.connect(_layout)
	_layout.call_deferred()
	start_button.pressed.connect(_on_start)
	continue_button.pressed.connect(_on_continue)
	load_button.pressed.connect(func() -> void: save_load_dialog.open_load())
	settings_button.pressed.connect(open_settings)
	credits_button.pressed.connect(open_credits)
	quit_button.pressed.connect(func() -> void: get_tree().quit())
	save_load_dialog.load_confirmed.connect(_on_load)
	save_load_dialog.dialog_closed.connect(_refresh_saves)
	seed_edit.text = str(SimFacade.pending_seed)
	_select_faction(SimFacade.pending_faction if _card_buttons.has(SimFacade.pending_faction) else "fac_france")
	_refresh_saves()
	var audio := get_node_or_null("/root/AudioDirector")
	if audio != null:
		audio.enter_menu()
	_play_intro()
	var args := OS.get_cmdline_user_args()
	# `-- --autostart[=fac_x]` : démarre directement une campagne (tests du jeu exporté, où la
	# scène ne peut pas être passée en argument) ; les autres options vont à la carte.
	for arg in args:
		if arg.begins_with("--autostart"):
			var faction := arg.trim_prefix("--autostart").trim_prefix("=")
			if faction != "" and _card_buttons.has(faction):
				_select_faction(faction)
			_autostart.call_deferred()
			return
	for arg in args:
		if arg == "--menu-stage=settings":
			open_settings()
		elif arg == "--menu-stage=credits":
			open_credits()
		elif arg == "--menu-stage=loading":
			_on_start.call_deferred()
			return
	for arg in args:
		if arg.begins_with("--screenshot="):
			_screenshot_then_quit(arg.trim_prefix("--screenshot="))


## Le contenu est centré sous le cartouche de l'illustration (s'il y a la place).
func _layout() -> void:
	var center: Control = $Center
	var cartouche := background.cartouche_rect()
	var content_height: float = ($Center/VBox as Control).get_combined_minimum_size().y
	var top := 0.0
	if cartouche.size != Vector2.ZERO and size.y - cartouche.end.y >= content_height + 16.0:
		top = cartouche.end.y
	center.offset_top = top


func _play_intro() -> void:
	fade.color.a = 1.0
	cards.modulate.a = 0.0
	var tween := create_tween()
	tween.tween_property(fade, "color:a", 0.0, FADE_SECONDS)
	tween.tween_property(cards, "modulate:a", 1.0, FADE_SECONDS * 0.8)


func _refresh_saves() -> void:
	latest_save = SaveSlots.latest()
	continue_button.visible = not latest_save.is_empty()
	if not latest_save.is_empty():
		continue_button.tooltip_text = "%s — %s, %s" % [latest_save.get("label", ""), SimFacade.faction_short_name(str(latest_save.get("faction", ""))), latest_save.get("date", "")]
	# Audit A3 M3 : plus de ligne de débogage ; un message seulement si le jeu est incomplet.
	status_label.text = "" if SimFacade.store_loaded() else "Données de jeu introuvables : le jeu est incomplet, réinstallez-le."
	status_label.visible = status_label.text != ""


## Capture de l'écran de démarrage (`-- --screenshot=<chemin.png>`) puis sortie.
func _screenshot_then_quit(path: String) -> void:
	for _i in 60:
		await get_tree().process_frame
	# Audit A3 M1 : la capture tombait au milieu du fondu d'ouverture (cartes à demi transparentes).
	while fade.color.a > 0.01 or cards.modulate.a < 0.99:
		await get_tree().process_frame
	await RenderingServer.frame_post_draw
	var image := get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(path.get_base_dir())
	var err := image.save_png(path)
	print("StartMenu: screenshot %s (%s)" % [path, error_string(err)])
	get_tree().quit(0 if err == OK else 1)


func _build_cards() -> void:
	for child in cards.get_children():
		child.queue_free()
	_card_buttons.clear()
	_card_panels.clear()
	var group := ButtonGroup.new()
	for entry in PLAYABLE_FACTIONS:
		var faction_id: String = entry["id"]
		var info := SimFacade.faction_info(faction_id)
		var panel := PanelContainer.new()
		panel.custom_minimum_size = Vector2(320, 330)
		var vbox := VBoxContainer.new()
		vbox.add_theme_constant_override("separation", 6)
		panel.add_child(vbox)

		var swatch := ColorRect.new()
		swatch.custom_minimum_size = Vector2(0, 8)
		swatch.color = info.get("color", Color(0.5, 0.5, 0.5))
		vbox.add_child(swatch)

		var header := HBoxContainer.new()
		header.add_theme_constant_override("separation", 10)
		vbox.add_child(header)
		# Écu procédural de la faction (M10 assets), à gauche du nom.
		var shield_texture := PortraitLoader.heraldry_texture(faction_id)
		if shield_texture != null:
			var shield := TextureRect.new()
			shield.texture = shield_texture
			shield.custom_minimum_size = Vector2(80, 92)
			shield.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
			shield.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
			header.add_child(shield)
		var names := VBoxContainer.new()
		names.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		names.alignment = BoxContainer.ALIGNMENT_CENTER
		header.add_child(names)
		var name_label := Label.new()
		name_label.text = str(info.get("name", entry["fallback_name"])) if SimFacade.store_loaded() else entry["fallback_name"]
		name_label.add_theme_font_size_override("font_size", 22)
		name_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		names.add_child(name_label)
		var blazon := Label.new()
		var blazon_text := str(info.get("blazon", ""))
		blazon.text = blazon_text if blazon_text != "" else "Blason inconnu"
		blazon.add_theme_font_size_override("font_size", 13)
		blazon.add_theme_color_override("font_color", Color(0.40, 0.28, 0.14))
		blazon.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		names.add_child(blazon)

		var description := Label.new()
		description.text = entry["description"]
		description.add_theme_font_size_override("font_size", 15)
		description.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		vbox.add_child(description)

		# M10 : objectifs historiques de la faction (résumé des données).
		var summary := str(info.get("victory_summary", ""))
		if summary != "":
			var objectives := Label.new()
			objectives.text = "Objectifs (avant %d) : %s" % [int(info.get("victory_end_year", 0)), summary]
			objectives.add_theme_font_size_override("font_size", 14)
			objectives.add_theme_color_override("font_color", Color(0.45, 0.12, 0.08))
			objectives.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
			objectives.size_flags_vertical = Control.SIZE_EXPAND_FILL
			vbox.add_child(objectives)
		else:
			description.size_flags_vertical = Control.SIZE_EXPAND_FILL

		var choose := Button.new()
		choose.text = "Choisir"
		choose.toggle_mode = true
		choose.button_group = group
		choose.pressed.connect(func() -> void: _select_faction(faction_id))
		vbox.add_child(choose)
		panel.gui_input.connect(func(event: InputEvent) -> void:
			if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
				_select_faction(faction_id)
				if event.double_click:
					_on_start())

		cards.add_child(panel)
		_card_buttons[faction_id] = choose
		_card_panels[faction_id] = panel


func _select_faction(faction_id: String) -> void:
	selected_faction = faction_id
	for id in _card_buttons:
		_card_buttons[id].button_pressed = id == faction_id
		_card_buttons[id].text = "Choisie" if id == faction_id else "Choisir"
		_card_panels[id].modulate = Color(1, 1, 1, 1) if id == faction_id else Color(0.84, 0.82, 0.78, 1)
		_card_panels[id].scale = Vector2.ONE
	start_button.text = "Commencer — %s" % SimFacade.faction_short_name(faction_id)


func card_count() -> int:
	return _card_panels.size()


# --- Fenêtres ---------------------------------------------------------------------


func open_settings() -> void:
	_open_overlay(SettingsMenu.new())


func open_credits() -> void:
	_open_overlay((load("res://scenes/ui/credits_screen.tscn") as PackedScene).instantiate())


func _open_overlay(overlay: Control) -> void:
	if _overlay != null and is_instance_valid(_overlay):
		_overlay.queue_free()
	_overlay = overlay
	overlay.connect("closed", func() -> void: _overlay = null)
	add_child(overlay)


func overlay_open() -> bool:
	return _overlay != null and is_instance_valid(_overlay)


# --- Départ -----------------------------------------------------------------------


func _on_start() -> void:
	SimFacade.pending_faction = selected_faction
	SimFacade.pending_seed = int(seed_edit.text) if seed_edit.text.is_valid_int() else seed_edit.text.hash()
	SimFacade.pending_load_path = ""
	_leave()


## Démarrage direct (jeu exporté, tests) : pas de fondu ni d'écran de chargement, pour que les
## captures `--screenshot` de la carte gardent leur délai habituel.
func _autostart() -> void:
	SimFacade.pending_faction = selected_faction
	SimFacade.pending_seed = int(seed_edit.text) if seed_edit.text.is_valid_int() else seed_edit.text.hash()
	SimFacade.pending_load_path = ""
	get_tree().change_scene_to_file(CAMPAIGN_SCENE)


func _on_continue() -> void:
	if latest_save.is_empty():
		return
	_on_load(str(latest_save["path"]))


func _on_load(path: String) -> void:
	SimFacade.pending_load_path = path
	_leave()


## Fondu au noir puis écran de chargement (qui remplace ce menu par la carte).
func _leave() -> void:
	if _leaving:
		return
	_leaving = true
	var tween := create_tween()
	tween.tween_property(fade, "color:a", 1.0, FADE_SECONDS * 0.6)
	await tween.finished
	LoadingScreen.start(get_tree(), CAMPAIGN_SCENE)


func _give_backing(label: Label) -> void:
	var box := StyleBoxFlat.new()
	box.bg_color = Color(0.93, 0.87, 0.74, 0.88)
	box.set_corner_radius_all(4)
	box.content_margin_left = 14
	box.content_margin_right = 14
	box.content_margin_top = 3
	box.content_margin_bottom = 3
	label.add_theme_stylebox_override("normal", box)
	label.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
