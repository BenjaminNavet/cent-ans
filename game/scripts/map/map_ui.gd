class_name MapUI
extends CanvasLayer

## Couche UI de la carte de campagne : barre supérieure (faction, trésor, solde, date, menu),
## HUD « à la Total War » (F10b : bandeau d'ost `ArmyStrip` et sceau du chef `GeneralSeal` pour
## l'armée sélectionnée, cloche de fin de saison `EndTurnCluster`, lettres scellées
## `NewsLetters`), journal des événements (bas gauche, replié par défaut), panneau de province,
## aperçu de chemin au survol, notifications, dialogue sauver/charger.
## Ne connaît pas la simulation : `CampaignMap` alimente les vues et reçoit les signaux.

signal end_turn_pressed
signal save_requested(save_name: String)
signal load_requested(path: String)
signal main_menu_requested
signal quit_requested
signal recruit_requested(province_id: String, unit_type: String)
signal create_army_requested(province_id: String, unit_indices: Array)
signal build_requested(province_id: String, building_id: String)
signal cancel_build_requested(province_id: String)
signal tax_rate_changed(faction_id: String, rate: String)
signal stance_changed(army_id: String, stance: String)
## Bandeau d'ost : régiments à détacher de l'armée `army_id` (ordre `split_army`).
signal army_split_requested(army_id: String, unit_indices: Array)
## Bandeau d'ost (lot C7d) : régiments de l'armée `army_id` à laisser en garnison de la
## colonie où elle se trouve (ordre `garrison_units`).
signal army_garrison_requested(army_id: String, unit_indices: Array)
## Sceau du chef : fiche du chef (`character_id`), ou `""` si l'armée n'a pas de chef.
signal army_general_clicked(character_id: String)
## Cloche : clic sur une pastille d'alerte (ou sur la cloche quand une décision bloque).
signal alert_activated(alert: Dictionary)
## Lettre scellée ouverte.
signal news_activated(item: Dictionary)
signal province_panel_closed
signal faction_panel_requested
signal court_panel_requested
signal province_court_requested
signal character_selected(character_id: String)
signal governor_requested(character_id: String, province_id: String)
signal general_requested(character_id: String, army_id: String)
signal marriage_requested(character_id: String, spouse_id: String)
signal learn_skill_requested(character_id: String, skill_id: String)
# M6 : technologies.
signal tech_panel_requested
signal research_requested(technology_id: String)

const MENU_SAVE := 0
const MENU_LOAD := 1
const MENU_MAIN := 3
const MENU_QUIT := 4
const MAX_LOG_LINES := 200
const TOAST_SECONDS := 3.5

@onready var faction_swatch: ColorRect = %FactionSwatch
@onready var faction_label: Label = %FactionLabel
@onready var treasury_label: Label = %TreasuryLabel
@onready var income_label: Label = %IncomeLabel
@onready var date_label: Label = %DateLabel
@onready var menu_button: MenuButton = %MenuButton
@onready var toast: Label = %Toast
@onready var hover_label: Label = %HoverLabel
@onready var event_log: PanelContainer = %EventLog
@onready var log_title: Label = %LogTitle
@onready var log_toggle: Button = %LogToggle
@onready var log_scroll: ScrollContainer = %LogScroll
@onready var log_text: RichTextLabel = %LogText
# F10b : HUD de campagne (noms de nœuds stables, ciblés par le tutoriel).
@onready var army_strip: ArmyStrip = %ArmyStrip
@onready var general_seal: GeneralSeal = %GeneralSeal
@onready var end_turn_cluster: EndTurnCluster = %EndTurnCluster
@onready var news_letters: NewsLetters = %NewsLetters
## Minicarte (lot C1), ajoutée par `MinimapController` ; placée en haut à droite, lettres dessous.
var minimap: Control = null
## Lot C7b : panneaux ancrés comme le panneau de province, à gauche de la minicarte (panneau de
## colonie) ; voir `dock_right_panel`.
var docked_panels: Array[Control] = []
## Abscisse écran du bord droit de ces panneaux (dernier `layout_hud`).
var docked_right_x: float = 0.0
@onready var province_panel: ProvincePanel = %ProvincePanel
@onready var faction_panel: FactionPanel = %FactionPanel
@onready var court_button: Button = %CourtButton
@onready var court_panel: CourtPanel = %CourtPanel
@onready var character_sheet: CharacterSheet = %CharacterSheet
@onready var save_load_dialog: SaveLoadDialog = %SaveLoadDialog
# M6 : technologies.
@onready var tech_button: Button = %TechButton
@onready var research_box: VBoxContainer = %ResearchBox
@onready var research_label: Label = %ResearchLabel
@onready var research_bar: ProgressBar = %ResearchBar
@onready var tech_panel: TechPanel = %TechPanel

var _log_lines: PackedStringArray = PackedStringArray()
var _toast_timer: SceneTreeTimer


func _ready() -> void:
	menu_button.get_popup().id_pressed.connect(_on_menu_item)
	# M10 assets : entrée « Son… » (volumes Musique / Effets) dans le menu.
	var audio := get_node_or_null("/root/AudioDirector")
	if audio != null:
		audio.add_sound_menu(menu_button.get_popup(), self)
	log_toggle.pressed.connect(_toggle_log)
	province_panel.hide()
	province_panel.recruit_requested.connect(func(p: String, u: String) -> void: recruit_requested.emit(p, u))
	province_panel.create_army_requested.connect(func(p: String, i: Array) -> void: create_army_requested.emit(p, i))
	province_panel.build_requested.connect(func(p: String, b: String) -> void: build_requested.emit(p, b))
	province_panel.cancel_build_requested.connect(func(p: String) -> void: cancel_build_requested.emit(p))
	province_panel.closed.connect(func() -> void: province_panel_closed.emit())
	province_panel.court_requested.connect(func() -> void: province_court_requested.emit())
	faction_panel.closed.connect(func() -> void: faction_panel.hide())
	faction_panel.tax_rate_changed.connect(func(f: String, r: String) -> void: tax_rate_changed.emit(f, r))
	faction_swatch.gui_input.connect(_on_faction_swatch_input)
	faction_label.gui_input.connect(_on_faction_swatch_input)
	faction_panel.hide()
	court_button.pressed.connect(func() -> void: court_panel_requested.emit())
	court_panel.character_selected.connect(func(id: String) -> void: character_selected.emit(id))
	court_panel.closed.connect(func() -> void: court_panel.hide())
	# Le journal occupe la même colonne : masqué tant que la cour est ouverte.
	court_panel.visibility_changed.connect(func() -> void: event_log.visible = not court_panel.visible)
	court_panel.hide()
	character_sheet.closed.connect(func() -> void: character_sheet.hide())
	character_sheet.character_requested.connect(func(id: String) -> void: character_selected.emit(id))
	character_sheet.governor_requested.connect(func(c: String, p: String) -> void: governor_requested.emit(c, p))
	character_sheet.general_requested.connect(func(c: String, a: String) -> void: general_requested.emit(c, a))
	character_sheet.marriage_requested.connect(func(c: String, s: String) -> void: marriage_requested.emit(c, s))
	character_sheet.learn_skill_requested.connect(func(c: String, s: String) -> void: learn_skill_requested.emit(c, s))
	character_sheet.hide()
	# --- M6 : technologies ---
	tech_button.pressed.connect(func() -> void: tech_panel_requested.emit())
	research_box.gui_input.connect(func(event: InputEvent) -> void:
		if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			tech_panel_requested.emit())
	tech_panel.research_requested.connect(func(id: String) -> void: research_requested.emit(id))
	tech_panel.closed.connect(func() -> void: tech_panel.hide())
	# Le panneau recouvre le journal : masqué tant que les technologies sont ouvertes.
	tech_panel.visibility_changed.connect(func() -> void: event_log.visible = not tech_panel.visible and not court_panel.visible)
	tech_panel.hide()
	# --- fin M6 ---
	save_load_dialog.save_confirmed.connect(func(n: String) -> void: save_requested.emit(n))
	save_load_dialog.load_confirmed.connect(func(p: String) -> void: load_requested.emit(p))
	save_load_dialog.dialog_closed.connect(func() -> void: set_end_turn_enabled(true))
	hover_label.text = ""
	hover_label.hide()
	toast.hide()
	_decorate_top_bar()  # F2
	_setup_hud()  # F10b : la cloche porte seule le raccourci `campaign_end_turn`


# --- Icônes et infobulles de la barre (F2) -------------------------------------------


const TOP_ICON_SIZE := 20.0
var _season_icon: TextureRect
var _treasury_icon: TextureRect
var _income_icon: TextureRect
var _research_icon: TextureRect


func _decorate_top_bar() -> void:
	var bar := treasury_label.get_parent()
	bar.add_theme_constant_override("separation", 5)
	_treasury_icon = _insert_icon_before(treasury_label, "hud_treasury")
	_income_icon = _insert_icon_before(income_label, "hud_income")
	_season_icon = _insert_icon_before(date_label, "hud_season_spring")
	_research_icon = _insert_icon_before(research_box, "hud_research")
	for label in [treasury_label, income_label, date_label]:
		label.set_script(RichLabel)
		label.mouse_filter = Control.MOUSE_FILTER_PASS
	treasury_label.tooltip_text = RichTooltip.hud("hud_treasury")
	income_label.tooltip_text = RichTooltip.hud("hud_income")
	# Boutons à icône seule (le libellé passe dans l'infobulle) : la barre tient en 1440 px.
	_decorate_button(court_button, "hud_court", true)
	_decorate_button(tech_button, "hud_technologies", true)
	_add_codex_button()
	# Boutons ajoutés par les contrôleurs (Diplomatie, Chronique) après ce _ready.
	bar.child_entered_tree.connect(func(node: Node) -> void: _decorate_late_button.call_deferred(node))


func _insert_icon_before(control: Control, icon_id: String) -> TextureRect:
	var rect: TextureRect = IconLibrary.make_rect(icon_id, TOP_ICON_SIZE)
	var parent := control.get_parent()
	parent.add_child(rect)
	parent.move_child(rect, control.get_index())
	return rect


func _decorate_button(button: Button, icon_id: String, icon_only: bool = false) -> void:
	IconLibrary.decorate_button(button, icon_id, int(TOP_ICON_SIZE) + (6 if icon_only else 0))
	if icon_only:
		button.text = ""
	if not (button is RichButton):
		button.set_script(RichButton)
	button.tooltip_text = RichTooltip.hud(icon_id)


func _decorate_late_button(node: Node) -> void:
	if not (node is Button) or not is_instance_valid(node) or (node as Button).icon != null:
		return
	var button := node as Button
	if button.text.begins_with("Diplomatie"):
		_decorate_button(button, "hud_diplomacy", true)
	elif button.text.begins_with("Chronique"):
		_decorate_button(button, "hud_chronicle")


# --- Barre supérieure ------------------------------------------------------------


func set_faction(label: String, color: Color) -> void:
	faction_label.text = label
	faction_swatch.color = color


## `economy` : `get_faction_economy` (vide si indisponible). Affiche le revenu **net** prévu
## (recettes - entretien des armées, des bâtiments et de la cour) ; détail en infobulle.
## Sans économie, repli sur `income` (revenu brut du dernier tour).
func set_treasury(treasury: int, income: int, economy: Dictionary = {}) -> void:
	treasury_label.text = "Trésor : %s ℔" % ProvincePanel._thousands(treasury)
	if economy.is_empty():
		income_label.text = "Revenu : %s ℔" % _signed(income)
		income_label.tooltip_text = "Revenu brut du dernier tour."
		return
	var gross := int(economy.get("projected_income", income))
	var armies := int(economy.get("army_upkeep", 0))
	var buildings := int(economy.get("building_upkeep", 0))
	var court := int(economy.get("administration_upkeep", 0))
	var table := int(economy.get("table_upkeep", 0))  # H3 : la Table (diètes des provinces)
	# H5 : le seigneuriage est déjà dans les recettes, la refonte dans l'administration.
	var seigniorage := int(economy.get("seigniorage", 0))
	var recoinage := int(economy.get("recoinage", 0))
	var net := gross - armies - buildings - court - table
	income_label.text = "Solde : %s ℔ / saison" % _signed(net)
	income_label.add_theme_color_override("font_color", Color(0.55, 0.12, 0.10) if net < 0 else Color(0.22, 0.14, 0.07))
	var lines := PackedStringArray(["Prévision pour la prochaine saison"])
	lines.append("Recettes : %s ℔" % ProvincePanel._thousands(gross))
	if seigniorage != 0:
		lines.append("    dont seigneuriage : %s ℔" % _signed(seigniorage))
	lines.append("Armées : -%s ℔" % ProvincePanel._thousands(armies))
	lines.append("Bâtiments : -%s ℔" % ProvincePanel._thousands(buildings))
	lines.append("Cour et administration : -%s ℔" % ProvincePanel._thousands(court))
	if recoinage != 0:
		lines.append("    dont refonte des monnaies : -%s ℔" % ProvincePanel._thousands(recoinage))
	if table != 0:
		lines.append("La Table : -%s ℔" % ProvincePanel._thousands(table))
	lines.append("Solde : %s ℔" % _signed(net))
	income_label.tooltip_text = "\n".join(lines)


static func _signed(value: int) -> String:
	return ("+" if value >= 0 else "-") + ProvincePanel._thousands(absi(value))


func set_date(text: String) -> void:
	date_label.text = text
	var season := RichTooltip.season_of(text)
	if _season_icon != null and season != "":
		_season_icon.texture = IconLibrary.get_icon("hud_season_" + season)
		date_label.tooltip_text = RichTooltip.hud("hud_season_" + season, "Un tour = une saison.")


func set_end_turn_enabled(enabled: bool) -> void:
	end_turn_cluster.set_end_turn_enabled(enabled)


func _on_menu_item(id: int) -> void:
	match id:
		MENU_SAVE:
			set_end_turn_enabled(false)
			save_load_dialog.open_save("partie_%s" % Time.get_date_string_from_system())
		MENU_LOAD:
			set_end_turn_enabled(false)
			save_load_dialog.open_load()
		MENU_MAIN:
			main_menu_requested.emit()
		MENU_QUIT:
			quit_requested.emit()


func is_dialog_open() -> bool:
	return save_load_dialog.visible


# --- Survol et notifications -------------------------------------------------------


func set_hovered(province: Dictionary) -> void:
	hover_label.text = str(province.get("display_name", province.get("name", ""))) if not province.is_empty() else ""
	if hover_label.text == "" and army_strip.visible:
		hover_label.text = _army_status  # F10b : position et ordre de l'armée sélectionnée
	hover_label.visible = hover_label.text != ""
	_fit_hover_label()


## Survol d'une destination avec une armée sélectionnée : nom, coût et faisabilité.
func set_hover_path(province_name: String, steps: int, cost: int, reachable_this_turn: bool) -> void:
	if steps <= 0:
		hover_label.text = "%s — aucun chemin" % province_name
	elif reachable_this_turn:
		hover_label.text = "→ %s : %d étape%s, coût %d — clic droit pour partir" % [province_name, steps, "s" if steps > 1 else "", cost]
	else:
		hover_label.text = "→ %s : %d étape%s, plusieurs tours — clic droit pour partir" % [province_name, steps, "s" if steps > 1 else ""]
	hover_label.visible = true
	_fit_hover_label()


func show_toast(text: String, is_error: bool = false) -> void:
	toast.text = text
	toast.add_theme_color_override("font_color", Color(0.55, 0.12, 0.10) if is_error else Color(0.22, 0.14, 0.07))
	toast.show()
	_toast_timer = get_tree().create_timer(TOAST_SECONDS)
	var timer := _toast_timer
	timer.timeout.connect(func() -> void:
		if _toast_timer == timer:
			toast.hide())


# --- Journal des événements ------------------------------------------------------------


## Faction du joueur et nom court d'une faction (`Callable(id) -> String`) : le journal
## masque les événements courants des autres factions et nomme la faction des autres.
var journal_player_faction: String = ""
var journal_faction_name: Callable = Callable()

## Événements d'une autre faction sans intérêt pour le joueur (gestion interne).
const FOREIGN_MINOR_KINDS := [
	"income", "bankruptcy", "attrition", "recruited", "building_completed", "trait_acquired",
	"skill_learned", "appointment", "technology_researched", "regency", "birth", "raid",
]


## Vrai si l'événement doit figurer au journal du joueur.
func journal_keeps(event: Dictionary) -> bool:
	var faction := str(event.get("faction", ""))
	if faction == "" or journal_player_faction == "" or faction == journal_player_faction:
		return true
	return not FOREIGN_MINOR_KINDS.has(str(event.get("kind", "")))


## Texte de l'événement, préfixé du nom de sa faction s'il ne la nomme pas déjà.
func journal_text(event: Dictionary) -> String:
	var text := str(event.get("text_fr", event.get("text", "")))
	var faction := str(event.get("faction", ""))
	if text == "" or faction == "" or faction == journal_player_faction or not journal_faction_name.is_valid():
		return text
	var name := str(journal_faction_name.call(faction))
	if name == "" or text.contains(name):
		return text
	return "%s — %s" % [name, text]


## Ajoute les événements d'un tour en tête du journal (plus récents en haut).
func add_events(events: Array, date_text: String) -> void:
	var new_lines := PackedStringArray()
	for event in events:
		if not journal_keeps(event):
			continue
		var news := NewsLetters.news_from_event(event)  # F10b : lettre scellée (trace persistante)
		if not news.is_empty():
			news_letters.push_news(news)
		var kind: String = str(event.get("kind", ""))
		var text: String = journal_text(event)
		if text == "":
			continue
		var line: String
		if kind == "battle" or kind == "siege" or kind == "province_taken":
			line = "[color=#8b1a1a][b]⚔ %s[/b][/color]" % text
		elif kind == "revolt":
			line = "[color=#a1121a][b]⚑ %s[/b][/color]" % text
		elif kind == "plague":
			line = "[color=#4a6b2a][b]☠ %s[/b][/color]" % text
		elif kind == "famine":
			line = "[color=#8a5a10][b]⚠ %s[/b][/color]" % text
		elif kind == "building_completed":
			line = "[color=#1a5c8b]⚒ %s[/color]" % text
		elif kind == "birth":
			line = "[color=#2a7a4a]✚ %s[/color]" % text
		elif kind == "marriage":
			line = "[color=#8a3a8a][b]♥ %s[/b][/color]" % text
		elif kind == "death":
			line = "[color=#3a3a3a][b]✝ %s[/b][/color]" % text
		elif kind == "succession":
			line = "[color=#7a5a10][b]♔ %s[/b][/color]" % text
		elif kind == "regency":
			line = "[color=#7a5a10]⚖ %s[/color]" % text
		elif kind == "trait_acquired":
			line = "[color=#2a5a7a]✦ %s[/color]" % text
		elif kind == "skill_learned":
			line = "[color=#2a5a7a]★ %s[/color]" % text
		elif kind == "appointment":
			line = "[color=#4a3a10]⚑ %s[/color]" % text
		elif kind == "war_declared":
			line = "[color=#8b1a1a][b]⚔ %s[/b][/color]" % text
		elif kind == "peace_signed":
			line = "[color=#2a6a2a][b]☮ %s[/b][/color]" % text
		elif kind == "alliance_formed" or kind == "vassalage":
			line = "[color=#1a3a8b][b]⚜ %s[/b][/color]" % text
		elif kind == "alliance_broken" or kind == "vassal_rebellion":
			line = "[color=#a1121a][b]⚡ %s[/b][/color]" % text
		elif kind == "embargo" or kind == "diplomatic_offer" or kind == "diplomacy":
			line = "[color=#4a3a10]✉ %s[/color]" % text
		elif kind == "excommunication" or kind == "schism" or kind == "heresy":
			line = "[color=#5a2a6a][b]✠ %s[/b][/color]" % text
		elif kind == "chronicle":  # M10
			line = "[color=#7a3b0c][b]§ %s[/b][/color]" % text
		elif kind == "technology_researched":
			line = "[color=#5a2a8a][b]⚙ %s[/b][/color]" % text
		elif kind == "income":
			line = "[color=#4a3a10]%s[/color]" % text
		elif SeasonReport.KIND_STYLES.has(kind):  # H3/H4/H11 : table, médecine, monnaie, rançon, chevalerie
			var style: Dictionary = SeasonReport.KIND_STYLES[kind]
			line = "[color=%s]%s %s[/color]" % [style["color"], style["glyph"], text]
		else:
			line = text
		new_lines.append(line)
	if new_lines.is_empty():
		new_lines.append("[i]Rien à signaler.[/i]")
	var header := "[b]— %s —[/b]" % date_text
	var block := PackedStringArray([header])
	block.append_array(new_lines)
	block.append_array(_log_lines)
	_log_lines = block.slice(0, mini(block.size(), MAX_LOG_LINES))
	_render_log()
	log_title.text = "Journal (%d)" % new_lines.size()


func clear_log() -> void:
	_log_lines = PackedStringArray()
	news_letters.clear()
	log_text.text = "[i]Aucun événement pour l'instant.[/i]"
	log_title.text = "Journal"


func _render_log() -> void:
	log_text.text = "\n".join(_log_lines)
	log_scroll.scroll_vertical = 0


func _toggle_log() -> void:
	set_log_expanded(not log_scroll.visible)


## Journal déplié ou replié (replié par défaut : le sceau et le bandeau occupent le bas).
func set_log_expanded(expanded: bool) -> void:
	log_scroll.visible = expanded
	log_toggle.text = "Replier" if expanded else "Déplier"
	queue_layout()


func log_line_count() -> int:
	return _log_lines.size()


# --- Panneaux ----------------------------------------------------------------------


func show_province(province: Dictionary, state: Dictionary = {}, recruitable: Array = [], is_player_owner: bool = false, label_of: Callable = Callable(), city: Dictionary = {}) -> void:
	province_panel.show_province(province, state, recruitable, is_player_owner, label_of, city)


func hide_province() -> void:
	province_panel.hide()


func show_faction(id: String, label: String, color: Color, economy: Dictionary) -> void:
	faction_panel.show_faction(id, label, color, economy)


func hide_faction() -> void:
	faction_panel.hide()


func faction_panel_visible() -> bool:
	return faction_panel.visible


func _on_faction_swatch_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		faction_panel_requested.emit()


## F10b : armée sélectionnée dans le bandeau et le sceau. `army` = `get_army(id)`,
## `character` = `get_character(army.general)` (vide = sans chef), `title` = rubrique du
## bandeau, `status` = position et ordre en cours (étiquette au-dessus du bandeau),
## `can_split` = la simulation accepte `split_army` et l'armée est au joueur. Lot C7d :
## `can_garrison` affiche le bouton « Garnison » (armée du joueur sur une colonie qu'il
## contrôle) ; `garrison_disabled_reason` le désactive avec une infobulle française
## (colonie assiégée ou pleine) quand il n'est pas vide.
func show_army(army_id: String, army: Dictionary, character: Dictionary, faction: String, is_player: bool, title: String, status: String, can_split: bool = false, can_garrison: bool = false, garrison_disabled_reason: String = "") -> void:
	if army.is_empty():
		hide_army()
		return
	if _unit_catalog.is_empty():
		_unit_catalog = ArmyStrip.load_unit_catalog(MapPaths.data_dir)
	var keep: Array = []
	if army_id == current_army_id and army_strip.visible:
		var count := (army.get("units", []) as Array).size()
		for index in army_strip.get_selection():
			if index < count:
				keep.append(index)
	current_army_id = army_id
	army_strip.can_split = can_split
	army_strip.can_garrison = can_garrison
	army_strip.garrison_disabled_reason = garrison_disabled_reason
	army_strip.set_army(army, int(army.get("max_units", DEFAULT_ARMY_CAPACITY)), _unit_catalog, title)
	if not keep.is_empty():
		army_strip.select(keep)
	general_seal.can_change_stance = is_player
	general_seal.set_general(character, army, faction)
	general_seal.show()
	_army_status = status
	set_hovered({})
	queue_layout()


func hide_army() -> void:
	current_army_id = ""
	_army_status = ""
	army_strip.clear()
	army_strip.hide()
	general_seal.hide()
	army_actions.hide()
	set_hovered({})
	queue_layout()


func show_court(rows: Array[Dictionary], faction_label: String, faction_color: Color, preset_filter: int = -1) -> void:
	court_panel.show_court(rows, faction_label, faction_color, preset_filter)


func hide_court() -> void:
	court_panel.hide()


func court_panel_visible() -> bool:
	return court_panel.visible


func show_character(character: Dictionary, skill_tree: Array, learnable: Array, governable: Array, commandable: Array, candidates: Array) -> void:
	character_sheet.show_character(character, skill_tree, learnable, governable, commandable, candidates)


func hide_character() -> void:
	character_sheet.hide()


# --- Technologies (M6) --------------------------------------------------------------


func show_tech_tree(tree: Array, research: Dictionary, points_per_turn: int, faction_label: String, faction_color: Color) -> void:
	tech_panel.show_tree(tree, research, points_per_turn, faction_label, faction_color)


func hide_tech() -> void:
	tech_panel.hide()


func tech_panel_visible() -> bool:
	return tech_panel.visible


## Barre supérieure : recherche en cours (`get_research`, vide si aucune).
func set_research_progress(research: Dictionary, points_per_turn: int) -> void:
	if research.is_empty():
		research_label.text = "Aucune recherche"
		research_bar.max_value = 1
		research_bar.value = 0
		research_box.tooltip_text = "Aucune recherche en cours : %d points par tour perdus (clic : technologies)." % points_per_turn
		return
	var turns := int(research.get("turns_left", -1))
	research_label.text = str(research.get("name", ""))
	research_bar.max_value = maxi(1, int(research.get("cost", 1)))
	research_bar.value = int(research.get("progress", 0))
	research_box.tooltip_text = "Recherche : %s\n%d / %d points, +%d par tour%s" % [
		str(research.get("name", "")), int(research.get("progress", 0)), int(research.get("cost", 0)),
		int(research.get("points_per_turn", points_per_turn)),
		", %d tour(s) restant(s)" % turns if turns >= 0 else ""]


# --- HUD de campagne (F10b) ----------------------------------------------------------


## Capacité affichée du bandeau (`8/20`) tant que la simulation n'expose pas `max_units`.
const DEFAULT_ARMY_CAPACITY := 20
const HUD_MARGIN := 16.0
const LOG_WIDTH := 420.0

## Armée affichée dans le bandeau (`""` si aucune).
var current_army_id: String = ""
## Boîte d'actions contextuelles au-dessus du bandeau (assaut de siège, M8) : les contrôleurs
## ajoutent leurs contrôles dans `army_actions_box` ; visible quand un de ses enfants l'est.
var army_actions: PanelContainer
var army_actions_box: VBoxContainer
var _army_status: String = ""
var _unit_catalog: Dictionary = {}
var _layout_queued := false


## Accesseur stable (tutoriel F8) : le bandeau d'ost de l'armée sélectionnée (nœud `ArmyStrip`).
func selected_army_widget() -> ArmyStrip:
	return army_strip


## Accesseur stable (tutoriel F8) : la cloche de fin de saison (nœud `EndTurnCluster`).
func end_turn_control() -> EndTurnCluster:
	return end_turn_cluster


func _setup_hud() -> void:
	end_turn_cluster.end_turn_requested.connect(func() -> void: end_turn_pressed.emit())
	end_turn_cluster.alert_activated.connect(func(alert: Dictionary) -> void: alert_activated.emit(alert))
	general_seal.general_requested.connect(func(id: String) -> void: army_general_clicked.emit(id))
	general_seal.stance_selected.connect(func(stance: String) -> void:
		if current_army_id != "":
			stance_changed.emit(current_army_id, stance))
	army_strip.split_requested.connect(func(indices: PackedInt32Array) -> void:
		if current_army_id != "":
			army_split_requested.emit(current_army_id, Array(indices)))
	army_strip.garrison_requested.connect(func(indices: PackedInt32Array) -> void:
		if current_army_id != "":
			army_garrison_requested.emit(current_army_id, Array(indices)))
	news_letters.news_activated.connect(func(item: Dictionary) -> void: news_activated.emit(item))
	army_actions = PanelContainer.new()
	army_actions.name = "ArmyActions"
	army_actions.theme = event_log.theme
	army_actions.add_theme_stylebox_override("panel", HudStyle.panel_box(8))
	army_actions_box = VBoxContainer.new()
	army_actions_box.custom_minimum_size = Vector2(320, 0)
	army_actions.add_child(army_actions_box)
	add_child(army_actions)
	move_child(army_actions, army_strip.get_index())
	army_actions.hide()
	army_strip.hide()
	general_seal.hide()
	# Le journal est placé par `layout_hud` (au-dessus du sceau quand une armée est choisie).
	event_log.set_anchors_preset(Control.PRESET_TOP_LEFT)
	set_log_expanded(false)
	army_strip.minimum_size_changed.connect(queue_layout)
	news_letters.resized.connect(queue_layout)
	event_log.minimum_size_changed.connect(queue_layout)
	for panel in [province_panel, faction_panel, character_sheet]:
		panel.visibility_changed.connect(queue_layout)
	get_viewport().size_changed.connect(queue_layout)
	queue_layout()


## Replace le HUD au prochain cycle (plusieurs demandes → un seul placement).
func queue_layout() -> void:
	if _layout_queued or not is_inside_tree():
		return
	_layout_queued = true
	layout_hud.call_deferred()


## Placement du HUD (guide `docs/design/hud-campagne.md` § 3.1) : sceau en bas à gauche,
## cloche en bas à droite, bandeau centré entre les deux, journal au-dessus du sceau, lettres
## sous la barre à droite, panneau de province arrêté au-dessus de la cloche.
func layout_hud() -> void:
	_layout_queued = false
	var view := get_viewport().get_visible_rect().size
	var top: float = ($TopBar as Control).size.y + 8.0
	end_turn_cluster.size = end_turn_cluster.get_combined_minimum_size()
	end_turn_cluster.position = view - end_turn_cluster.size - Vector2(HUD_MARGIN, HUD_MARGIN) * 0.5
	general_seal.size = general_seal.get_combined_minimum_size()
	general_seal.position = Vector2(HUD_MARGIN, view.y - general_seal.size.y - HUD_MARGIN)
	var left := general_seal.position.x + general_seal.size.x + 16.0
	var right := end_turn_cluster.position.x + end_turn_cluster.fan_left_edge() - 10.0
	var gap := right - left
	army_strip.max_width = gap
	army_strip.size = Vector2.ZERO
	army_strip.position = Vector2(left + (gap - army_strip.size.x) * 0.5, view.y - army_strip.size.y - HUD_MARGIN * 0.75)
	# Étiquette de chemin : au-dessus du bandeau, sinon en bas au centre.
	var label_bottom := army_strip.position.y - 6.0 if army_strip.visible else view.y - HUD_MARGIN
	hover_label.position.y = label_bottom - hover_label.size.y
	var actions_visible := false
	for child in army_actions_box.get_children():
		if (child as Control).visible:
			actions_visible = true
	army_actions.visible = army_strip.visible and actions_visible
	army_actions.size = Vector2.ZERO
	army_actions.position = Vector2(army_strip.position.x, label_bottom - hover_label.size.y - army_actions.size.y - 6.0)
	# Journal : bas gauche, au-dessus du sceau quand il est affiché.
	var log_bottom := general_seal.position.y - 8.0 if general_seal.visible else view.y - HUD_MARGIN
	var log_height := event_log.get_combined_minimum_size().y
	event_log.size = Vector2(LOG_WIDTH, log_height)
	event_log.position = Vector2(HUD_MARGIN, maxf(top, log_bottom - log_height))
	# Minicarte (C1) puis lettres : haut droite. La minicarte reste visible sous les panneaux de
	# province et de colonie (placés à sa gauche, lot C7b) ; les grands panneaux de droite
	# (faction, fiche de personnage) la masquent, les lettres sont masquées par tout panneau.
	var docked_open := province_panel.visible
	for panel in docked_panels:
		docked_open = docked_open or panel.visible
	var wide_panel_open := faction_panel.visible or character_sheet.visible
	var letters_top := top
	# Bord droit (distance au bord de l'écran) des panneaux de province et de colonie.
	var dock_right := HUD_MARGIN
	if minimap != null:
		minimap.size = minimap.get_combined_minimum_size()
		minimap.position = Vector2(view.x - minimap.size.x - HUD_MARGIN, top)
		minimap.visible = not wide_panel_open
		letters_top = minimap.position.y + minimap.size.y + 10.0
		if minimap.visible:
			dock_right = view.x - minimap.position.x + 8.0
	docked_right_x = view.x - dock_right
	news_letters.position = Vector2(view.x - NewsLetters.LETTER_WIDTH - HUD_MARGIN, letters_top)
	news_letters.visible = not (docked_open or wide_panel_open)
	# Panneaux de province et de colonie : de la barre jusqu'au-dessus de la cloche, à gauche de
	# la minicarte.
	for panel: Control in [province_panel] + docked_panels:
		_dock_panel(panel, top, dock_right)


## Lot C7b : ancre `panel` (panneau de colonie) comme le panneau de province, à gauche de la
## minicarte ; replacé à chaque `layout_hud`.
func dock_right_panel(panel: Control) -> void:
	if docked_panels.has(panel):
		return
	docked_panels.append(panel)
	panel.visibility_changed.connect(queue_layout)
	queue_layout()


func _dock_panel(panel: Control, top: float, right: float) -> void:
	var width := maxf(panel.offset_right - panel.offset_left, panel.custom_minimum_size.x)
	panel.anchor_left = 1.0
	panel.anchor_right = 1.0
	panel.anchor_top = 0.0
	panel.anchor_bottom = 1.0
	# Contenu plus large que prévu : le panneau s'étend vers la gauche, jamais sur la minicarte.
	panel.grow_horizontal = Control.GROW_DIRECTION_BEGIN
	# Contenu plus haut que la place : le panneau déborde vers le bas (sur la cloche),
	# jamais vers le haut (sur la barre supérieure).
	panel.grow_vertical = Control.GROW_DIRECTION_END
	panel.offset_right = -right
	panel.offset_left = -right - width
	panel.offset_top = top
	panel.offset_bottom = -(end_turn_cluster.size.y + HUD_MARGIN * 0.5)


func _fit_hover_label() -> void:
	hover_label.reset_size()
	hover_label.size.x = maxf(hover_label.get_combined_minimum_size().x + 24.0, 400.0)
	hover_label.position.x = (get_viewport().get_visible_rect().size.x - hover_label.size.x) * 0.5
	queue_layout()


## Bouton « Codex » (H2) dans la barre du haut, après Technologies ; la touche K reste active.
func _add_codex_button() -> void:
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles == null or tech_button.get_parent().has_node("CodexButton"):
		return
	var button := Button.new()
	button.name = "CodexButton"
	button.tooltip_text = "Codex : l'histoire et le savoir du temps (K)"
	button.focus_mode = Control.FOCUS_NONE
	button.pressed.connect(func() -> void: bubbles.call("toggle_window"))
	tech_button.get_parent().add_child(button)
	tech_button.get_parent().move_child(button, tech_button.get_index() + 1)
	if IconLibrary.has_icon("hud_codex"):
		_decorate_button(button, "hud_codex", true)
	else:
		button.text = "Codex"
