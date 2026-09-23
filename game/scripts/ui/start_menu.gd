class_name StartMenu
extends Control

## Écran de démarrage : trois cartes de faction (nom, blason et couleur depuis
## `GameDataStore.get_faction`, description courte), graine, « Commencer » et
## « Charger une partie » (`user://saves/*.json`). Transmet le choix à `SimFacade`
## puis charge `campaign_map.tscn`.

const CAMPAIGN_SCENE := "res://scenes/campaign_map.tscn"

## Textes d'interface (pas des données de jeu) : deux lignes d'accroche par faction jouable.
const PLAYABLE_FACTIONS := [
	{
		"id": "fac_france",
		"fallback_name": "Royaume de France",
		"description": "Le plus riche royaume d'Occident, mais un roi contesté et des vassaux turbulents.\nDéfendez la Guyenne, tenez la Flandre et repoussez l'Anglais.",
	},
	{
		"id": "fac_england",
		"fallback_name": "Royaume d'Angleterre",
		"description": "Un royaume insulaire aux finances tendues, mais aux archers redoutés.\nRevendiquez la couronne de France depuis la Guyenne et la Flandre.",
	},
	{
		"id": "fac_burgundy",
		"fallback_name": "Duché de Bourgogne",
		"description": "Un duché prospère, vassal du roi de France et son beau-frère.\nGrandissez dans l'ombre des deux couronnes sans vous faire dévorer.",
	},
]

@onready var cards: HBoxContainer = %Cards
@onready var seed_edit: LineEdit = %SeedEdit
@onready var start_button: Button = %StartButton
@onready var load_button: Button = %LoadButton
@onready var quit_button: Button = %QuitButton
@onready var status_label: Label = %StatusLabel
@onready var save_load_dialog: SaveLoadDialog = %SaveLoadDialog

var selected_faction: String = "fac_france"
var _card_buttons: Dictionary = {}  # faction_id → Button (toggle)
var _card_panels: Dictionary = {}  # faction_id → PanelContainer


func _ready() -> void:
	_build_cards()
	start_button.pressed.connect(_on_start)
	load_button.pressed.connect(func() -> void: save_load_dialog.open_load())
	quit_button.pressed.connect(func() -> void: get_tree().quit())
	save_load_dialog.load_confirmed.connect(_on_load)
	seed_edit.text = str(SimFacade.pending_seed)
	_select_faction(SimFacade.pending_faction if _card_buttons.has(SimFacade.pending_faction) else "fac_france")
	var store_text := "données chargées" if SimFacade.store_loaded() else "données de jeu indisponibles (core/build.sh ?)"
	status_label.text = "Simulation %s — %s — %d sauvegarde(s)" % [SimFacade.engine_label(), store_text, SimFacade.list_saves().size()]
	# M10 assets : musique du menu et curseurs de volume.
	var audio := get_node_or_null("/root/AudioDirector")
	if audio != null:
		audio.enter_menu()
		$Center/VBox.add_child(audio.make_volume_controls())
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--screenshot="):
			_screenshot_then_quit(arg.trim_prefix("--screenshot="))


## Capture de l'écran de démarrage (`-- --screenshot=<chemin.png>`) puis sortie.
func _screenshot_then_quit(path: String) -> void:
	for _i in 30:
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
		panel.custom_minimum_size = Vector2(300, 260)
		var vbox := VBoxContainer.new()
		vbox.add_theme_constant_override("separation", 8)
		panel.add_child(vbox)

		var swatch := ColorRect.new()
		swatch.custom_minimum_size = Vector2(0, 12)
		swatch.color = info.get("color", Color(0.5, 0.5, 0.5))
		vbox.add_child(swatch)
		# M10 assets : écu procédural de la faction sous la bande de couleur.
		var shield_texture := PortraitLoader.heraldry_texture(faction_id)
		if shield_texture != null:
			var shield := TextureRect.new()
			shield.texture = shield_texture
			shield.custom_minimum_size = Vector2(0, 72)
			shield.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
			shield.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
			vbox.add_child(shield)

		var name_label := Label.new()
		name_label.text = str(info.get("name", entry["fallback_name"])) if SimFacade.store_loaded() else entry["fallback_name"]
		name_label.add_theme_font_size_override("font_size", 24)
		name_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		vbox.add_child(name_label)

		var blazon := Label.new()
		var blazon_text := str(info.get("blazon", ""))
		blazon.text = blazon_text if blazon_text != "" else "Blason inconnu"
		blazon.add_theme_font_size_override("font_size", 14)
		blazon.add_theme_color_override("font_color", Color(0.40, 0.28, 0.14))
		blazon.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		vbox.add_child(blazon)

		var description := Label.new()
		description.text = entry["description"]
		description.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		description.size_flags_vertical = Control.SIZE_EXPAND_FILL
		vbox.add_child(description)

		var choose := Button.new()
		choose.text = "Choisir"
		choose.toggle_mode = true
		choose.button_group = group
		choose.pressed.connect(func() -> void: _select_faction(faction_id))
		vbox.add_child(choose)

		cards.add_child(panel)
		_card_buttons[faction_id] = choose
		_card_panels[faction_id] = panel


func _select_faction(faction_id: String) -> void:
	selected_faction = faction_id
	for id in _card_buttons:
		_card_buttons[id].button_pressed = id == faction_id
		_card_buttons[id].text = "Choisie" if id == faction_id else "Choisir"
		_card_panels[id].modulate = Color(1, 1, 1, 1) if id == faction_id else Color(0.86, 0.86, 0.86, 1)
	start_button.text = "Commencer — %s" % SimFacade.faction_short_name(faction_id)


func card_count() -> int:
	return _card_panels.size()


func _on_start() -> void:
	SimFacade.pending_faction = selected_faction
	SimFacade.pending_seed = int(seed_edit.text) if seed_edit.text.is_valid_int() else seed_edit.text.hash()
	SimFacade.pending_load_path = ""
	get_tree().change_scene_to_file(CAMPAIGN_SCENE)


func _on_load(path: String) -> void:
	SimFacade.pending_load_path = path
	get_tree().change_scene_to_file(CAMPAIGN_SCENE)
