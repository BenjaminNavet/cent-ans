class_name BattleHud
extends CanvasLayer

## HUD parchemin de la bataille (construit en code sur `parchment_theme.tres`) : barre du haut
## (nom, horloge, météo, rapport de forces), journal repliable et regroupé, en bas un bandeau
## compact (U9, UB1) : sceau du chef à gauche, cartes du joueur (`UnitCard`)
## rangées par « bataille » (`BattleGroups`), ordres en icônes, « Retraite générale » à part et
## confirmée, minicarte (`BattleMinimap`) et boutons de vitesse ; aide F1 (F5b, audit UI § 3.2).
## L'écran de fin est `BattleResultScreen` (B2), posé par la scène sur `root`.
## Aucune règle : tout vient de `BattleSim.get_units()` et des événements.

signal card_clicked(unit_id: int, additive: bool)
signal card_double_clicked(unit_id: int)  # B3 / T6 : centrer la caméra sur ce régiment
signal card_hovered(unit_id: int)  # CB-M2 : carte survolée (-1 : plus aucune), contour pâle
signal command_pressed(command: String)
signal speed_pressed(index: int)  # -1 : pause, 0..3 : index dans BattleScene.SPEEDS
signal minimap_clicked(world: Vector2)
signal leader_clicked(double: bool)  # UB1 : sceau du chef (clic : sélection, double : caméra)
signal ui_feedback(kind: String)  # UB1 : sons d'interface (« card », « alert », « cancel »)

const UNIT_CARD := preload("res://scripts/battle/unit_card.gd")
const MINIMAP := preload("res://scripts/battle/battle_minimap.gd")
const ORDERS_BAR := preload("res://scripts/battle/leader_orders_bar.gd")
const ALERTS_COLUMN := preload("res://scripts/battle/battle_alerts_column.gd")  # CB5
## CB3 : ralenti ×0,5 ajouté en tête (`BattleScene.SPEEDS`).
const SPEED_TOOLTIPS := ["Pause (Espace)", "Ralenti ×0,5 (+ / −)", "Vitesse ×1 (+ / −)", "Vitesse ×2 (+ / −)", "Vitesse ×4 (+ / −)"]
const HELP_TEXT := """[b]Bataille — commandes[/b] (F1 : fermer)
• Espace : pause (ordres possibles en pause) · + / − : vitesse ×1, ×2, ×4 (boutons en bas à droite).
• Clic gauche : sélection (glisser : rectangle, Maj : ajouter) · clic sur une carte : sélectionner · double clic sur une carte : centrer la caméra dessus.
• Clic droit : déplacer ou attaquer · double clic droit : au pas de course · glisser-droit : orienter la ligne, sa longueur donne la largeur du front.
• Bannières au-dessus des troupes : clic = sélection, clic droit sur l'ennemi = attaque ; pastilles : déroute, hésite, sous le feu, charge, mode.
• Caméra : {camera}, molette, {rotate}, bouton du milieu ; clic sur la minicarte : y aller.
• C : verrouille la caméra sur la sélection (ou le général) ; suivi doux, bouton du milieu = orbite ; {camera} ou glisser libèrent la caméra.
{keys}"""

const THEME_PATH := "res://scenes/ui/parchment_theme.tres"
const INK := Color(0.22, 0.14, 0.07)
const MAX_LOG := 8
## UB1 / U9 : hauteur du bandeau du bas (cartes de 94 px + libellé de « bataille » + marges),
## lue aussi par la barre des ordres du chef.
const BAND_HEIGHT := 128.0
const SEAL_SIZE := 112.0
## Deux entrées de même texte à moins de GROUP_SECONDS s'agrègent en « (×2) » (audit A3 B4).
const GROUP_SECONDS := 8.0
## Boutons d'ordres : [commande, nom, infobulle] ; le raccourci vient de `BattleHotkeys` (CB2).
const COMMANDS := [
	["formation", "Formation", "Changer de formation (ligne, colonne, schiltron, coin)"],
	["fire_at_will", "Tir à volonté", "Tir à volonté ou tir retenu"],
	["halt", "Halte", "Arrêter la sélection sur place"],
	["withdraw", "Retraite", "Faire quitter le champ aux régiments choisis"],
]
const CATEGORY_ICON := {"infantry": "⚔", "archer": "➶", "cavalry": "♞", "siege": "⚙", "tower": "♜", "ram": "⚒"}

var root: Control
var title_label: Label
var clock_label: Label
var weather_label: Label
var site_label: Label  # B6 : « Sol sec · été · village · haies »
## CV3-2 : badge d'ouverture (« Embuscade ! », « Camp retranché »…), caché en bataille normale.
var opening_badge: Label
var _active_speed: int = 0  # -1 : pause
var balance_bar: Control
var balance_label: Label
var log_box: VBoxContainer
var cards_box: HBoxContainer
var help_panel: PanelContainer  # aide F1 (remplace la ligne d'aide permanente)
var siege_panel: PanelContainer
var siege_label: Label
var minimap: BattleMinimap
var alerts_column: BattleAlertsColumn  # CB5
var groups := BattleGroups.new()
var _speed_buttons: Array[Button] = []
var _battle_columns: Dictionary = {}  # "vanguard"/"main"/"rear" -> VBoxContainer
var _cards: Dictionary = {}  # unit id -> UnitCard
var _balance: Array = [1, 1]
var _colors: Array = [Color.RED, Color.BLUE]
var player_faction: String = ""  # B2 : blason des vignettes
var _log_entries: Array[Dictionary] = []  # {time, text, count}, le plus récent en tête
var log_expanded := true
var log_toggle: Button
var leader_seal: Control
var withdraw_all_button: Button
var confirm_panel: PanelContainer
var _leader: Dictionary = {}  # général du joueur (setup) : character, name, command
var _leader_unit: Dictionary = {}  # son régiment (get_units)
var _leader_portrait: Texture2D = null
var _leader_arms: Texture2D = null
var _card_names: Dictionary = {}  # unit id -> nom distinctif (« Chevaliers II »)
## CB2 : boutons de mode (mode -> Button) et leur état pour la sélection {on, able}.
var _mode_buttons: Dictionary = {}
var _mode_state: Dictionary = {}


func _ready() -> void:
	root = Control.new()
	root.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.mouse_filter = Control.MOUSE_FILTER_IGNORE
	root.theme = load(THEME_PATH)
	add_child(root)
	_build_top_bar()
	_build_log()
	_build_bottom()
	_build_alerts()  # CB5
	# UB1 / U13 : sons d'interface (bus Interface d'AU1).
	ui_feedback.connect(func(kind: String) -> void:
		if kind == "card" or kind == "alert":
			UiSounds.play(kind))


func _label(text: String, size: int = 16) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", size)
	label.add_theme_color_override("font_color", INK)
	return label


func _build_top_bar() -> void:
	var panel := PanelContainer.new()
	panel.anchor_left = 0.5
	panel.anchor_right = 0.5
	panel.offset_left = -470
	panel.offset_right = 470
	panel.offset_top = 8
	root.add_child(panel)
	var box := HBoxContainer.new()
	box.add_theme_constant_override("separation", 18)
	panel.add_child(box)
	title_label = _label("Bataille", 20)
	box.add_child(title_label)
	opening_badge = _label("", 16)
	opening_badge.name = "OpeningBadge"
	opening_badge.add_theme_color_override("font_color", Color(0.62, 0.08, 0.06))
	opening_badge.mouse_filter = Control.MOUSE_FILTER_STOP
	opening_badge.visible = false
	box.add_child(opening_badge)
	clock_label = _label("00:00")
	box.add_child(clock_label)
	# Météo et, dessous, le site en une ligne compacte (B6).
	var sky := VBoxContainer.new()
	sky.add_theme_constant_override("separation", 0)
	box.add_child(sky)
	weather_label = _label("")
	sky.add_child(weather_label)
	site_label = _label("", 12)
	site_label.visible = false
	sky.add_child(site_label)
	var balance := VBoxContainer.new()
	balance.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_child(balance)
	balance_label = _label("", 13)
	balance_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	balance.add_child(balance_label)
	balance_bar = Control.new()
	balance_bar.custom_minimum_size = Vector2(220, 12)
	balance_bar.draw.connect(_draw_balance)
	balance.add_child(balance_bar)
	# Siège (M8) : murailles, brèches, porte, place centrale ; caché en bataille rangée.
	siege_panel = PanelContainer.new()
	siege_panel.anchor_left = 0.5
	siege_panel.anchor_right = 0.5
	siege_panel.offset_left = -300
	siege_panel.offset_right = 300
	siege_panel.offset_top = 62
	siege_panel.visible = false
	root.add_child(siege_panel)
	siege_label = _label("", 14)
	siege_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	siege_panel.add_child(siege_label)


func _draw_balance() -> void:
	var size := balance_bar.size
	var total := maxf(float(_balance[0] + _balance[1]), 1.0)
	var split := size.x * float(_balance[0]) / total
	balance_bar.draw_rect(Rect2(0, 0, split, size.y), _colors[0])
	balance_bar.draw_rect(Rect2(split, 0, size.x - split, size.y), _colors[1])
	balance_bar.draw_rect(Rect2(Vector2.ZERO, size), INK, false, 1.0)


func _build_log() -> void:
	var panel := PanelContainer.new()
	panel.name = "BattleLog"
	panel.anchor_left = 1.0
	panel.anchor_right = 1.0
	panel.offset_left = -360
	panel.offset_right = -12
	panel.offset_top = 70
	panel.mouse_filter = Control.MOUSE_FILTER_PASS
	root.add_child(panel)
	log_box = VBoxContainer.new()
	log_box.add_theme_constant_override("separation", 1)
	panel.add_child(log_box)
	var header := HBoxContainer.new()
	log_box.add_child(header)
	var title := _label("Journal de bataille", 16)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title)
	log_toggle = Button.new()
	log_toggle.name = "LogToggle"
	log_toggle.focus_mode = Control.FOCUS_NONE
	log_toggle.flat = true
	log_toggle.tooltip_text = "Replier ou déplier le journal (J)"
	log_toggle.pressed.connect(toggle_log)
	header.add_child(log_toggle)
	_update_log_toggle()


func toggle_log() -> void:
	log_expanded = not log_expanded
	_update_log_toggle()
	_render_log()


func _update_log_toggle() -> void:
	if log_toggle != null:
		log_toggle.text = "▾ replier" if log_expanded else "▸ déplier"


## CB5 : colonne d'alertes en haut à gauche (le journal est en haut à droite, `_build_log`) ;
## sa hauteur suit son contenu (au plus 5 lignes), bien au-dessus du bandeau du bas.
func _build_alerts() -> void:
	alerts_column = ALERTS_COLUMN.new()
	root.add_child(alerts_column)


func _build_bottom() -> void:
	var panel := PanelContainer.new()
	panel.name = "BottomBand"
	panel.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	panel.offset_left = 8
	panel.offset_right = -8
	panel.offset_top = -BAND_HEIGHT - 6
	panel.offset_bottom = -6
	panel.add_theme_stylebox_override("panel", BattleUiKit.page_box(6))
	root.add_child(panel)
	var outer := HBoxContainer.new()
	outer.add_theme_constant_override("separation", 8)
	panel.add_child(outer)
	outer.add_child(_build_leader_seal())
	outer.add_child(VSeparator.new())
	var scroll := ScrollContainer.new()
	scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	outer.add_child(scroll)
	cards_box = HBoxContainer.new()
	cards_box.add_theme_constant_override("separation", 12)
	scroll.add_child(cards_box)
	for key in BattleGroups.ORDER:
		var column := VBoxContainer.new()
		column.name = "Battle_%s" % key
		column.set_meta("battle", key)
		column.visible = false  # montrée dès sa première carte
		column.add_theme_constant_override("separation", 1)
		column.add_child(_label(str(BattleGroups.LABELS[key]), 12))
		var row := HBoxContainer.new()
		row.name = "Cards"
		row.add_theme_constant_override("separation", 3)
		column.add_child(row)
		cards_box.add_child(column)
		_battle_columns[key] = column
	outer.add_child(VSeparator.new())
	# Ordres en icônes (2 × 2), puis la retraite générale, isolée et confirmée.
	var commands := VBoxContainer.new()
	commands.alignment = BoxContainer.ALIGNMENT_CENTER
	commands.add_theme_constant_override("separation", 5)
	outer.add_child(commands)
	var buttons := GridContainer.new()
	buttons.name = "Commands"
	buttons.columns = 5  # CB2 : ordres puis modes, sur deux rangées
	buttons.add_theme_constant_override("h_separation", 4)
	buttons.add_theme_constant_override("v_separation", 4)
	commands.add_child(buttons)
	for entry in COMMANDS:
		var button := RichButton.new()  # B1 : infobulle riche auto-liée (T : bulle du Codex)
		button.name = "Command_%s" % entry[0]
		button.custom_minimum_size = Vector2(46, 38)
		button.focus_mode = Control.FOCUS_NONE
		var key := BattleHotkeys.key_label(str(entry[0]))
		var key_hint := " (%s)" % key if key != "" else ""
		button.tooltip_text = "[b]%s[/b]%s\n%s" % [entry[1], key_hint, entry[2]]
		button.draw.connect(_draw_command_icon.bind(button, str(entry[0]), key))
		button.pressed.connect(func() -> void: command_pressed.emit(str(entry[0])))
		buttons.add_child(button)
	_build_mode_buttons(buttons)
	withdraw_all_button = RichButton.new()
	withdraw_all_button.name = "WithdrawAll"
	withdraw_all_button.text = "Retraite générale"
	withdraw_all_button.focus_mode = Control.FOCUS_NONE
	withdraw_all_button.tooltip_text = "Sonner la retraite de toute l'armée (confirmation demandée)"
	withdraw_all_button.add_theme_font_size_override("font_size", 12)
	withdraw_all_button.add_theme_color_override("font_color", Color(0.98, 0.92, 0.8))
	withdraw_all_button.add_theme_color_override("font_hover_color", Color(1, 1, 0.92))
	var wax := BattleUiKit.parchment_box(4, Color(0.50, 0.10, 0.07), Color(0.30, 0.05, 0.03), 1)
	wax.shadow_size = 0
	withdraw_all_button.add_theme_stylebox_override("normal", wax)
	var wax_hover := wax.duplicate() as StyleBoxFlat
	wax_hover.bg_color = Color(0.64, 0.16, 0.10)
	withdraw_all_button.add_theme_stylebox_override("hover", wax_hover)
	withdraw_all_button.add_theme_stylebox_override("pressed", wax_hover)
	withdraw_all_button.pressed.connect(ask_withdraw_all)
	commands.add_child(withdraw_all_button)
	outer.add_child(VSeparator.new())
	outer.add_child(_build_corner())
	_build_help()
	_build_confirm()


## Pictogrammes vectoriels des ordres (pas de glyphe de police), raccourci en coin.
func _draw_command_icon(button: Button, command: String, key: String) -> void:
	var c := button.size * 0.5 + Vector2(2, 1)
	var ink := INK
	# DA5 : icône d'ordre à l'encre (or au survol), repli sur le pictogramme vectoriel.
	var texture := HudStyle.icon("battle_" + command)
	if texture != null:
		var side := minf(button.size.x, button.size.y) - 12.0
		# Encre cuite dans le PNG ; modulation = couleur voulue / encre (or au survol).
		var tint := Color.WHITE
		if button.disabled:
			tint = Color(HudStyle.INK_FADED.r / INK.r, HudStyle.INK_FADED.g / INK.g, HudStyle.INK_FADED.b / INK.b, 0.55)
		elif button.is_hovered() or button.button_pressed:
			tint = Color(HudStyle.GOLD.r / INK.r, HudStyle.GOLD.g / INK.g, HudStyle.GOLD.b / INK.b)
		button.draw_texture_rect(texture, Rect2(c - Vector2(side, side) * 0.5, Vector2(side, side)), false, tint)
		command = ""
	match command:
		"formation":  # trois rangs
			for i in 3:
				button.draw_rect(Rect2(c + Vector2(-11, -8 + i * 6), Vector2(22, 3)), ink)
		"fire_at_will":  # arc et flèche
			button.draw_arc(c + Vector2(-4, 0), 10.0, -PI * 0.42, PI * 0.42, 12, ink, 2.0)
			button.draw_line(c + Vector2(-1, -9), c + Vector2(-1, 9), ink, 1.0)
			button.draw_line(c + Vector2(-6, 0), c + Vector2(12, 0), ink, 1.5)
			button.draw_colored_polygon(PackedVector2Array([c + Vector2(13, 0), c + Vector2(8, -3), c + Vector2(8, 3)]), ink)
		"halt":  # main levée (paume)
			button.draw_rect(Rect2(c + Vector2(-6, -2), Vector2(12, 11)), ink)
			for i in 4:
				button.draw_rect(Rect2(c + Vector2(-6 + i * 3.2, -10), Vector2(2.4, 9)), ink)
		"withdraw":  # flèche de repli
			button.draw_line(c + Vector2(10, 4), c + Vector2(-6, 4), ink, 2.5)
			button.draw_colored_polygon(PackedVector2Array([c + Vector2(-11, 4), c + Vector2(-5, -2), c + Vector2(-5, 10)]), ink)
			button.draw_arc(c + Vector2(4, -3), 7.0, -PI * 0.5, PI * 0.5, 8, ink, 2.0)
	if key != "":
		button.draw_string(get_font_for(button), Vector2(4, 12), key, HORIZONTAL_ALIGNMENT_LEFT, -1, 10, BattleUiKit.INK_FADED)


func get_font_for(control: Control) -> Font:
	return control.get_theme_default_font()


## Sceau du chef (à gauche du bandeau, comme le sceau de campagne) : portrait en médaillon sur
## une cire, anneau de moral de sa garde ; clic : sélection, double clic : caméra.
func _build_leader_seal() -> Control:
	leader_seal = Control.new()
	leader_seal.name = "LeaderSeal"
	leader_seal.custom_minimum_size = Vector2(SEAL_SIZE, SEAL_SIZE)
	leader_seal.mouse_filter = Control.MOUSE_FILTER_STOP
	leader_seal.tooltip_text = "Sans chef"
	leader_seal.draw.connect(_draw_leader_seal)
	leader_seal.gui_input.connect(func(event: InputEvent) -> void:
		if event is InputEventMouseButton and event.pressed and (event as InputEventMouseButton).button_index == MOUSE_BUTTON_LEFT:
			ui_feedback.emit("card")
			leader_clicked.emit((event as InputEventMouseButton).double_click))
	return leader_seal


## UB1 : le général du joueur (entrée `general` du setup, ou null) et son blason.
func set_leader(general: Variant, faction: String) -> void:
	_leader = general if general is Dictionary else {}
	_leader_portrait = PortraitLoader.portrait_texture(str(_leader.get("character", "")))
	# DA1 : armes de la maison du général (`house` posé par la scène), à défaut de la faction.
	var house := str(_leader.get("house", HouseArms.house_of(str(_leader.get("character", "")))))
	_leader_arms = PortraitLoader.house_heraldry_texture(house, faction)
	_refresh_leader_tooltip()
	leader_seal.queue_redraw()


func leader_unit_id() -> int:
	return int(_leader_unit.get("id", -1))


func _refresh_leader_tooltip() -> void:
	if _leader.is_empty():
		leader_seal.tooltip_text = "L'ost combat sans général."
		return
	var text := "%s\nCommandement %d / %s" % [str(_leader.get("name", "")), int(_leader.get("command", 0)), RuleValues.text("max_skill_level")]
	if not _leader_unit.is_empty():
		text += "\nGarde du chef : %d hommes · moral %d" % [int(_leader_unit["soldiers"]), int(_leader_unit["morale"])]
		if not bool(_leader_unit["present"]):
			text += "\nLe chef est tombé ou a quitté le champ."
	text += "\nClic : sélectionner sa garde · double clic : y aller"
	leader_seal.tooltip_text = text


func _draw_leader_seal() -> void:
	var c := leader_seal.size * 0.5
	var r := minf(c.x, c.y) - 3.0
	var radius := HudStyle.draw_wax_seal(leader_seal, c, r, _colors[0].darkened(0.45).lerp(HudStyle.WAX, 0.5), 11)
	var inner := radius * 0.74
	# Anneau de moral de la garde du chef.
	var morale := float(_leader_unit.get("morale", 100.0)) / 100.0 if not _leader_unit.is_empty() else 1.0
	leader_seal.draw_arc(c, inner + 4.0, -PI * 0.5, -PI * 0.5 + TAU * clampf(morale, 0.0, 1.0), 48, BattleUnitMarkers.morale_color(morale), 4.0)
	leader_seal.draw_circle(c, inner, BattleUiKit.PARCHMENT_DARK)
	if _leader_portrait != null:
		HudStyle.draw_texture_disc(leader_seal, _leader_portrait, c, inner - 1.0)
	elif _leader_arms != null:
		HudStyle.draw_texture_fit(leader_seal, _leader_arms, c, inner * 1.3)
	leader_seal.draw_arc(c, inner, 0, TAU, 48, BattleUiKit.GOLD, 2.0)
	if not _leader_unit.is_empty() and not bool(_leader_unit["present"]):
		leader_seal.draw_circle(c, inner, Color(0.1, 0.05, 0.03, 0.55))
		leader_seal.draw_line(c + Vector2(-inner, -inner) * 0.6, c + Vector2(inner, inner) * 0.6, BattleUiKit.RUBRIC, 4.0)
	var command := int(_leader.get("command", 0))
	if command > 0:
		var font := BattleUiKit.body_font(700)
		if font == null:
			font = leader_seal.get_theme_default_font()
		var text := "★ %d" % command
		var width := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, 14).x
		var pos := Vector2(c.x - width * 0.5, leader_seal.size.y - 4)
		leader_seal.draw_string_outline(font, pos, text, HORIZONTAL_ALIGNMENT_LEFT, -1, 14, 4, Color(0.1, 0.05, 0.02))
		leader_seal.draw_string(font, pos, text, HORIZONTAL_ALIGNMENT_LEFT, -1, 14, Color(1, 0.84, 0.45))


## « Retraite générale » : demande de confirmation (audit A3 B3).
func _build_confirm() -> void:
	confirm_panel = PanelContainer.new()
	confirm_panel.name = "ConfirmWithdrawAll"
	confirm_panel.anchor_left = 0.5
	confirm_panel.anchor_right = 0.5
	confirm_panel.anchor_top = 0.38
	confirm_panel.anchor_bottom = 0.38
	confirm_panel.offset_left = -260
	confirm_panel.offset_right = 260
	confirm_panel.add_theme_stylebox_override("panel", BattleUiKit.page_box(18))
	confirm_panel.visible = false
	root.add_child(confirm_panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 10)
	confirm_panel.add_child(box)
	var title := BattleUiKit.label("Sonner la retraite générale ?", 26, BattleUiKit.RUBRIC, true)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(title)
	var text := BattleUiKit.label("Tous vos régiments encore en ordre quittent le champ. La bataille sera perdue, mais l'ost sera sauf.", 16)
	text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	text.custom_minimum_size = Vector2(480, 0)
	text.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(text)
	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_CENTER
	row.add_theme_constant_override("separation", 16)
	box.add_child(row)
	var yes := Button.new()
	yes.name = "Confirm"
	yes.text = "Sonner la retraite"
	yes.focus_mode = Control.FOCUS_NONE
	yes.pressed.connect(func() -> void:
		confirm_panel.visible = false
		command_pressed.emit("withdraw_all"))
	row.add_child(yes)
	var no := Button.new()
	no.name = "Cancel"
	no.text = "Tenir le champ"
	no.focus_mode = Control.FOCUS_NONE
	no.pressed.connect(func() -> void:
		confirm_panel.visible = false
		ui_feedback.emit("cancel"))
	row.add_child(no)


func ask_withdraw_all() -> void:
	confirm_panel.visible = true
	ui_feedback.emit("alert")


## J : replier / déplier le journal ; Échap ferme la confirmation de retraite générale.
func _unhandled_input(event: InputEvent) -> void:
	if not (event is InputEventKey) or not event.pressed or event.echo:
		return
	var key := event as InputEventKey
	if key.physical_keycode == KEY_J and not key.ctrl_pressed:
		toggle_log()
		get_viewport().set_input_as_handled()
	elif key.keycode == KEY_ESCAPE and confirm_panel != null and confirm_panel.visible:
		confirm_panel.visible = false
		get_viewport().set_input_as_handled()


## Coin bas droit : minicarte, et à sa droite la colonne des boutons-icônes de vitesse (pause,
## ×0,5, ×1, ×2, ×4 — CB3), pour tenir dans le bandeau compact.
func _build_corner() -> HBoxContainer:
	var corner := HBoxContainer.new()
	corner.add_theme_constant_override("separation", 4)
	minimap = MINIMAP.new()
	minimap.name = "Minimap"
	minimap.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	minimap.clicked.connect(func(world: Vector2) -> void: minimap_clicked.emit(world))
	corner.add_child(minimap)
	var row := VBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_CENTER
	row.add_theme_constant_override("separation", 3)
	corner.add_child(row)
	for index in range(-1, 4):  # CB3 : pause, ×0,5, ×1, ×2, ×4
		var button := Button.new()
		button.name = "Speed%d" % index
		button.custom_minimum_size = Vector2(38, 24)
		button.focus_mode = Control.FOCUS_NONE
		button.toggle_mode = true
		button.tooltip_text = SPEED_TOOLTIPS[index + 1]
		button.pressed.connect(func() -> void: speed_pressed.emit(index))
		button.draw.connect(_draw_speed_icon.bind(button, index))
		row.add_child(button)
		_speed_buttons.append(button)
	return corner


## Pictogramme dessiné (pas de glyphe de police) : deux barres pour la pause, un demi-triangle
## pour le ralenti ×0,5 (CB3), puis 1 à 3 triangles pleins pour ×1, ×2, ×4.
func _draw_speed_icon(button: Button, index: int) -> void:
	var active := index == _active_speed
	var color := Color(0.6, 0.1, 0.08) if active and index < 0 else (Color(0.55, 0.35, 0.02) if active else INK)
	var center := button.size * 0.5
	if active:
		button.draw_rect(Rect2(Vector2(2, 2), button.size - Vector2(4, 4)), Color(0.95, 0.75, 0.15), false, 2.0)
	if index < 0:
		button.draw_rect(Rect2(center + Vector2(-6, -6), Vector2(4, 12)), color)
		button.draw_rect(Rect2(center + Vector2(2, -6), Vector2(4, 12)), color)
		return
	if index == 0:
		button.draw_colored_polygon(PackedVector2Array([Vector2(center.x - 4.5, center.y - 6), Vector2(center.x, center.y), Vector2(center.x - 4.5, center.y + 6)]), color)
		return
	var count := index
	var left := center.x - count * 4.5
	for i in count:
		var x := left + i * 9.0
		button.draw_colored_polygon(PackedVector2Array([Vector2(x, center.y - 6), Vector2(x + 9, center.y), Vector2(x, center.y + 6)]), color)


# --- Mises à jour ---------------------------------------------------------------------


func set_title(text: String, weather: String, colors: Array) -> void:
	title_label.text = text
	weather_label.text = weather
	_colors = colors


## CV3-2 : badge d'ouverture depuis `BattleSim.get_opening()` : « Embuscade ! » quand la
## bataille s'ouvre en embuscade, puis « Marche forcée » / « Camp retranché » des camps concernés.
func set_opening(opening: Dictionary, player_side: String) -> void:
	var parts := PackedStringArray()
	var tips := PackedStringArray()
	if str(opening.get("kind", "standard")) == "ambush":
		parts.append("Embuscade !")
		var victim := str(opening.get("victim", ""))
		tips.append("Vous êtes surpris en colonne de marche : aucun déploiement." if victim == player_side else "L'ennemi est surpris en colonne de marche : frappez ses flancs.")
	for side in ["attacker", "defender"]:
		var own: bool = side == player_side
		if bool(opening.get("%s_forced_march" % side, false)):
			parts.append("Marche forcée")
			tips.append(("Votre ost" if own else "L'ennemi") + " arrive fourbu de marche forcée.")
		if bool(opening.get("%s_entrenched" % side, false)):
			parts.append("Camp retranché")
			tips.append(("Votre ost" if own else "L'ennemi") + " tient un camp retranché : pieux et palissade.")
	opening_badge.text = " · ".join(parts)
	opening_badge.tooltip_text = "\n".join(tips)
	opening_badge.visible = not parts.is_empty()


## B6 : le site de la bataille (sol, saison, village, haies, côte), vide pour le masquer.
func set_site(text: String) -> void:
	site_label.text = text
	site_label.visible = text != ""


func set_clock(seconds: float, speed: float, paused: bool) -> void:
	var total := int(seconds)
	clock_label.text = "%02d:%02d" % [total / 60, total % 60]
	var active := -1 if paused else maxi(BattleScene.SPEEDS.find(speed), 0)
	if active == _active_speed and not _speed_buttons.is_empty() and _speed_buttons[active + 1].button_pressed:
		return
	_active_speed = active
	for i in _speed_buttons.size():
		_speed_buttons[i].set_pressed_no_signal(i - 1 == active)
		_speed_buttons[i].queue_redraw()


func active_speed() -> int:
	return _active_speed


func _build_help() -> void:
	help_panel = PanelContainer.new()
	help_panel.name = "BattleHelp"
	help_panel.anchor_left = 0.5
	help_panel.anchor_right = 0.5
	help_panel.offset_left = -380
	help_panel.offset_right = 380
	help_panel.offset_top = 110
	help_panel.visible = false
	root.add_child(help_panel)
	var text := RichTextLabel.new()
	text.bbcode_enabled = true
	text.fit_content = true
	text.add_theme_color_override("default_color", INK)
	# Touches physiques affichées selon la disposition du clavier (AZERTY : Z Q S D, A / E).
	var label := func(keycode: Key) -> String: return ORDERS_BAR.physical_label(keycode)
	text.text = CodexText.format(HELP_TEXT.format({
		# CB2 : raccourcis tirés de la table unique `BattleHotkeys`.
		"keys": BattleHotkeys.help_bbcode({"orders": ORDERS_BAR.hotkey_labels()}),
		"orders": ORDERS_BAR.hotkey_labels(),
		"camera": " ".join([label.call(KEY_W), label.call(KEY_A), label.call(KEY_S), label.call(KEY_D)]),
		"rotate": "%s / %s" % [label.call(KEY_Q), label.call(KEY_E)],
	}), true)
	help_panel.add_child(text)
	# BP1 : mots du Codex cliquables (bulles imbriquées) dans l'aide de bataille (F1).
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles != null:
		bubbles.call("attach", text)


func toggle_help() -> void:
	help_panel.visible = not help_panel.visible


func set_siege_status(text: String) -> void:
	siege_panel.visible = text != ""
	siege_label.text = text


func set_balance(player_name: String, player_strength: int, enemy_name: String, enemy_strength: int) -> void:
	_balance = [player_strength, enemy_strength]
	balance_label.text = "%s %d — %d %s" % [player_name, player_strength, enemy_strength, enemy_name]
	balance_bar.queue_redraw()


func add_events(events: Array) -> void:
	for event in events:
		var time := float(event.get("time", 0.0))
		var text := str(event.get("text_fr", ""))
		var merged := false
		# Regroupe les répétitions proches (« Les Archers… plantent leurs pieux (×2) »).
		for entry in _log_entries:
			if time - float(entry["time"]) > GROUP_SECONDS:
				break
			if str(entry["text"]) == text:
				entry["count"] = int(entry["count"]) + 1
				entry["time"] = time
				merged = true
				break
		if not merged:
			_log_entries.push_front({"time": time, "text": text, "count": 1})
	while _log_entries.size() > MAX_LOG:
		_log_entries.pop_back()
	_render_log()


func _render_log() -> void:
	if log_box == null:
		return
	for i in range(log_box.get_child_count() - 1, 0, -1):
		var child := log_box.get_child(i)
		log_box.remove_child(child)
		child.queue_free()
	var shown := _log_entries.size() if log_expanded else mini(1, _log_entries.size())
	for i in shown:
		var entry: Dictionary = _log_entries[i]
		var seconds := int(float(entry["time"]))
		var text := "%02d:%02d  %s" % [seconds / 60, seconds % 60, str(entry["text"])]
		if int(entry["count"]) > 1:
			text += " (×%d)" % int(entry["count"])
		var label := _label(text, 13)
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		label.custom_minimum_size = Vector2(330, 0)
		if i > 0:
			label.modulate = Color(1, 1, 1, 0.8)
		log_box.add_child(label)


func log_line_count() -> int:
	return _log_entries.size()


## Cartes des unités du joueur (créées une fois dans la colonne de leur « bataille »).
func update_cards(units: Array, side: String, selected: Array) -> void:
	if _card_names.is_empty():
		_card_names = distinct_names(units, side)
	# CB1 : une unité en déroute ou hors du champ quitte son groupe verrouillé.
	groups.prune_locks(units)
	for unit in units:
		if str(unit["side"]) != side:
			continue
		var id := int(unit["id"])
		if not _cards.has(id):
			_cards[id] = _make_card(unit)
		var card: UnitCard = _cards[id]
		card.refresh(unit, selected.has(id))
		card.set_groups(groups.numbers_of(id))
		card.set_locked(groups.is_locked(id))
		if bool(unit["is_general"]):
			var changed := _leader_unit.is_empty() or int(_leader_unit["morale"]) != int(unit["morale"]) or bool(_leader_unit["present"]) != bool(unit["present"])
			_leader_unit = unit
			if changed:
				_refresh_leader_tooltip()
				leader_seal.queue_redraw()
	update_mode_buttons(units, selected)


## CB2 : un bouton par mode d'unité (après les ordres), glyphe dessiné en code, raccourci de
## `BattleHotkeys`, infobulle chiffrée par RuleValues ; bascule le mode sur la sélection.
func _build_mode_buttons(grid: GridContainer) -> void:
	for entry in BattleModeIcons.MODES:
		var mode := str(entry["mode"])
		var button := RichButton.new()
		button.name = "Mode_%s" % mode
		button.custom_minimum_size = Vector2(46, 38)
		button.focus_mode = Control.FOCUS_NONE
		var key := BattleHotkeys.key_label(mode)
		if key.length() > 3:  # « bouton » : pas de touche
			key = ""
		var key_hint := " (%s)" % key if key != "" else ""
		button.tooltip_text = "[b]%s[/b]%s\n%s" % [str(entry["label"]), key_hint, BattleModeIcons.tip_of(mode)]
		button.draw.connect(_draw_mode_button.bind(button, mode, key))
		button.pressed.connect(func() -> void: command_pressed.emit(mode))
		grid.add_child(button)
		_mode_buttons[mode] = button


## CB2 : état des boutons de mode pour la sélection : actif (fond doré) si toutes les unités
## qui peuvent le prendre l'ont, grisé si aucune ne le peut.
func update_mode_buttons(units: Array, selected: Array) -> void:
	var state := mode_button_state(units, selected)
	if state == _mode_state:
		return
	_mode_state = state
	for mode in _mode_buttons:
		var button: Button = _mode_buttons[mode]
		button.disabled = not bool(state[mode]["able"])
		button.queue_redraw()


## CB2 : {mode: {on, able}} des modes pour les unités `selected` (fonction pure, testée).
static func mode_button_state(units: Array, selected: Array) -> Dictionary:
	var out := {}
	for entry in BattleModeIcons.MODES:
		var mode := str(entry["mode"])
		var able := 0
		var on := 0
		for unit in units:
			if not selected.has(int(unit["id"])) or not Array(unit.get("modes", [])).has(mode):
				continue
			able += 1
			if bool(unit.get(str(entry["field"]), false)):
				on += 1
		out[mode] = {"able": able > 0, "on": able > 0 and on == able}
	return out


func _draw_mode_button(button: Button, mode: String, key: String) -> void:
	var state: Dictionary = _mode_state.get(mode, {"on": false, "able": false})
	var c := button.size * 0.5 + Vector2(2, 2)
	if bool(state["on"]):
		button.draw_rect(Rect2(Vector2(3, 3), button.size - Vector2(6, 6)), Color(HudStyle.GOLD, 0.45))
	var ink := INK
	if button.disabled:
		ink = Color(HudStyle.INK_FADED, 0.55)
	elif button.is_hovered():
		ink = HudStyle.GOLD.darkened(0.3)
	BattleModeIcons.draw_mode(button, mode, c, 1.15, ink)
	if key != "":
		button.draw_string(get_font_for(button), Vector2(4, 12), key, HORIZONTAL_ALIGNMENT_LEFT, -1, 10, BattleUiKit.INK_FADED)


## Noms distinctifs des régiments homonymes d'un camp : « Chevaliers I », « Chevaliers II »…
## (audit A3 B6), dans l'ordre de `units`.
static func distinct_names(units: Array, side: String) -> Dictionary:
	var counts := {}
	for unit in units:
		if str(unit["side"]) == side:
			var name := str(unit["name"])
			counts[name] = int(counts.get(name, 0)) + 1
	var seen := {}
	var names := {}
	for unit in units:
		if str(unit["side"]) != side:
			continue
		var name := str(unit["name"])
		if int(counts[name]) > 1:
			seen[name] = int(seen.get(name, 0)) + 1
			name = "%s %s" % [name, roman(int(seen[name]))]
		names[int(unit["id"])] = name
	return names


static func roman(value: int) -> String:
	const NUMERALS := ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII"]
	return NUMERALS[value - 1] if value >= 1 and value <= NUMERALS.size() else str(value)


func _make_card(unit: Dictionary) -> UnitCard:
	var battle_key := BattleGroups.default_battle(unit)
	var column: VBoxContainer = _battle_columns[battle_key]
	column.visible = true
	var card: UnitCard = UNIT_CARD.new()
	(column.get_node("Cards") as HBoxContainer).add_child(card)
	card.setup(unit, get_node_or_null("/root/IconLibrary"), player_faction, _colors[0])
	if _card_names.has(int(unit["id"])):
		card.unit_name = str(_card_names[int(unit["id"])]) + (" ★" if card.is_general else "")
	card.clicked.connect(func(id: int, additive: bool) -> void:
		ui_feedback.emit("card")
		card_clicked.emit(id, additive))
	card.double_clicked.connect(func(id: int) -> void: card_double_clicked.emit(id))
	var hovered_id := int(unit["id"])
	card.mouse_entered.connect(func() -> void: card_hovered.emit(hovered_id))
	card.mouse_exited.connect(func() -> void: card_hovered.emit(-1))
	return card


func card_count() -> int:
	return _cards.size()


## Clé de « bataille » de la carte de l'unité ("" si pas de carte).
func card_battle(unit_id: int) -> String:
	if not _cards.has(unit_id):
		return ""
	return str((_cards[unit_id] as Node).get_parent().get_parent().get_meta("battle", ""))


# --- F5c : message éphémère (refus de déploiement, sortie de la garnison) ---------------

const TOAST_SECONDS := 4.0
var toast_label: Label = null
var _toast_serial: int = 0


func show_toast(text: String, is_error: bool = true) -> void:
	if toast_label == null:
		var panel := PanelContainer.new()
		panel.name = "Toast"
		panel.anchor_left = 0.5
		panel.anchor_right = 0.5
		panel.anchor_top = 0.3
		panel.anchor_bottom = 0.3
		panel.offset_left = -300
		panel.offset_right = 300
		panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
		root.add_child(panel)
		toast_label = _label("", 16)
		toast_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		toast_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		panel.add_child(toast_label)
	toast_label.text = text
	toast_label.add_theme_color_override("font_color", Color(0.55, 0.12, 0.10) if is_error else INK)
	toast_label.get_parent().visible = true
	_toast_serial += 1
	var serial := _toast_serial
	get_tree().create_timer(TOAST_SECONDS).timeout.connect(func() -> void:
		if serial == _toast_serial and toast_label != null:
			toast_label.get_parent().visible = false)
