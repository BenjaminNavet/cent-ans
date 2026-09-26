class_name MapUI
extends CanvasLayer

## Couche UI de la carte de campagne : barre supérieure (faction, trésor, solde, date, menu),
## HUD de campagne (F10b : bandeau d'ost `ArmyStrip` et sceau du chef `GeneralSeal` pour
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
# C5 : couche des routes commerciales.
signal trade_layer_toggle_requested

const MENU_SAVE := 0
const MENU_LOAD := 1
const MENU_MAIN := 3
const MENU_QUIT := 4
const MAX_LOG_LINES := 200
const TOAST_SECONDS := 3.5
## Q2 : un panneau ouvert plus de tant de ms après le bandeau le fait disparaître s'il le
## chevauche (le message d'avant ne masque plus le titre du panneau suivant).
const TOAST_GRACE_MS := 300

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
# C5 : commerce.
@onready var trade_button: Button = %TradeButton

var _log_lines: PackedStringArray = PackedStringArray()
var _toast_timer: SceneTreeTimer
var _toast_shown_at := 0
## Lot U1 (audit A3) : pile des panneaux (exclusivité, Échap, mise de côté des panneaux ancrés).
var panels := PanelStack.new()
## Province affichée par le panneau de province (une autre province = nouvelle sélection).
var _province_panel_id: String = ""


func _ready() -> void:
	# BP1 : mots du Codex cliquables (bulles imbriquées) dans le journal de campagne.
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles != null:
		bubbles.call("attach", log_text)
	menu_button.get_popup().id_pressed.connect(_on_menu_item)
	# « Son… » (Q2) : ajouté par `FlowController`, ouvre l'onglet Son des réglages.
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
	# --- C5 : routes commerciales ---
	trade_button.toggled.connect(func(_pressed: bool) -> void: trade_layer_toggle_requested.emit())
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
	_setup_panel_stack()  # U1
	_apply_access_args()  # U12 : captures


## Captures (lot U12) : `--access=colorblind,contrast,motion` active ces réglages sans les
## enregistrer dans le fichier du joueur.
func _apply_access_args() -> void:
	var settings := get_node_or_null("/root/Settings")
	if settings == null:
		return
	var keys := {"colorblind": Accessibility.KEY_COLORBLIND, "contrast": Accessibility.KEY_HIGH_CONTRAST, "motion": Accessibility.KEY_REDUCE_MOTION}
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--access="):
			for word in arg.trim_prefix("--access=").split(","):
				if keys.has(word):
					settings.call("set_value", keys[word], true, false)


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
	research_box.set_script(RichBox)  # B1 : recherche en infobulle riche (technologie liée au Codex)
	treasury_label.tooltip_text = RichTooltip.hud("hud_treasury")
	income_label.tooltip_text = RichTooltip.hud("hud_income")
	# UX2 (C9) : chaque bouton porte un libellé court quand la barre a la place, sinon son
	# icône seule (nom et touche dans l'infobulle) ; voir `fit_top_bar`.
	# U7 : chaque bouton porte la lettre de son raccourci (lue dans l'InputMap).
	_decorate_button(court_button, "hud_court")
	_add_keycap(court_button, "map_toggle_court")
	_register_top_label(court_button, "Cour")
	_decorate_button(tech_button, "hud_technologies")
	_add_keycap(tech_button, "map_toggle_tech")
	_register_top_label(tech_button, "Techniques")
	_add_codex_button()
	var objectives := _add_action_button("ObjectivesButton", "⚑", "Objectifs", "map_toggle_objectives",
		"[b]Objectifs[/b]\nObjectifs historiques de votre faction et score.", tech_button.get_index() + 2)
	_register_top_label(objectives, "Objectifs", "" if _apply_top_medallion(objectives, "hud_objectives") else "⚑")
	var agents := _add_action_button("AgentsButton", "✦", "Agents", "map_toggle_agents",
		"[b]Agents[/b]\nRegistre des espions, hérauts et prédicateurs.", tech_button.get_index() + 3)
	_register_top_label(agents, "Agents", "" if _apply_top_medallion(agents, "hud_agents") else "✦")
	if _apply_top_medallion(menu_button, "hud_menu"):
		menu_button.add_theme_font_size_override("font_size", TOP_LABEL_FONT)
	get_viewport().size_changed.connect(queue_fit_top_bar)
	($TopBar as Control).resized.connect(queue_fit_top_bar)
	var settings := get_node_or_null("/root/Settings")
	if settings != null:
		settings.changed.connect(func(key: String) -> void:
			if key == "input/layout":
				refresh_keycaps())
	# Boutons ajoutés par les contrôleurs (Diplomatie, Chronique) après ce _ready.
	bar.child_entered_tree.connect(func(node: Node) -> void: _decorate_late_button.call_deferred(node))


func _insert_icon_before(control: Control, icon_id: String) -> TextureRect:
	var rect: TextureRect = IconLibrary.make_rect(icon_id, TOP_ICON_SIZE)
	var parent := control.get_parent()
	parent.add_child(rect)
	parent.move_child(rect, control.get_index())
	return rect


## DA5 : boutons de la barre du haut portant un médaillon enluminé (id d'icône → médaillon).
const TOP_MEDALLIONS := {
	"hud_court": "court", "hud_technologies": "technologies", "hud_codex": "codex",
	"hud_diplomacy": "diplomacy", "hud_chronicle": "chronicle", "hud_objectives": "objectives",
	"hud_agents": "agents", "hud_menu": "menu",
}
## Diamètre du médaillon dans la barre (l'icône à l'encre reste le repli à `TOP_ICON_SIZE`).
const TOP_MEDALLION_SIZE := 26


## Médaillon enluminé sur un bouton de la barre (DA5) : le médaillon remplace le fond plat au
## repos (le cadre du thème ne revient qu'au survol) ; faux si l'image manque (repli : icône).
func _apply_top_medallion(button: Button, icon_id: String) -> bool:
	if not TOP_MEDALLIONS.has(icon_id):
		return false
	if not IconLibrary.decorate_medallion(button, str(TOP_MEDALLIONS[icon_id]), TOP_MEDALLION_SIZE):
		return false
	button.set_meta("medallion", true)
	button.add_theme_constant_override("h_separation", 5)
	_pad_for_keycap(button, true)
	return true


func _decorate_button(button: Button, icon_id: String, icon_only: bool = false) -> void:
	IconLibrary.decorate_button(button, icon_id, int(TOP_ICON_SIZE) + (6 if icon_only else 0))
	_apply_top_medallion(button, icon_id)
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
		_decorate_button(button, "hud_diplomacy")
		_add_keycap(button, "map_toggle_diplomacy")
		_register_top_label(button, "Diplomatie")
	elif button.text.begins_with("Chronique"):
		_decorate_button(button, "hud_chronicle")
		_register_top_label(button, "Chronique")


# --- Barre supérieure ------------------------------------------------------------


func set_faction(label: String, color: Color) -> void:
	faction_label.text = label
	faction_swatch.color = color


## `economy` : `get_faction_economy` (vide si indisponible). Affiche le solde **net** prévu
## (calculé par `core/`, `net_income`) ; infobulle : rubriques signées du budget et écart « par
## rapport à la saison passée » (lot U3). Sans économie, repli sur `income` (revenu brut).
func set_treasury(treasury: int, income: int, economy: Dictionary = {}) -> void:
	treasury_label.text = "Trésor : %s" % Money.amount(treasury)
	if economy.is_empty():
		income_label.text = "Revenu : %s" % Money.signed(income)
		income_label.tooltip_text = "Revenu brut du dernier tour."
		return
	var net := int(economy.get("net_income", 0))
	income_label.text = "Solde : %s / saison" % Money.signed(net)
	income_label.add_theme_color_override("font_color", Money.LOSS_COLOR if net < 0 else Money.INK_COLOR)
	income_label.tooltip_text = budget_tooltip(economy)
	treasury_label.tooltip_text = RichTooltip.hud("hud_treasury", treasury_tooltip(economy))


## Infobulle du solde : rubriques signées (prévu, et saison passée entre parenthèses), solde,
## puis l'écart par rapport à la saison passée et sa principale cause.
static func budget_tooltip(economy: Dictionary) -> String:
	var lines := PackedStringArray(["[b]Solde prévu pour la prochaine saison[/b]"])
	var has_past := economy.has("net_change")
	var biggest_key := ""
	var biggest := 0
	for line in economy.get("budget_lines", []):
		var key := str(line.get("key", ""))
		if key == "other":
			continue
		var projected := int(line.get("projected", 0))
		var text := "%s : [color=#%s]%s[/color]" % [BudgetTable.RUBRICS.get(key, key), Money.color_of(projected).to_html(false), Money.signed(projected)]
		if has_past and line.has("last"):
			text += " (saison passée %s)" % Money.signed(int(line["last"]))
		lines.append(text)
		var delta := int(line.get("delta", 0))
		if has_past and absi(delta) > absi(biggest):
			biggest = delta
			biggest_key = key
	var net := int(economy.get("net_income", 0))
	lines.append("[b]Solde : [color=#%s]%s[/color][/b]" % [Money.color_of(net).to_html(false), Money.signed(net)])
	if has_past:
		var change := int(economy.get("net_change", 0))
		var sentence := "%s par rapport à la saison passée" % Money.signed(change)
		if change != 0 and biggest_key != "" and biggest != 0:
			sentence += ", dont %s sur « %s »" % [Money.signed(biggest), str(BudgetTable.RUBRICS.get(biggest_key, biggest_key)).to_lower()]
		lines.append("[color=#%s]%s.[/color]" % [Money.color_of(change).to_html(false), sentence])
	else:
		lines.append("Premier tour : pas encore de saison passée.")
	lines.append("Détail : panneau de faction (clic sur le blason).")
	return "\n".join(lines)


## Infobulle du trésor : variation de la dernière saison, hors budget compris.
static func treasury_tooltip(economy: Dictionary) -> String:
	var history: Array = economy.get("budget_history", [])
	if history.is_empty():
		return "Aucune saison résolue pour l'instant."
	var last: Dictionary = history.back()
	var text := "Saison passée : %s au trésor" % Money.signed(int(last.get("change", 0)))
	var other := int(last.get("other", 0))
	if other != 0:
		text += " (dont %s hors budget : rançons, tributs, agents, chronique)" % Money.signed(other)
	return text + "."


static func _signed(value: int) -> String:
	return Money.signed(value)


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


## C5 : synchronise le bouton « Commerce » de la barre supérieure avec la couche.
func set_trade_mode(active: bool) -> void:
	# Sans signal : `toggled` redemanderait la bascule (boucle infinie).
	trade_button.set_pressed_no_signal(active)


## C5 : infobulle de la route commerciale survolée (texte vide = pas de route sous la souris,
## le survol de province reprend la main).
func set_hover_trade(text: String) -> void:
	if text == "":
		return
	hover_label.text = text
	hover_label.visible = true
	_fit_hover_label()


func show_toast(text: String, is_error: bool = false) -> void:
	toast.text = text
	toast.add_theme_color_override("font_color", Color(0.55, 0.12, 0.10) if is_error else Color(0.22, 0.14, 0.07))
	# Q1 : le bandeau passait sous les panneaux ancrés et le rapport de saison (message invisible).
	toast.move_to_front()
	toast.show()
	_toast_shown_at = Time.get_ticks_msec()
	_toast_timer = get_tree().create_timer(TOAST_SECONDS)
	var timer := _toast_timer
	timer.timeout.connect(func() -> void:
		if _toast_timer == timer:
			toast.hide())


## Q2 : masque le bandeau quand un panneau ouvert après lui le chevauche.
func _hide_toast_under_panels() -> void:
	if not toast.visible or Time.get_ticks_msec() - _toast_shown_at < TOAST_GRACE_MS:
		return
	var rect := toast.get_global_rect()
	for panel in panels.visible_panels():
		if panel != toast and panel.get_global_rect().intersects(rect):
			toast.hide()
			return


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


## Lot U5 : filtre d'intérêt des lettres et du bandeau (voisins, alliés, ennemis, grandes
## puissances), recalculé en fin de tour par `HudController.update_interest` ; nul = tout passe.
var news_interest: NewsInterest = null


## Vrai si la nouvelle mérite une lettre ou le bandeau du haut (le journal garde tout).
func keeps_news(event: Dictionary) -> bool:
	return news_interest == null or news_interest.keeps(event)


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
	var new_news: Array = []  # lettres du tour, poussées en un seul lot (une reconstruction, un son)
	for event in events:
		if not journal_keeps(event):
			continue
		var news := NewsLetters.news_from_event(event)  # F10b : lettre scellée (trace persistante)
		if not news.is_empty() and keeps_news(event):  # U5 : filtre d'intérêt
			if news_interest != null:
				news["interest"] = NewsInterest.interest_label(news_interest.event_interest(event))
			new_news.append(news)
		var kind: String = str(event.get("kind", ""))
		var text: String = journal_text(event)
		if text == "":
			continue
		text = CodexText.format(text, true)  # BP1 : liens du Codex
		var line: String
		if kind == "battle" or kind == "siege_started" or kind == "province_captured":
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
		elif kind == "trade":  # C5
			line = "[color=#4a3a10]⚓ %s[/color]" % text
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
	news_letters.push_news_batch(new_news)
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
	# U1 : une autre province choisie sur la carte referme les grands panneaux ; la même
	# province (rafraîchissement de fin de tour) reste de côté sous le panneau central.
	var id := str(province.get("id", ""))
	if id != _province_panel_id:
		panels.reveal(province_panel)
	_province_panel_id = id
	province_panel.show_province(province, state, recruitable, is_player_owner, label_of, city)


func hide_province() -> void:
	_province_panel_id = ""
	panels.forget_suspended(province_panel)
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


## Lot U10 (audit A3 § 4) : choix du général d'une armée sans chef, ouvert depuis le sceau
## « Sans chef ». `candidates` : `[{id, name, detail, reason}]` (`reason` non vide = grisé, avec
## le motif). Choisir émet `general_requested(personnage, armée)` ; « Toute la Cour… » émet
## `court_panel_requested`.
var general_picker: PanelContainer


func show_general_picker(army_id: String, title: String, candidates: Array) -> void:
	if general_picker == null:
		general_picker = PanelContainer.new()
		general_picker.name = "GeneralPicker"
		general_picker.theme = event_log.theme
		general_picker.add_theme_stylebox_override("panel", HudStyle.panel_box(10))
		add_child(general_picker)
		register_panel(general_picker, PanelStack.Kind.CENTRAL)
	for child in general_picker.get_children():
		general_picker.remove_child(child)
		child.queue_free()
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 4)
	general_picker.add_child(box)
	var header := HBoxContainer.new()
	box.add_child(header)
	var heading := HudStyle.label(title, HudStyle.FONT_TITLE + 1, HudStyle.RUBRIC)
	heading.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(heading)
	var close := Button.new()
	close.text = "×"
	close.tooltip_text = "Fermer (Échap)"
	close.pressed.connect(general_picker.hide)
	header.add_child(close)
	var any_free := false
	for candidate in candidates:
		var reason := str(candidate.get("reason", ""))
		var button := RichButton.new()  # B1 : infobulle riche auto-liée
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.text = "%s — %s" % [str(candidate.get("name", "?")), str(candidate.get("detail", ""))]
		if reason != "":
			button.text += " (%s)" % reason
			button.disabled = true
			button.tooltip_text = "Impossible : %s." % reason
		else:
			any_free = true
			var character_id := str(candidate.get("id", ""))
			button.pressed.connect(func() -> void:
				general_picker.hide()
				general_requested.emit(character_id, army_id))
		box.add_child(button)
	if not any_free:
		box.add_child(HudStyle.label("Aucun personnage disponible sur place : amenez-en un jusqu'à l'armée.", HudStyle.FONT_BODY, HudStyle.INK_SOFT))
	var court := Button.new()
	court.text = "Toute la Cour…"
	court.pressed.connect(func() -> void:
		general_picker.hide()
		court_panel_requested.emit())
	box.add_child(court)
	general_picker.show()
	general_picker.reset_size()
	var view := get_viewport().get_visible_rect().size
	general_picker.position = Vector2(HUD_MARGIN, maxf(60.0, view.y - general_seal.size.y - HUD_MARGIN - general_picker.size.y - 8.0))


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
	research_box.tooltip_text = "Recherche : [b]%s[/b]\n%d / %d points, +%d par tour%s" % [
		RichTooltip.entity_name(str(research.get("technology", "")), str(research.get("name", ""))),
		int(research.get("progress", 0)), int(research.get("cost", 0)),
		int(research.get("points_per_turn", points_per_turn)),
		", %s restant%s" % [FrText.count(turns, "tour"), FrText.s(turns)] if turns >= 0 else ""]


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


## Lot U5 (audit A3, T5) : bandeau « Tour des autres factions » affiché pendant la résolution de
## la fin de saison. `end_turn_gate` (posé par `FlowController`) dit si la fin de tour aura lieu
## tout de suite (pas de confirmation en attente) ; invalide = oui.
var end_turn_gate: Callable = Callable()
var turn_banner: PanelContainer
var _turn_banner_title: Label
var _turn_banner_detail: Label
var _turn_banner_tween: Tween
const TURN_BANNER_HOLD := 1.1
## Fin de tour demandée, pas encore émise (deux déclenchements rapprochés = une seule saison).
var _end_turn_pending := false


## Cloche ou Entrée : bandeau des autres factions, une image pour l'afficher, puis la fin de tour.
func request_end_turn() -> void:
	if _end_turn_pending:
		return
	if end_turn_gate.is_valid() and not bool(end_turn_gate.call()):
		end_turn_pressed.emit()
		return
	_end_turn_pending = true
	show_turn_banner()
	await get_tree().process_frame
	await get_tree().process_frame
	_end_turn_pending = false
	end_turn_pressed.emit()
	finish_turn_banner()


func _setup_turn_banner() -> void:
	turn_banner = PanelContainer.new()
	PanelStack.set_tier(turn_banner, PanelStack.Tier.BANNER)  # Q4 : au-dessus des panneaux, sous les modales
	turn_banner.name = "TurnBanner"
	turn_banner.theme = event_log.theme
	turn_banner.mouse_filter = Control.MOUSE_FILTER_IGNORE
	turn_banner.add_theme_stylebox_override("panel", HudStyle.illuminated_box(12))
	var column := VBoxContainer.new()
	column.alignment = BoxContainer.ALIGNMENT_CENTER
	column.mouse_filter = Control.MOUSE_FILTER_IGNORE
	turn_banner.add_child(column)
	_turn_banner_title = HudStyle.label("Tour des autres factions", 22, HudStyle.RUBRIC)
	_turn_banner_title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	column.add_child(_turn_banner_title)
	_turn_banner_detail = HudStyle.label("", HudStyle.FONT_BODY + 2, HudStyle.INK_SOFT)
	_turn_banner_detail.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	column.add_child(_turn_banner_detail)
	add_child(turn_banner)
	turn_banner.hide()


func show_turn_banner() -> void:
	if turn_banner == null:
		return
	if _turn_banner_tween != null:
		_turn_banner_tween.kill()
	_turn_banner_title.text = "Tour des autres factions"
	_turn_banner_detail.text = "Les princes d'Europe jouent leur saison…"
	turn_banner.modulate.a = 1.0
	queue_restack()  # Q4 : étage BANNER, au-dessus des panneaux
	turn_banner.show()
	_place_turn_banner()


## Après la résolution : « à vous de jouer », puis le bandeau s'efface (sans fondu si
## « Réduire les animations » est coché).
func finish_turn_banner() -> void:
	if turn_banner == null or not turn_banner.visible:
		return
	_turn_banner_title.text = date_label.text.get_slice(" — ", 0)
	_turn_banner_detail.text = "Les autres factions ont joué : à vous."
	_place_turn_banner()
	if _turn_banner_tween != null:
		_turn_banner_tween.kill()
	_turn_banner_tween = create_tween()
	_turn_banner_tween.tween_interval(TURN_BANNER_HOLD)
	if Accessibility.reduce_motion():
		_turn_banner_tween.tween_callback(turn_banner.hide)
	else:
		_turn_banner_tween.tween_property(turn_banner, "modulate:a", 0.0, 0.45)
		_turn_banner_tween.tween_callback(turn_banner.hide)


func _place_turn_banner() -> void:
	var view := get_viewport().get_visible_rect().size
	turn_banner.reset_size()
	turn_banner.size.x = maxf(turn_banner.get_combined_minimum_size().x, 380.0)
	turn_banner.position = Vector2((view.x - turn_banner.size.x) * 0.5, ($TopBar as Control).size.y + 48.0)


func _setup_hud() -> void:
	end_turn_cluster.end_turn_requested.connect(request_end_turn)
	_setup_turn_banner()
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
	end_turn_cluster.minimum_size_changed.connect(queue_layout)  # U5 : colonne de pastilles
	news_letters.resized.connect(queue_layout)
	event_log.minimum_size_changed.connect(queue_layout)
	for panel in [province_panel, faction_panel, character_sheet, court_panel]:
		panel.visibility_changed.connect(queue_layout)
	court_panel.resized.connect(queue_layout)  # C7 : onglet arbre (panneau élargi)
	get_viewport().size_changed.connect(queue_layout)
	queue_layout()


# --- Pile des panneaux (audit A3, lot U1) ------------------------------------------------


func _setup_panel_stack() -> void:
	panels.register(province_panel, PanelStack.Kind.DOCKED)
	panels.register(faction_panel, PanelStack.Kind.CENTRAL)
	panels.register(court_panel, PanelStack.Kind.CENTRAL)
	panels.register(tech_panel, PanelStack.Kind.CENTRAL)
	panels.register(character_sheet, PanelStack.Kind.COMPANION, [court_panel])
	panels.register(save_load_dialog, PanelStack.Kind.MODAL)
	for panel in docked_panels:
		panels.register(panel, PanelStack.Kind.DOCKED)
	# U11 : fenêtre commune « Codex » (Histoire / Règles), panneau central.
	codex_hub = CodexHub.new()
	add_child(codex_hub)
	register_panel(codex_hub, PanelStack.Kind.CENTRAL)
	for child in get_children():
		_auto_register(child)
	child_entered_tree.connect(_auto_register)
	# Q4 : ordre des enfants HUD < panneaux < bandeaux < modales < tutoriel (voir PanelStack).
	child_entered_tree.connect(func(_node: Node) -> void: queue_restack())
	panels.changed.connect(queue_restack)
	panels.changed.connect(queue_layout)
	panels.changed.connect(func() -> void: _hide_toast_under_panels.call_deferred())
	queue_restack()


var _restack_queued := false


## Q4 : trie les enfants de l'interface par étage (fin d'image, une fois).
func queue_restack() -> void:
	if _restack_queued:
		return
	_restack_queued = true
	(func() -> void:
		_restack_queued = false
		if is_inside_tree():
			panels.restack(self)).call_deferred()


## Enregistre un panneau de la carte : `kind` = `PanelStack.Kind` ; `companion_of` : panneaux
## centraux qu'il accompagne (fiche à côté de la Cour…).
func register_panel(panel: Control, kind: PanelStack.Kind, companion_of: Array = []) -> void:
	panels.register(panel, kind, companion_of)
	queue_restack()
	if not panel.resized.is_connected(queue_layout):
		panel.resized.connect(queue_layout)


## Lot U11 : fenêtre commune Codex / encyclopédie.
var codex_hub: CodexHub


## Panneaux ajoutés par les contrôleurs (diplomatie, chronique, agents, rançons, flow).
func _auto_register(node: Node) -> void:
	if node is Encyclopedia and codex_hub != null:
		codex_hub.adopt_encyclopedia.call_deferred(node)  # U11 : onglet « Règles »
		return
	if not (node is Control) or panels.is_registered(node):
		return
	if node is DiplomacyPanel or node is ChronicleWindow or node.name == &"AgentRegistry":
		register_panel(node, PanelStack.Kind.CENTRAL)
	elif node is RansomPanel:
		register_panel(node, PanelStack.Kind.COMPANION, [faction_panel])
	elif node is PauseMenu or node is SettingsMenu or node is SeasonReport:
		register_panel(node, PanelStack.Kind.MODAL)


## Échap ferme le panneau du dessus avant que la carte (désélection) ou le menu pause ne la
## reçoivent (`_shortcut_input` passe avant `_unhandled_input`).
func _shortcut_input(event: InputEvent) -> void:
	if get_tree().paused or not visible or event.is_echo():
		return
	# Lot U7 (partiel) : sauvegarde rapide F5, chargement rapide F9.
	if event is InputEventKey and event.pressed and not panels.has_modal_open():
		if event.is_action_pressed("quick_save"):
			save_requested.emit(QUICK_SAVE_NAME)
			get_viewport().set_input_as_handled()
			return
		if event.is_action_pressed("quick_load"):
			var path := SaveSlots.SAVES_DIR.path_join(QUICK_SAVE_NAME.validate_filename() + ".json")
			if FileAccess.file_exists(path):
				load_requested.emit(path)
			else:
				show_toast("Aucune sauvegarde rapide (F5 pour en faire une).", true)
			get_viewport().set_input_as_handled()
			return
	if not event.is_action_pressed("ui_cancel"):
		return
	if panels.close_top():
		get_viewport().set_input_as_handled()


## Nom de l'emplacement de la sauvegarde rapide (F5 / F9).
const QUICK_SAVE_NAME := "Sauvegarde rapide"


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
	# Actions d'armée (siège) : jamais sur le journal ni sur son bouton « Déplier » (audit U1).
	if army_actions.visible and event_log.visible and event_log.get_rect().intersects(army_actions.get_rect()):
		army_actions.position.x = event_log.position.x + event_log.size.x + 8.0
	# C7 : la fiche se range à droite de la Cour (repli en une colonne si la place manque) ;
	# l'arbre de la Cour se rétrécit pour lui laisser au moins sa largeur repliée.
	if court_panel.visible and character_sheet.visible:
		court_panel.set_max_right(view.x - CharacterSheet.COMPACT_WIDTH - 2.0 * CharacterSheet.SCREEN_MARGIN)
	else:
		court_panel.set_max_right(INF)
	# Bord droit réel (la liste peut élargir le panneau au-delà de ses marges).
	character_sheet.fit_beside(court_panel.get_global_rect().end.x if court_panel.visible else 0.0, view.x)
	# Minicarte (C1) puis lettres : haut droite. La minicarte reste visible sous les panneaux de
	# province et de colonie (placés à sa gauche, lot C7b) ; les grands panneaux de droite
	# (faction, fiche de personnage) la masquent, les lettres sont masquées par tout panneau.
	var docked_open := province_panel.visible
	for panel in docked_panels:
		docked_open = docked_open or panel.visible
	# U1 : panneaux centraux gardés à l'écran (bouton × visible), puis la minicarte se masque
	# dès qu'un grand panneau la recouvrirait (elle ne passe plus jamais par-dessus).
	var wide_panel_open := false
	for panel in panels.visible_panels():
		var kind := panels.kind_of(panel)
		if kind == PanelStack.Kind.CENTRAL or kind == PanelStack.Kind.COMPANION:
			_keep_on_screen(panel, top, view)
			wide_panel_open = true
	var letters_top := top
	# Bord droit (distance au bord de l'écran) des panneaux de province et de colonie.
	var dock_right := HUD_MARGIN
	if minimap != null:
		minimap.size = minimap.get_combined_minimum_size()
		minimap.position = Vector2(view.x - minimap.size.x - HUD_MARGIN, top)
		minimap.visible = not _covers(minimap.get_global_rect())
		letters_top = minimap.position.y + minimap.size.y + 10.0
		if minimap.visible:
			dock_right = view.x - minimap.position.x + 8.0
	docked_right_x = view.x - dock_right
	news_letters.position = Vector2(view.x - NewsLetters.LETTER_WIDTH - HUD_MARGIN, letters_top)
	# U5 : les lettres s'arrêtent au-dessus des pastilles d'alerte de la cloche.
	news_letters.fit_height(end_turn_cluster.position.y + end_turn_cluster.stack_top() - 8.0 - letters_top)
	news_letters.visible = not (docked_open or wide_panel_open)
	# Panneaux de province et de colonie : de la barre jusqu'au-dessus de la cloche, à gauche de
	# la minicarte.
	for panel: Control in [province_panel] + docked_panels:
		_dock_panel(panel, top, dock_right)


## U1 : vrai si un panneau central ou compagnon ouvert recouvre `rect` (coordonnées écran).
func _covers(rect: Rect2) -> bool:
	for panel in panels.visible_panels():
		var kind := panels.kind_of(panel)
		if (kind == PanelStack.Kind.CENTRAL or kind == PanelStack.Kind.COMPANION) and panel.get_global_rect().intersects(rect):
			return true
	return false


## U1 : le coin haut droit (bouton ×) d'un panneau reste sous la barre du haut et dans l'écran.
func _keep_on_screen(panel: Control, top: float, view: Vector2) -> void:
	if panel.get_parent() != self:
		return
	# Trop grand pour l'écran : rétréci (dans la limite de sa taille minimale).
	var room := Vector2(view.x - 8.0, view.y - top - 4.0)
	if panel.size.x > room.x or panel.size.y > room.y:
		panel.size = Vector2(minf(panel.size.x, room.x), minf(panel.size.y, room.y))
	var rect := panel.get_global_rect()
	var shift := Vector2.ZERO
	if rect.end.x > view.x - 4.0:
		shift.x = view.x - 4.0 - rect.end.x
	if rect.position.x + shift.x < 4.0:
		shift.x = 4.0 - rect.position.x
	if rect.position.y < top:
		shift.y = top - rect.position.y
	if shift != Vector2.ZERO:
		panel.position += shift


## Lot C7b : ancre `panel` (panneau de colonie) comme le panneau de province, à gauche de la
## minicarte ; replacé à chaque `layout_hud`.
func dock_right_panel(panel: Control) -> void:
	if docked_panels.has(panel):
		return
	docked_panels.append(panel)
	panels.register(panel, PanelStack.Kind.DOCKED)
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
	panel.offset_bottom = -(end_turn_cluster.bell_height() + HUD_MARGIN * 0.5)


func _fit_hover_label() -> void:
	hover_label.reset_size()
	hover_label.size.x = maxf(hover_label.get_combined_minimum_size().x + 24.0, 400.0)
	hover_label.position.x = (get_viewport().get_visible_rect().size.x - hover_label.size.x) * 0.5
	queue_layout()


## Bouton « Codex » (H2) dans la barre du haut, après Technologies ; lot U7 : icône, libellé
## « Codex » et lettre de raccourci (K), bien visible.
func _add_codex_button() -> void:
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles == null or tech_button.get_parent().has_node("CodexButton"):
		return
	var button := Button.new()
	button.name = "CodexButton"
	button.focus_mode = Control.FOCUS_NONE
	button.pressed.connect(func() -> void: bubbles.call("toggle_window"))
	tech_button.get_parent().add_child(button)
	tech_button.get_parent().move_child(button, tech_button.get_index() + 1)
	if IconLibrary.has_icon("hud_codex"):
		_decorate_button(button, "hud_codex")
	button.text = "Codex"
	button.add_theme_font_size_override("font_size", 17)
	button.tooltip_text = "[b]Codex[/b]\nL'histoire et le savoir du temps, et les règles du jeu (onglets Histoire et Règles)."
	_add_keycap(button, "codex_open")
	_register_top_label(button, "Codex")


# --- Raccourcis sur les boutons (lot U7) ------------------------------------------------

## [cartouche, action, bouton] des boutons de la barre.
var _keycaps: Array = []


## Cartouche de la touche d'`action` dans le coin bas droit de `button`, et rappel dans
## l'infobulle (« Cour (C) »).
## Lot MF1 : pose la touche d'une action sur un bouton ajouté par un contrôleur.
func add_keycap(button: Button, action: String) -> void:
	_add_keycap(button, action)


func _add_keycap(button: Button, action: String) -> void:
	if button == null or button.has_node("Keycap"):
		return
	var cap := Label.new()
	cap.name = "Keycap"
	cap.mouse_filter = Control.MOUSE_FILTER_IGNORE
	cap.add_theme_font_size_override("font_size", 11)
	cap.add_theme_color_override("font_color", HudStyle.INK)
	var box := HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.INK_SOFT)
	box.content_margin_left = 3
	box.content_margin_right = 3
	box.content_margin_top = 0
	box.content_margin_bottom = 0
	cap.add_theme_stylebox_override("normal", box)
	button.add_child(cap)
	_keycaps.append([cap, action, button, button.tooltip_text])
	button.resized.connect(func() -> void: _place_keycap(cap, button))
	_update_keycap(_keycaps.back())


func _update_keycap(entry: Array) -> void:
	var cap: Label = entry[0]
	var key := ShortcutSheet.first_key(str(entry[1]))
	cap.text = key
	cap.visible = key != ""
	var button: Button = entry[2]
	var tooltip := str(entry[3])
	button.tooltip_text = tooltip + ("\nRaccourci : %s" % key if key != "" else "")
	_place_keycap(cap, button)


func _place_keycap(cap: Label, button: Button) -> void:
	cap.size = cap.get_combined_minimum_size()
	cap.position = button.size - cap.size + Vector2(2, 2)


## Libellés des touches recalculés (réglage « Disposition du clavier »).
func refresh_keycaps() -> void:
	for entry in _keycaps:
		if is_instance_valid(entry[0]):
			_update_keycap(entry)
	queue_fit_top_bar()  # UX2 : marge du cartouche selon la nouvelle lettre


## Bouton de la barre qui déclenche une action de l'InputMap (même chemin que le clavier).
func _add_action_button(node_name: String, glyph: String, text: String, action: String, tooltip: String, index: int) -> Button:
	var bar := tech_button.get_parent()
	if bar.has_node(node_name):
		return bar.get_node(node_name)
	var button := Button.new()
	button.name = node_name
	button.focus_mode = Control.FOCUS_NONE
	button.text = glyph if text == "" else "%s %s" % [glyph, text]
	button.add_theme_font_size_override("font_size", 17)
	button.custom_minimum_size = Vector2(TOP_ICON_SIZE + 22.0, 0)
	button.set_script(RichButton)
	button.tooltip_text = tooltip
	button.pressed.connect(func() -> void: press_action(action))
	bar.add_child(button)
	bar.move_child(button, mini(index, bar.get_child_count() - 1))
	_add_keycap(button, action)
	return button


## Simule l'action `action` (appui puis relâche), comme si la touche avait été frappée.
func press_action(action: String) -> void:
	for pressed in [true, false]:
		var event := InputEventAction.new()
		event.action = action
		event.pressed = pressed
		Input.parse_input_event(event)


# --- UX2 : libellés de la barre du haut (audit A3, C9) ------------------------------------

## Taille du texte des boutons libellés de la barre.
const TOP_LABEL_FONT := 15
## Ordre de repli en icône seule quand la place manque (le premier se replie d'abord).
const TOP_COLLAPSE_ORDER := ["Agents", "Objectifs", "Codex", "Techniques", "Cour", "Diplomatie", "Chronique"]
## Marge laissée à droite de la barre (bord, respiration).
const TOP_BAR_SLACK := 4.0

## Boutons libellés : `{button, label, glyph, labelled}`.
var _top_labels: Array[Dictionary] = []
var _fit_queued := false


## Inscrit `button` dans la barre adaptative : `label` quand la place le permet, sinon l'icône
## (ou `glyph` pour les boutons sans icône). Nombre en attente : méta `count` (Chronique).
func _register_top_label(button: Button, label: String, glyph: String = "") -> void:
	if button == null:
		return
	for entry in _top_labels:
		if entry["button"] == button:
			return
	button.add_theme_font_size_override("font_size", TOP_LABEL_FONT)
	button.set_meta("top_label", label)
	var entry := {"button": button, "label": label, "glyph": glyph, "labelled": true}
	_top_labels.append(entry)
	if glyph == "" and button.icon == null:
		entry["glyph"] = label.left(1)
	_apply_top_label(entry, true)
	queue_fit_top_bar()


func _apply_top_label(entry: Dictionary, labelled: bool) -> void:
	var button: Button = entry["button"]
	if not is_instance_valid(button):
		return
	entry["labelled"] = labelled
	var count := int(button.get_meta("count", 0))
	var glyph := str(entry["glyph"])
	var parts := PackedStringArray()
	if glyph != "":
		parts.append(glyph)
	if labelled:
		parts.append(str(entry["label"]))
	var text := " ".join(parts)
	if count > 0:
		text = ("%s (%d)" % [text, count]) if labelled else ("%s %d" % [text, count]).strip_edges()
	button.text = text
	_pad_for_keycap(button, labelled)
	if button.has_meta("tooltip"):  # infobulle d'état fournie par le propriétaire (Chronique)
		button.tooltip_text = str(button.get_meta("tooltip"))


## États du bouton dont la marge droite s'élargit pour le cartouche de touche.
const KEYCAP_PAD_STATES := ["normal", "hover", "pressed", "hover_pressed", "disabled", "focus"]
## Écart entre la fin du libellé et le cartouche.
const KEYCAP_GAP := 4.0


## Bouton libellé portant un cartouche (coin bas droit) : marge droite du style élargie de la
## largeur du cartouche + `KEYCAP_GAP`, pour que le cartouche ne morde plus la dernière lettre.
## La largeur minimale du bouton l'inclut, donc `fit_top_bar` en tient compte. Icône seule :
## styles du thème (le cartouche se loge dans le coin, hors de l'icône).
func _pad_for_keycap(button: Button, labelled: bool) -> void:
	var cap := button.get_node_or_null("Keycap") as Label
	for state: String in KEYCAP_PAD_STATES:
		if button.has_theme_stylebox_override(state):
			button.remove_theme_stylebox_override(state)
	var medallion := bool(button.get_meta("medallion", false))
	var padded_caps := cap != null and cap.visible and labelled
	if not padded_caps and not medallion:
		return
	var cap_width := cap.get_combined_minimum_size().x + KEYCAP_GAP if padded_caps else 0.0
	for state: String in KEYCAP_PAD_STATES:
		var base := button.get_theme_stylebox(state)
		if base == null:
			continue
		var padded: StyleBox = base.duplicate() as StyleBox
		# DA5 : au repos, le médaillon se pose sur le bandeau sans cadre plat.
		if medallion and state in ["normal", "disabled", "focus"]:
			padded = StyleBoxEmpty.new()
			for side in [SIDE_LEFT, SIDE_TOP, SIDE_RIGHT, SIDE_BOTTOM]:
				padded.set_content_margin(side, maxf(base.get_margin(side), 0.0))
		padded.content_margin_right = maxf(base.get_margin(SIDE_RIGHT), 0.0) + cap_width
		button.add_theme_stylebox_override(state, padded)


## Réapplique le libellé de `button` (après un changement de méta `count` ou `tooltip`).
func refresh_top_button(button: Button) -> void:
	for entry in _top_labels:
		if entry["button"] == button:
			_apply_top_label(entry, bool(entry["labelled"]))
			queue_fit_top_bar()
			return


## Vrai si le bouton `label` (« Cour », « Techniques »…) porte son libellé en ce moment.
func top_button_labelled(label: String) -> bool:
	for entry in _top_labels:
		if str(entry["label"]) == label:
			return bool(entry["labelled"])
	return false


func queue_fit_top_bar() -> void:
	if _fit_queued or not is_inside_tree():
		return
	_fit_queued = true
	fit_top_bar.call_deferred()


## Libellés partout si la barre a la place, sinon repli en icône seule dans l'ordre de
## `TOP_COLLAPSE_ORDER` jusqu'à ce que la barre tienne dans la largeur de l'écran.
func fit_top_bar() -> void:
	_fit_queued = false
	# Largeur de l'écran (la barre, ancrée, s'élargit au-delà quand son contenu déborde).
	var available := get_viewport().get_visible_rect().size.x
	for entry in _top_labels:
		_apply_top_label(entry, true)
	for label in TOP_COLLAPSE_ORDER:
		if _top_bar_width() <= available - TOP_BAR_SLACK:
			break
		for entry in _top_labels:
			if str(entry["label"]) == label:
				_apply_top_label(entry, false)


## Largeur minimale de la barre (contenu et marges), calculée sur les enfants visibles.
func _top_bar_width() -> float:
	var top_bar := $TopBar as PanelContainer
	var bar := tech_button.get_parent() as HBoxContainer
	var width := 0.0
	var count := 0
	for child in bar.get_children():
		var control := child as Control
		if control == null or not control.visible or control.top_level:
			continue
		width += control.get_combined_minimum_size().x
		count += 1
	width += float(maxi(count - 1, 0) * bar.get_theme_constant("separation"))
	var style := top_bar.get_theme_stylebox("panel")
	if style != null:
		width += style.get_margin(SIDE_LEFT) + style.get_margin(SIDE_RIGHT)
	return width
