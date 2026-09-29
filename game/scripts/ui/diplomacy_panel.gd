class_name DiplomacyPanel
extends PanelContainer

## Écran de diplomatie plein écran (lot DP1, ADR 0025), en registre de manuscrit enluminé :
## - à gauche, les factions (blason, souverain, relation, attitude ; raisons en infobulle) ;
## - au centre, la carte diplomatique (provinces teintées selon notre position diplomatique,
##   lot DP2 : allié, accord, neutre, tension, guerre, vassal ; un clic choisit la faction qui
##   tient la province) ;
## - à droite, la fiche de la faction choisie et trois onglets : « Négociation » (clauses
##   communes, colonnes « Vous offrez » / « Vous demandez », barre d'acceptation en direct,
##   leur raisonnement ligne à ligne et la contre-offre quand un seul point bloque (DP2),
##   « Que faudrait-il ? », actions unilatérales), « Guerre » (score, fatigue, buts de guerre),
##   « Traités » (historique).
## Aucune règle ici : tout vient de `CampaignSim` (`get_diplomacy`, `treaty_options`,
## `evaluate_treaty`, `counter_treaty`, `get_war_summary`, `get_treaty_history`). Le panneau
## est enregistré comme panneau central de la pile d'UI2 (`map_ui.gd::_auto_register`).

signal order_requested(order: Dictionary, success_text: String)
signal offer_answered(offer_id: int, accept: bool)
## FE6 : verdict sur une guerre privée entre deux vassaux (« impose_peace », « take_side », « let_be »).
signal arbitration_requested(offer_id: int, verdict: String, side: String)
signal closed

const STATUS_LABELS := {
	"war": "En guerre", "truce": "Trêve", "peace": "Paix", "alliance": "Alliance",
	"vassal": "Notre vassal", "suzerain": "Notre suzerain",
}
const STATUS_COLORS := {
	"war": Color(0.62, 0.12, 0.10), "truce": Color(0.66, 0.50, 0.08), "peace": Color(0.35, 0.33, 0.30),
	"alliance": Color(0.15, 0.32, 0.62), "vassal": Color(0.42, 0.20, 0.55), "suzerain": Color(0.42, 0.20, 0.55),
}
## Teintes de la carte diplomatique (plus saturées que la carte 3D : fond clair de la minicarte).
const MAP_COLORS := {
	"self": Color(0.86, 0.68, 0.18), "war": Color(0.78, 0.10, 0.08), "truce": Color(0.95, 0.55, 0.15),
	"alliance": Color(0.18, 0.40, 0.85), "vassal": Color(0.55, 0.25, 0.72), "suzerain": Color(0.55, 0.25, 0.72),
}
const FRIENDLY := Color(0.30, 0.62, 0.30)
const HOSTILE := Color(0.72, 0.36, 0.22)
## Taille plancher de la carte des relations (elle grandit ensuite jusqu'à remplir sa colonne).
const MAP_MIN_WIDTH := 200.0
const MAP_MIN_HEIGHT := 150.0
const NEUTRAL := Color(0.62, 0.60, 0.55)
const GIFT_AMOUNT := 1000
const DONATION_AMOUNT := 1000
const GOLD_STEPS := [500, 1000, 2500, 5000, 10000, 20000]
const TRIBUTE_STEPS := [100, 250, 500, 1000]
const TRIBUTE_SEASONS := 8
const TRUCE_TURNS := 8
const TAB_NEGOTIATION := 0
const TAB_WAR := 1
const TAB_HISTORY := 2
const FILTERS := ["Toutes", "En guerre", "Alliés et vassaux", "En paix"]

var sim: Object = null
var player_faction: String = ""
var province_name_of: Callable = Callable()
## Carte (facultative) : sans elle, la colonne centrale affiche une légende seule.
var map_data: MapData = null

var _entries: Array = []
var _selected: String = ""
var _filter := 0
var _tab := TAB_NEGOTIATION
## Brouillon du traité en cours (dictionnaires au format de `negotiation::Article`).
var _articles: Array = []
var _draft_for: String = ""
var _options: Dictionary = {}
var _verdict: Dictionary = {}

var _religion_label: Label
var _offers_box: VBoxContainer
var _list: VBoxContainer
var _minimap: CampaignMinimap
var _map_hint: Label
## Lot DZ : la carte montre les relations de la faction choisie (vrai) ou les nôtres.
var _map_their_view := true
var _map_view_toggle: CheckButton
var _map_caption: Label
var _head: VBoxContainer
var _tab_buttons: Array[Button] = []
var _pages: Array[Control] = []
## Q6 : boutons du traité (« Que faudrait-il ? », « Effacer », « Proposer le traité »), posés
## sous les pages et non dans la page défilante : toujours visibles sur l'onglet Négociation.
var _treaty_buttons: HBoxContainer
## Q6 : taille visée par `_fit_to_viewport` (le panneau ne doit pas la dépasser).
var _target_size := Vector2.ZERO
var _fit_queued := false
var _fitting := false
var _clauses: HFlowContainer
var _offer_list: VBoxContainer
var _demand_list: VBoxContainer
var _clause_menu: MenuButton
var _offer_menu: MenuButton
var _demand_menu: MenuButton
var _chance_bar: ProgressBar
var _chance_label: Label
var _reasons: RichTextLabel
## Lot DP2 : contre-offre proposée quand un seul point bloque.
var _counter_box: HBoxContainer
var _counter_label: Label
var _counter_articles: Array = []
var _explanation: Dictionary = {}
var _actions: HFlowContainer
var _war_page: VBoxContainer
var _history_page: VBoxContainer
var _unilateral_hint: RichTextLabel


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	add_theme_stylebox_override("panel", _page_box())
	set_anchors_preset(Control.PRESET_TOP_LEFT)
	visibility_changed.connect(_fit_to_viewport)
	get_viewport().size_changed.connect(_fit_to_viewport)
	var root := VBoxContainer.new()
	root.add_theme_constant_override("separation", 6)
	add_child(root)
	root.add_child(_build_header())
	_offers_box = VBoxContainer.new()
	root.add_child(_offers_box)
	root.add_child(_rule())
	var body := HBoxContainer.new()
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	body.add_theme_constant_override("separation", 14)
	root.add_child(body)
	# Q6 : un contenu qui grandit après l'ajustement (avis reçus, fiche) faisait déborder le
	# panneau sous l'écran, la carte gardant sa taille : on la réduit de l'excédent.
	resized.connect(_queue_fit_minimap)
	body.add_child(_build_faction_column())
	body.add_child(_build_map_column())
	body.add_child(_build_detail_column())
	# PO phase 2 (P2b, ADR 0097) : tailles (`UiType`) et ouverture/fermeture (`UiMotion`). P2g :
	# sur la carte, `map_ui` le réclame dans la zone `MODAL` de `UiLayout` (`claim_modal_panel`).
	_fit_to_viewport()


## Plein écran sous la barre du haut (zone `TOP_BAR` de `UiLayout`). P2g : position globale (le
## parent peut être la zone `MODAL`, décalée de l'écran) ; la carte repart de sa taille plancher
## puis se réajuste au cadre (`_fit_minimap` sur `resized`) : sinon sa taille minimale, fixée
## par un cadre plus grand, empêchait le panneau de tenir dans l'écran (1280×720, 640 px).
func _fit_to_viewport() -> void:
	if not is_inside_tree():
		return
	var view := get_viewport_rect().size
	var top: float = UiZones.rect(UiZones.Zone.TOP_BAR).end.y
	global_position = Vector2(6, top)
	_target_size = Vector2(view.x - 12.0, view.y - top - 4.0)
	size = _target_size
	_refit_minimap_later()


## P2g : la carte repart de sa taille plancher, puis se réajuste une image plus tard, une fois les
## conteneurs recalculés (lue dans la même image, la taille du cadre serait encore l'ancienne).
func _refit_minimap_later() -> void:
	if _minimap == null or not is_inside_tree():
		return
	var map_view := _minimap.find_child("MapView", true, false) as Control
	if map_view != null:
		map_view.custom_minimum_size = Vector2(MAP_MIN_WIDTH, MAP_MIN_HEIGHT)
	if not get_tree().process_frame.is_connected(_fit_minimap):
		get_tree().process_frame.connect(_fit_minimap, CONNECT_ONE_SHOT)


# ----- construction ------------------------------------------------------------------------

func _page_box() -> StyleBoxFlat:
	var box := HudStyle.card_box(HudStyle.PARCHMENT, HudStyle.INK, 2)
	box.content_margin_left = 18
	box.content_margin_right = 18
	box.content_margin_top = 12
	box.content_margin_bottom = 12
	box.shadow_color = HudStyle.SHADOW
	box.shadow_size = 8
	return box


func _rule() -> Control:
	var rule := ColorRect.new()
	rule.color = HudStyle.GOLD
	rule.custom_minimum_size = Vector2(0, 2)
	rule.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return rule


func _section(text: String) -> Label:
	var label := _label(text, UiType.HEADING, HudStyle.RUBRIC)
	label.add_theme_constant_override("outline_size", 0)
	return label


## PO phase 2 (P2b, ADR 0097) : remplace les anciennes tailles ad hoc (`HudStyle.FONT_SMALL` /
## `FONT_BODY` / `FONT_TITLE` et quelques valeurs littérales) par une variation `UiType` — les
## quatre tailles de la bible DA § 12.2, jamais moins de `Caption`. `HudStyle.label` reste le
## constructeur (police, couleur), `UiType.apply` pose ensuite la taille et la variation de type
## du thème.
static func _label(text: String, variation: String, color: Color = HudStyle.INK) -> Label:
	var node := HudStyle.label(text, UiType.size(variation), color)
	UiType.apply(node, variation)
	return node


func _build_header() -> Control:
	var header := HBoxContainer.new()
	header.add_theme_constant_override("separation", 12)
	var titles := VBoxContainer.new()
	titles.add_theme_constant_override("separation", 0)
	titles.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	# CV3-0 (#10) : lettrine du kit partagé (UI1, `Lettrine.attach`) au lieu de l'ancien
	# `DropCap` local, qui figeait le titre affiché à "iplomatie" (le "D" retiré à la main pour
	# lui faire de la place) au lieu de réserver la place comme le fait le kit.
	var title := HudStyle.label("Diplomatie", UiType.size(UiType.TITLE), HudStyle.INK)
	UiType.apply(title, UiType.TITLE)
	Lettrine.attach(title)
	titles.add_child(title)
	_religion_label = _label("", UiType.BODY, HudStyle.INK_SOFT)
	titles.add_child(_religion_label)
	header.add_child(titles)
	var donate := Button.new()
	donate.text = "Don à l'Église (%s)" % Money.amount(DONATION_AMOUNT)
	donate.tooltip_text = RuleValues.format("Augmente la faveur pontificale (+1 par {rule.donation_livres_per_favor} livres).")
	donate.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	donate.pressed.connect(func() -> void:
		order_requested.emit({"type": "donate_to_church", "amount": DONATION_AMOUNT}, "Don versé à l'Église."))
	header.add_child(donate)
	var close := Button.new()
	close.text = "×"
	RichTooltip.attach_plain(close, "close_escape")
	close.custom_minimum_size = Vector2(36, 36)
	close.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	close.pressed.connect(func() -> void:
		UiMotion.fade_out(self)  # PO phase 2 (P2b) : fermeture animée (`UiMotion`)
		closed.emit())
	header.add_child(close)
	return header


func _build_faction_column() -> Control:
	var column := VBoxContainer.new()
	column.custom_minimum_size = Vector2(290, 0)
	column.add_theme_constant_override("separation", 6)
	column.add_child(_section("Les puissances"))
	var filter := OptionButton.new()
	for label in FILTERS:
		filter.add_item(label)
	filter.selected = _filter
	filter.item_selected.connect(func(index: int) -> void:
		_filter = index
		_render_list())
	column.add_child(filter)
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_list = VBoxContainer.new()
	_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_list.add_theme_constant_override("separation", 3)
	scroll.add_child(_list)
	column.add_child(scroll)
	return column


func _build_map_column() -> Control:
	var column := VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_theme_constant_override("separation", 6)
	column.add_child(_section("Carte des relations"))
	var view_row := HBoxContainer.new()  # DZ
	view_row.add_theme_constant_override("separation", 8)
	_map_caption = _label("", UiType.BODY, HudStyle.INK)
	_map_caption.name = "MapViewCaption"
	_map_caption.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	view_row.add_child(_map_caption)
	_map_view_toggle = CheckButton.new()
	_map_view_toggle.name = "MapViewToggle"
	_map_view_toggle.text = "Vue de la faction choisie"
	_map_view_toggle.button_pressed = _map_their_view
	_map_view_toggle.focus_mode = Control.FOCUS_NONE
	RichTooltip.attach_plain(_map_view_toggle, "diplomacy_map_view")
	_map_view_toggle.toggled.connect(func(on: bool) -> void:
		_map_their_view = on
		_render_map())
	view_row.add_child(_map_view_toggle)
	column.add_child(view_row)
	var holder := CenterContainer.new()
	holder.name = "MapHolder"
	holder.size_flags_vertical = Control.SIZE_EXPAND_FILL
	holder.resized.connect(_fit_minimap)
	column.add_child(holder)
	column.add_child(DiplomaticStances.legend(UiType.CAPTION))  # DP2
	_map_hint = _label("Cliquez une province pour traiter avec son seigneur. Les terres voilées sont hors de vue de vos agents et de vos armées.", UiType.CAPTION, HudStyle.INK_FADED)
	_map_hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(_map_hint)
	return column


func _legend() -> Control:
	var flow := HFlowContainer.new()
	flow.add_theme_constant_override("h_separation", 12)
	for entry in [["self", "Nous"], ["war", "Guerre"], ["truce", "Trêve"], ["alliance", "Alliés"], ["vassal", "Vassaux"], ["friendly", "Bien disposés"], ["hostile", "Hostiles"]]:
		var key: String = entry[0]
		var chip := HBoxContainer.new()
		chip.add_theme_constant_override("separation", 4)
		var swatch := ColorRect.new()
		swatch.custom_minimum_size = Vector2(14, 14)
		swatch.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		swatch.color = MAP_COLORS.get(key, FRIENDLY if key == "friendly" else HOSTILE)
		chip.add_child(swatch)
		var symbol := Accessibility.relation_symbol(key) if Accessibility.colorblind() else ""
		chip.add_child(_label(("%s %s" % [symbol, entry[1]]).strip_edges(), UiType.CAPTION, HudStyle.INK))
		flow.add_child(chip)
	return flow


func _build_detail_column() -> Control:
	var column := VBoxContainer.new()
	column.custom_minimum_size = Vector2(500, 0)
	column.add_theme_constant_override("separation", 6)
	_head = VBoxContainer.new()
	_head.add_theme_constant_override("separation", 2)
	column.add_child(_head)
	var tabs := HBoxContainer.new()
	tabs.add_theme_constant_override("separation", 4)
	for index in 3:
		var button := Button.new()
		button.text = ["Négociation", "Guerre", "Traités"][index]
		button.toggle_mode = true
		button.focus_mode = Control.FOCUS_NONE
		button.pressed.connect(func() -> void: _show_tab(index))
		tabs.add_child(button)
		_tab_buttons.append(button)
	column.add_child(tabs)
	column.add_child(_rule())
	var stack := Control.new()
	stack.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(stack)
	var negotiation := _scroll_page(_build_negotiation())
	column.add_child(_treaty_buttons)
	var war := VBoxContainer.new()
	war.add_theme_constant_override("separation", 6)
	_war_page = war
	var history := VBoxContainer.new()
	history.add_theme_constant_override("separation", 6)
	_history_page = history
	for page in [negotiation, _scroll_page(war), _scroll_page(history)]:
		page.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		stack.add_child(page)
		_pages.append(page)
	_show_tab(TAB_NEGOTIATION)
	return column


func _scroll_page(content: Control) -> ScrollContainer:
	var scroll := ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	content.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(content)
	return scroll


func _build_negotiation() -> Control:
	var page := VBoxContainer.new()
	page.add_theme_constant_override("separation", 6)
	var clause_row := HBoxContainer.new()
	clause_row.add_child(_label("Clauses communes", UiType.HEADING, HudStyle.INK))
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	clause_row.add_child(spacer)
	_clause_menu = _add_menu("+ Clause")
	clause_row.add_child(_clause_menu)
	page.add_child(clause_row)
	_clauses = HFlowContainer.new()
	_clauses.add_theme_constant_override("h_separation", 6)
	page.add_child(_clauses)
	var columns := HBoxContainer.new()
	columns.add_theme_constant_override("separation", 10)
	page.add_child(columns)
	var offer := _article_column("Vous offrez")
	_offer_menu = offer[1]
	_offer_list = offer[2]
	columns.add_child(offer[0])
	var demand := _article_column("Vous demandez")
	_demand_menu = demand[1]
	_demand_list = demand[2]
	columns.add_child(demand[0])
	page.add_child(_rule())
	var chance_row := HBoxContainer.new()
	chance_row.add_theme_constant_override("separation", 8)
	_chance_label = _label("", UiType.HEADING, HudStyle.INK)
	_chance_label.custom_minimum_size = Vector2(250, 0)
	chance_row.add_child(_chance_label)
	_chance_bar = ProgressBar.new()
	_chance_bar.min_value = 0
	_chance_bar.max_value = 100
	_chance_bar.show_percentage = false
	_chance_bar.custom_minimum_size = Vector2(0, 18)
	_chance_bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_chance_bar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	chance_row.add_child(_chance_bar)
	page.add_child(chance_row)
	_reasons = RichTextLabel.new()
	_reasons.bbcode_enabled = true
	_reasons.fit_content = true
	_reasons.scroll_active = false
	_counter_box = HBoxContainer.new()
	_counter_box.name = "CounterOffer"
	_counter_box.add_theme_constant_override("separation", 8)
	_counter_label = _label("", UiType.BODY, HudStyle.INK)
	_counter_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_counter_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_counter_box.add_child(_counter_label)
	var adopt := Button.new()
	adopt.name = "AdoptCounter"
	adopt.text = "Reprendre leur contre-offre"
	RichTooltip.attach_plain(adopt, "treaty_adopt_counter")
	adopt.focus_mode = Control.FOCUS_NONE
	adopt.pressed.connect(_adopt_counter)
	_counter_box.add_child(adopt)
	_counter_box.hide()
	page.add_child(_counter_box)
	page.add_child(_reasons)
	# BP1 : mots du Codex cliquables (bulles imbriquées) dans les motifs de refus/acceptation.
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles != null:
		bubbles.call("attach", _reasons)
	var buttons := HBoxContainer.new()
	buttons.add_theme_constant_override("separation", 6)
	var counter := Button.new()
	counter.text = "Que faudrait-il ?"
	RichTooltip.attach_plain(counter, "treaty_ask_counter")
	counter.pressed.connect(_ask_counter)
	buttons.add_child(counter)
	var clear := Button.new()
	clear.text = "Effacer"
	clear.pressed.connect(func() -> void:
		_articles = []
		_render_draft())
	buttons.add_child(clear)
	var send := Button.new()
	send.name = "SendTreaty"
	send.text = "Proposer le traité"
	send.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	send.pressed.connect(_send_treaty)
	buttons.add_child(send)
	_treaty_buttons = buttons  # Q6 : hors de la page défilante (voir `_build_detail_column`)
	page.add_child(_rule())
	page.add_child(_label("Actions unilatérales", UiType.HEADING, HudStyle.INK))
	_actions = HFlowContainer.new()
	_actions.add_theme_constant_override("h_separation", 6)
	_actions.add_theme_constant_override("v_separation", 6)
	page.add_child(_actions)
	_unilateral_hint = RichTextLabel.new()
	_unilateral_hint.bbcode_enabled = true
	_unilateral_hint.fit_content = true
	_unilateral_hint.scroll_active = false
	page.add_child(_unilateral_hint)
	if bubbles != null:
		bubbles.call("attach", _unilateral_hint)
	return page


## [colonne, menu d'ajout, liste des articles].
func _article_column(title: String) -> Array:
	var box := PanelContainer.new()
	box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_theme_stylebox_override("panel", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.GOLD, 1))
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 4)
	box.add_child(column)
	var row := HBoxContainer.new()
	var label := _label(title, UiType.HEADING, HudStyle.RUBRIC)
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(label)
	var menu := _add_menu("+ Ajouter")
	row.add_child(menu)
	column.add_child(row)
	var list := VBoxContainer.new()
	list.custom_minimum_size = Vector2(0, 90)
	list.add_theme_constant_override("separation", 3)
	column.add_child(list)
	return [box, menu, list]


func _add_menu(text: String) -> MenuButton:
	var menu := MenuButton.new()
	menu.text = text
	menu.flat = false
	UiType.apply(menu, UiType.BODY)  # PO phase 2 (P2b) : plus de taille ad hoc, variation UiType
	menu.about_to_popup.connect(func() -> void: _fill_menu(menu))
	return menu


# ----- données -----------------------------------------------------------------------------

## Recharge tout depuis la simulation (à l'ouverture et après chaque ordre).
func refresh() -> void:
	if sim == null:
		return
	# PO phase 2 (P2b) : `DiplomacyController.open_panel` appelle `refresh()` avant `panel.show()`
	# — encore invisible ici, c'est donc l'ouverture : fondu d'entrée (`UiMotion`) posé depuis ce
	# panneau (le contrôleur, hors lot, ne fait alors qu'un `show()` sans effet, déjà visible).
	var opening := not visible
	_entries = sim.call("get_diplomacy", player_faction)
	_entries.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
		var rank_a := _status_rank(str(a["status"]))
		var rank_b := _status_rank(str(b["status"]))
		if rank_a != rank_b:
			return rank_a < rank_b
		return str(a["name"]) < str(b["name"]))
	if (_selected == "" or _entry(_selected).is_empty()) and not _entries.is_empty():
		_selected = str(_entries[0]["id"])
	_render_religion()
	_render_offers()
	_render_list()
	_render_map()
	_render_detail()
	if opening:
		show()
		UiMotion.fade_in(self)


func select_faction(faction_id: String) -> void:
	_selected = faction_id
	_render_list()
	_render_map()
	_render_detail()


static func _status_rank(status: String) -> int:
	return ["war", "suzerain", "vassal", "alliance", "truce", "peace"].find(status)


func _entry(id: String) -> Dictionary:
	for entry in _entries:
		if str(entry["id"]) == id:
			return entry
	return {}


func _render_religion() -> void:
	var religion: Dictionary = sim.call("get_religion_state", player_faction)
	var text := "Chancellerie — %s, faveur pontificale %d/100" % [religion.get("religion_name", "?"), int(religion.get("papal_favor", 0))]
	if bool(religion.get("excommunicated", false)):
		text += " — EXCOMMUNIÉ (%s)" % FrText.count(int(religion.get("turns_left", 0)), "tour", "tours")
	if bool(religion.get("schism", false)):
		text += " — Grand Schisme"
	_religion_label.text = text


func _render_offers() -> void:
	for child in _offers_box.get_children():
		child.queue_free()
	var offers: Array = sim.call("get_offers")
	if offers.is_empty():
		return
	_offers_box.add_child(_section("Propositions reçues"))
	var feudal := {}  # FE6 : appels féodaux (protection, arbitrage) par id d'offre
	if sim.has_method("get_feudal_offers"):
		for call in sim.call("get_feudal_offers"):
			feudal[int(call.get("id", -1))] = call
	for offer in offers:
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 6)
		var heraldry := TextureRect.new()
		heraldry.texture = PortraitLoader.heraldry_texture(str(offer.get("from", "")))
		heraldry.custom_minimum_size = Vector2(26, 26)
		heraldry.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		heraldry.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		row.add_child(heraldry)
		var text := _label("%s (%s)" % [str(offer["text"]), FrText.count(int(offer["expires_in"]), "tour", "tours")], UiType.BODY, HudStyle.INK)
		text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		row.add_child(text)
		var offer_id := int(offer["id"])
		var kind := str(offer.get("kind", ""))
		var yes := Button.new()
		yes.text = {"protection": "Intervenir", "arbitration": "Imposer la paix"}.get(kind, "Accepter")
		yes.pressed.connect(func() -> void: offer_answered.emit(offer_id, true))
		row.add_child(yes)
		if kind == "arbitration" and feudal.has(offer_id):
			var call: Dictionary = feudal[offer_id]
			for side in [["attacker", "attacker_name"], ["target", "target_name"]]:
				var side_id := str(call.get(side[0], ""))
				var take := Button.new()
				take.text = "Soutenir %s" % str(call.get(side[1], side_id))
				RichTooltip.attach_plain(take, "feudal_take_side", {"title": "Prendre le parti de %s" % str(call.get(side[1], side_id)), "body": "Guerre contre l'autre vassal."})
				take.pressed.connect(func() -> void: arbitration_requested.emit(offer_id, "take_side", side_id))
				row.add_child(take)
		var no := Button.new()
		no.text = {"protection": "Se dérober", "arbitration": "Laisser faire"}.get(kind, "Refuser")
		no.pressed.connect(func() -> void: offer_answered.emit(offer_id, false))
		row.add_child(no)
		_offers_box.add_child(row)


func _passes_filter(status: String) -> bool:
	match _filter:
		1:
			return status == "war"
		2:
			return status in ["alliance", "vassal", "suzerain"]
		3:
			return status in ["peace", "truce"]
	return true


func _render_list() -> void:
	for child in _list.get_children():
		child.queue_free()
	for entry in _entries:
		var status := str(entry["status"])
		if not _passes_filter(status):
			continue
		_list.add_child(_faction_row(entry))


func _faction_row(entry: Dictionary) -> Control:
	var id := str(entry["id"])
	var status := str(entry["status"])
	var attitude := int(entry["attitude"])
	var row := Button.new()
	row.name = "Faction_%s" % id
	row.toggle_mode = true
	row.button_pressed = id == _selected
	row.custom_minimum_size = Vector2(0, 58)
	row.focus_mode = Control.FOCUS_NONE
	row.pressed.connect(func() -> void: select_faction(id))
	var reasons := PackedStringArray(["Attitude envers nous : %+d" % attitude])
	for reason in entry.get("attitude_reasons", []):
		reasons.append("%+d  %s" % [int(reason["value"]), str(reason["text"])])
	RichTooltip.attach_plain(row, "faction_attitude", {"body": "\n".join(reasons)})
	var line := HBoxContainer.new()
	line.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	line.offset_left = 6
	line.offset_right = -6
	line.mouse_filter = Control.MOUSE_FILTER_IGNORE
	line.add_theme_constant_override("separation", 8)
	line.add_child(_heraldry(id, 36, str(entry.get("color", "#888888"))))
	var names := VBoxContainer.new()
	names.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	names.alignment = BoxContainer.ALIGNMENT_CENTER
	names.add_theme_constant_override("separation", -2)
	names.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var name_label := _label(str(entry["name"]), UiType.BODY, HudStyle.INK)
	name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	names.add_child(name_label)
	var ruler := str(entry.get("ruler", ""))
	if ruler != "":
		var ruler_label := _label(ruler, UiType.CAPTION, HudStyle.INK_FADED)
		ruler_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
		ruler_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
		names.add_child(ruler_label)
	line.add_child(names)
	var right := VBoxContainer.new()
	right.alignment = BoxContainer.ALIGNMENT_CENTER
	right.mouse_filter = Control.MOUSE_FILTER_IGNORE
	right.add_theme_constant_override("separation", 0)
	var symbol := Accessibility.relation_symbol(status) if Accessibility.colorblind() else ""
	var status_label := _label(("%s %s" % [symbol, STATUS_LABELS.get(status, status)]).strip_edges(), UiType.CAPTION, STATUS_COLORS.get(status, HudStyle.INK))
	status_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	status_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	right.add_child(status_label)
	var attitude_label := _label("%+d" % attitude, UiType.BODY, HudStyle.GOOD if attitude >= 0 else HudStyle.POOR)
	attitude_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	attitude_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	right.add_child(attitude_label)
	line.add_child(right)
	line.add_child(_attitude_gauge(attitude))
	row.add_child(line)
	return row


func _heraldry(faction_id: String, side: int, fallback: String) -> Control:
	var texture := PortraitLoader.heraldry_texture(faction_id)
	if texture == null:
		var swatch := ColorRect.new()
		swatch.color = Color.html(fallback)
		swatch.custom_minimum_size = Vector2(side * 0.8, side)
		swatch.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		swatch.mouse_filter = Control.MOUSE_FILTER_IGNORE
		return swatch
	var rect := TextureRect.new()
	rect.texture = texture
	rect.custom_minimum_size = Vector2(side, side)
	rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	rect.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return rect


## Jauge verticale d'attitude (-100..100) : moitié haute verte, moitié basse rouge.
func _attitude_gauge(attitude: int) -> Control:
	var gauge := AttitudeGauge.new()
	gauge.attitude = attitude
	return gauge


# ----- carte ---------------------------------------------------------------------------------

func _ensure_minimap() -> void:
	if _minimap != null or map_data == null:
		return
	var holder: Control = find_child("MapHolder", true, false)
	if holder == null:
		return
	_minimap = CampaignMinimap.new()
	_minimap.name = "DiplomacyMap"
	_minimap.setup(map_data)
	# Pas de boutons de mode : la carte des relations n'a qu'un rendu.
	for child in _minimap.get_child(0).get_children():
		if child is HBoxContainer:
			child.hide()
	_minimap.clicked.connect(_on_map_clicked)
	holder.add_child(_minimap)
	_fit_minimap()
	_refit_minimap_later()


## Q6 : ajustement différé (une fois par image) quand le panneau change de taille ; appelé
## directement depuis `resized`, il se relançait lui-même en rendant sa taille au panneau.
func _queue_fit_minimap() -> void:
	if _fit_queued or _fitting:
		return
	_fit_queued = true
	(func() -> void:
		_fit_queued = false
		if is_inside_tree():
			_fit_minimap()).call_deferred()


func _fit_minimap() -> void:
	if _minimap == null or _fitting:
		return
	var holder := _minimap.get_parent() as Control
	var view := _minimap.find_child("MapView", true, false) as Control
	if holder == null or view == null or _minimap.crop.size.x <= 0.0:
		return
	var aspect := _minimap.crop.size.y / _minimap.crop.size.x
	# Habillage réel de la minicarte (cadre, boutons) autour de la vue, plus un jeu de 4 px :
	# une marge fixe plus petite que le cadre ferait grandir le conteneur à chaque `resized`.
	var chrome := _minimap.get_combined_minimum_size() - view.get_combined_minimum_size() + Vector2(4.0, 4.0)
	# Q6 : place prise au-delà de la taille visée (le panneau a grandi avec son contenu).
	var excess := Vector2.ZERO
	if _target_size != Vector2.ZERO:
		excess = (size - _target_size).max(Vector2.ZERO)
	var room := holder.size - chrome - excess
	var width := maxf(room.x, MAP_MIN_WIDTH)
	var height := width * aspect
	if height > room.y:
		height = maxf(room.y, MAP_MIN_HEIGHT)
		width = height / aspect
	var fitted := Vector2(width, height).floor()
	if fitted != view.custom_minimum_size:
		view.custom_minimum_size = fitted
	if excess != Vector2.ZERO:
		_fitting = true
		size = _target_size  # rendu à la taille visée (bornée par la nouvelle taille minimale)
		_fitting = false
	_minimap.tooltip_text = ""


func _render_map() -> void:
	_ensure_minimap()
	if _minimap == null or sim == null:
		return
	var ids := PackedStringArray()
	for index in range(1, map_data.province_count + 1):
		ids.append(str(map_data.get_province(index).get("id", "")))
	# RS-E : instantané groupé (mêmes index que `ids`, même construction) au lieu d'un
	# `get_province_state` par province.
	var snapshot := ProvinceSnapshot.of(sim, map_data)
	# DP2 : mêmes couleurs que le mode « Diplomatie » de la carte et de la minicarte.
	if DiplomaticStances.available(sim):
		# DZ : vue de la faction choisie (ses ennemis en rouge, ses terres en blanc).
		var viewer := map_viewer()
		_update_map_caption(viewer)
		var stances := DiplomaticStances.stances(sim, ids, viewer)
		var stance_colors := PackedColorArray()
		for index in ids.size():
			var key := stances[index] if index < stances.size() else ""
			var stance_color := DiplomaticStances.color_of(key)
			if viewer == "" and key != "" and key != "self" and _selected != "" and _controller_of(snapshot, index) == _selected:
				stance_color = stance_color.lightened(0.3)
			stance_colors.append(stance_color)
		_minimap.set_province_colors(stance_colors)
		return
	var relations: PackedStringArray = sim.call("get_province_relations", ids)
	var attitude_of := {}
	for entry in _entries:
		attitude_of[str(entry["id"])] = int(entry["attitude"])
	var colors := PackedColorArray()
	for index in ids.size():
		var relation := relations[index] if index < relations.size() else ""
		var color := Color(0, 0, 0, 0)
		# Contrôleur lu dans l'instantané groupé (et seulement s'il sert).
		var controller: String = _controller_of(snapshot, index) if relation != "" and relation != "self" else ""
		if MAP_COLORS.has(relation):
			color = MAP_COLORS[relation]
		elif relation == "peace":
			var attitude := int(attitude_of.get(controller, 0))
			color = NEUTRAL.lerp(FRIENDLY if attitude >= 0 else HOSTILE, clampf(absf(attitude) / 60.0, 0.0, 1.0))
		if relation != "" and relation != "self" and _selected != "" and controller == _selected:
			color = color.lightened(0.22)
		colors.append(color)
	_minimap.set_province_colors(colors)


## DZ : faction dont la carte montre les relations ("" : le joueur).
func map_viewer() -> String:
	if not _map_their_view or _selected == "" or _selected == player_faction:
		return ""
	if sim == null or not sim.has_method("get_province_stances_for"):
		return ""
	return _selected


func _update_map_caption(viewer: String) -> void:
	if _map_caption == null:
		return
	_map_caption.text = "Relations de %s" % SimFacade.faction_short_name(viewer) if viewer != "" else "Vos relations"
	if _map_view_toggle != null:
		_map_view_toggle.visible = sim != null and sim.has_method("get_province_stances_for")


## Contrôleur de la province d'index `index` dans l'instantané groupé (RS-E : `ProvinceSnapshot`,
## au lieu d'une lecture `get_province_state` par province).
func _controller_of(snapshot: ProvinceSnapshot, index: int) -> String:
	if index < 0 or index >= snapshot.controller.size():
		return ""
	return snapshot.controller[index]


func _on_map_clicked(map_pos: Vector2) -> void:
	if map_data == null or sim == null:
		return
	var index := map_data.province_index_at(map_pos.x, map_pos.y)
	if index <= 0:
		return
	var id := str(map_data.get_province(index).get("id", ""))
	var state: Dictionary = sim.call("get_province_state", id)
	var owner := str(state.get("controller", state.get("owner", "")))
	if owner != "" and owner != player_faction and not _entry(owner).is_empty():
		select_faction(owner)


# ----- fiche et onglets ------------------------------------------------------------------------

func _show_tab(index: int) -> void:
	_tab = index
	for i in _pages.size():
		_pages[i].visible = i == index
	if _treaty_buttons != null:
		_treaty_buttons.visible = index == TAB_NEGOTIATION
	for i in _tab_buttons.size():
		_tab_buttons[i].set_pressed_no_signal(i == index)


func _render_detail() -> void:
	for child in _head.get_children():
		child.queue_free()
	var entry := _entry(_selected)
	if entry.is_empty():
		return
	var status := str(entry["status"])
	var top := HBoxContainer.new()
	top.add_theme_constant_override("separation", 10)
	top.add_child(_heraldry(_selected, 64, str(entry.get("color", "#888888"))))
	var names := VBoxContainer.new()
	names.add_theme_constant_override("separation", 0)
	names.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	names.add_child(_label(str(entry["name"]), UiType.HEADING, HudStyle.INK))
	var ruler := str(entry.get("ruler", ""))
	names.add_child(_label(ruler if ruler != "" else "Souverain inconnu", UiType.BODY, HudStyle.INK_SOFT))
	var line := "%s — attitude %+d" % [STATUS_LABELS.get(status, status), int(entry["attitude"])]
	names.add_child(_label(line, UiType.BODY, STATUS_COLORS.get(status, HudStyle.INK)))
	top.add_child(names)
	_head.add_child(top)
	var facts := PackedStringArray()
	facts.append("Religion : %s" % entry.get("religion_name", "?"))
	facts.append("Puissance : %s" % HudStyle.thousands(int(entry.get("power", 0))))
	if int(entry.get("truce_turns_left", 0)) > 0:
		facts.append("Trêve : encore %s" % FrText.count(int(entry["truce_turns_left"]), "tour", "tours"))
	if bool(entry.get("trade_agreement", false)):
		# C5 : un embargo suspend les routes sans rompre l'accord (la guerre le rompt).
		var suspended := bool(entry.get("embargo_by_us", false)) or bool(entry.get("embargo_on_us", false))
		facts.append("Accord commercial" + (" (suspendu)" if suspended else ""))
	if bool(entry.get("access_received", false)):
		facts.append("Accès militaire accordé")
	facts.append_array(_passage_facts(_selected))
	if int(entry.get("loyalty", -1)) >= 0:
		facts.append("Loyauté %d/100" % int(entry["loyalty"]))
	if bool(entry.get("embargo_by_us", false)):
		facts.append("Sous notre embargo")
	if bool(entry.get("embargo_on_us", false)):
		facts.append("Nous impose un embargo")
	var facts_label := _label(" · ".join(facts), UiType.CAPTION, HudStyle.INK_SOFT)
	facts_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_head.add_child(facts_label)
	_render_trade_routes(_head, _selected)
	if _draft_for != _selected:
		_draft_for = _selected
		_articles = [{"kind": "peace"}] if status == "war" else []
	_options = sim.call("treaty_options", _selected) if sim.has_method("treaty_options") else {}
	_render_draft()
	_render_actions(entry)
	_render_war(entry)
	_render_history()


# ----- négociation ---------------------------------------------------------------------------

func _render_draft() -> void:
	for box in [_clauses, _offer_list, _demand_list]:
		for child in box.get_children():
			child.queue_free()
	_verdict = {}
	if sim != null and sim.has_method("evaluate_treaty") and not _articles.is_empty():
		_verdict = sim.call("evaluate_treaty", _selected, _articles)
	var values: Array = _verdict.get("articles", [])
	for index in _articles.size():
		var article: Dictionary = _articles[index]
		var value: Dictionary = values[index] if index < values.size() else {}
		var giver := str(article.get("giver", ""))
		var target: Container = _clauses if giver == "" else (_offer_list if giver == "proposer" else _demand_list)
		target.add_child(_article_row(index, article, value))
	for list in [_offer_list, _demand_list]:
		if list.get_child_count() == 0:
			list.add_child(_label("—", UiType.BODY, HudStyle.INK_FADED))
	if _clauses.get_child_count() == 0:
		_clauses.add_child(_label("Aucune clause commune.", UiType.CAPTION, HudStyle.INK_FADED))
	_render_chance()


func _article_row(index: int, article: Dictionary, value: Dictionary) -> Control:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 4)
	var label_text := str(value.get("label", _fallback_label(article)))
	var label := _label(label_text, UiType.BODY, HudStyle.INK)
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.custom_minimum_size = Vector2(120, 0)
	var tips := PackedStringArray()
	for reason in value.get("reasons", []):
		tips.append("%+d  %s" % [int(reason["value"]), str(reason["text"])])
	if str(value.get("blocked", "")) != "":
		tips.append("Impossible : %s" % value["blocked"])
	RichTooltip.attach_plain(label, "treaty_clause_detail", {"body": "\n".join(tips)})
	label.mouse_filter = Control.MOUSE_FILTER_STOP
	row.add_child(label)
	if not value.is_empty():
		var points := int(value.get("value", 0))
		var points_label := _label("%+d" % points, UiType.BODY, HudStyle.GOOD if points >= 0 else HudStyle.POOR)
		RichTooltip.attach_plain(points_label, "treaty_clause_value_for_them")
		points_label.mouse_filter = Control.MOUSE_FILTER_STOP
		row.add_child(points_label)
	var remove := Button.new()
	remove.text = "×"
	RichTooltip.attach_plain(remove, "treaty_clause_remove")
	remove.focus_mode = Control.FOCUS_NONE
	remove.pressed.connect(func() -> void:
		_articles.remove_at(index)
		_render_draft())
	row.add_child(remove)
	return row


func _fallback_label(article: Dictionary) -> String:
	return str(article.get("kind", "?")).replace("_", " ")


func _render_chance() -> void:
	var fill := StyleBoxFlat.new()
	_counter_box.hide()
	_counter_articles = []
	_explanation = {}
	if _articles.is_empty():
		_chance_label.text = "Ajoutez des clauses"
		_chance_bar.value = 0
		_reasons.text = ""
		return
	var chance := int(_verdict.get("chance", 0))
	var blocked := str(_verdict.get("blocked", ""))
	_chance_bar.value = chance
	fill.bg_color = HudStyle.gauge_color(chance / 100.0)
	_chance_bar.add_theme_stylebox_override("fill", fill)
	var word := "accepterait" if chance >= 66 else ("hésite" if chance >= 34 else "refuserait")
	_chance_label.text = "Chance d'acceptation : %d %% (%s)" % [chance, word]
	if _render_explanation():
		return
	var text := ""
	if blocked != "":
		text += "[color=#8b1a1a][b]Impossible :[/b] %s[/color]\n" % CodexText.format(blocked, true)
	var context: Array = _verdict.get("context", [])
	var parts := PackedStringArray()
	for reason in context:
		var v := int(reason["value"])
		parts.append("[color=%s]%+d[/color] %s" % ["#2a6a2a" if v >= 0 else "#8b1a1a", v, CodexText.format(str(reason["text"]), true)])
	if not parts.is_empty():
		text += "[b]Considérations :[/b] " + " · ".join(parts)
	_reasons.text = text


## Lot DP2 : leur raisonnement ligne à ligne, chaque raison avec son poids (à la Warhammer III :
## « Ils se méfient de vous −12 », « Accord commercial — routes communes +8 »), puis la
## contre-offre quand un seul point bloque. Faux si la simulation ne l'explique pas.
func _render_explanation() -> bool:
	if sim == null or not sim.has_method("explain_treaty"):
		return false
	_explanation = sim.call("explain_treaty", _selected, _articles)
	if not bool(_explanation.get("ok", false)):
		return false
	var blocker: Dictionary = _explanation.get("blocker", {})
	var blocker_text := str(blocker.get("text", ""))
	var accept := bool(_explanation.get("accept", false))
	var text := "[b][color=%s]%s[/color][/b]\n" % ["#2a6a2a" if accept else "#8b1a1a", _explanation.get("summary", "")]
	var lines: Array = _explanation.get("lines", [])
	var shown := 0
	for line in lines:
		var v := int(line["value"])
		var line_text := str(line["text"])
		var entry := "[color=%s]%+d[/color]  %s" % ["#2a6a2a" if v >= 0 else "#8b1a1a", v, line_text]
		if blocker_text != "" and (line_text == blocker_text or line_text.begins_with(blocker_text + " — ")):
			entry = "[b]%s[/b]  ◄" % entry
		text += entry + "\n"
		shown += 1
		if shown >= 14 and lines.size() > 15:
			text += "[color=#6b5a45]… %d autres raisons de moindre poids[/color]\n" % (lines.size() - shown)
			break
	_reasons.text = text.strip_edges()
	var counter: Array = _explanation.get("counter", [])
	if not accept and not counter.is_empty():
		_counter_articles = counter
		_counter_label.text = "Leur contre-offre (%d %%) : %s." % [int(_explanation.get("counter_chance", 0)), _explanation.get("counter_text", "")]
		_counter_box.show()
	return true


func _adopt_counter() -> void:
	if _counter_articles.is_empty():
		return
	_articles = _counter_articles.duplicate(true)
	_render_draft()


func _fill_menu(menu: MenuButton) -> void:
	var popup := menu.get_popup()
	popup.clear()
	for child in popup.get_children():
		if child is PopupMenu:
			child.queue_free()
	if popup.id_pressed.is_connected(_on_menu_id):
		popup.id_pressed.disconnect(_on_menu_id)
	var items: Array = []  # [label, article | null, children[]]
	var ours: Dictionary = _options.get("ours", {})
	var theirs: Dictionary = _options.get("theirs", {})
	var at_war := bool(_options.get("at_war", false))
	if menu == _clause_menu:
		if at_war:
			items.append(["Paix", {"kind": "peace"}])
			items.append(["Trêve de deux ans", {"kind": "truce", "turns": TRUCE_TURNS}])
		if not bool(_options.get("allied", false)):
			items.append(["Alliance", {"kind": "alliance"}])
		if not bool(_options.get("trade", false)):
			items.append(["Accord commercial", {"kind": "trade_agreement"}])
		var pairs: Array = []
		for a in ours.get("marriageable", []):
			for b in theirs.get("marriageable", []):
				if bool(a.get("female", false)) != bool(b.get("female", false)) and pairs.size() < 10:
					pairs.append(["%s et %s" % [a["name"], b["name"]], {"kind": "marriage", "character": a["id"], "spouse": b["id"]}])
		if not pairs.is_empty():
			items.append(["Mariage", null, pairs])
	else:
		var giver := "proposer" if menu == _offer_menu else "recipient"
		var side: Dictionary = ours if giver == "proposer" else theirs
		var other_occupies := "occupée par eux" if giver == "proposer" else "occupée par nous"
		var gold: Array = []
		for amount in GOLD_STEPS:
			if amount <= int(side.get("treasury", 0)):
				gold.append([Money.amount(amount), {"kind": "gold", "giver": giver, "amount": amount}])
		if not gold.is_empty():
			items.append(["Or", null, gold])
		var tribute: Array = []
		for amount in TRIBUTE_STEPS:
			tribute.append(["%s par saison, %d saisons" % [Money.amount(amount), TRIBUTE_SEASONS], {"kind": "tribute", "giver": giver, "per_season": amount, "seasons": TRIBUTE_SEASONS}])
		items.append(["Tribut", null, tribute])
		var access_key := "access_given" if giver == "proposer" else "access_received"
		if not bool(_options.get(access_key, false)):
			items.append(["Accès militaire", {"kind": "military_access", "giver": giver}])
		var provinces: Array = []
		for province in side.get("provinces", []):
			var label := str(province["name"])
			if bool(province.get("capital", false)):
				label += " (capitale)"
			if bool(province.get("occupied", false)):
				label += " — %s" % other_occupies
			if bool(province.get("war_goal", false)):
				label = "★ " + label
			provinces.append([label, {"kind": "cede_province", "giver": giver, "province": province["id"]}])
		if not provinces.is_empty():
			items.append(["Province", null, provinces])
		var places: Array = []
		for place in side.get("settlements", []):
			if places.size() >= 24:
				break
			places.append(["%s (%s)" % [place["name"], place["province"]], {"kind": "cede_settlement", "giver": giver, "settlement": place["id"]}])
		if not places.is_empty():
			items.append(["Place forte ou colonie", null, places])
		var captives: Array = []
		for captive in side.get("captives", []):
			captives.append([str(captive["name"]), {"kind": "release_captive", "giver": giver, "character": captive["id"]}])
		if not captives.is_empty():
			items.append(["Libérer un captif", null, captives])
		var hostages: Array = []
		for hostage in side.get("hostages", []):
			hostages.append([str(hostage["name"]), {"kind": "hostage", "giver": giver, "character": hostage["id"]}])
		if not hostages.is_empty():
			items.append(["Otage", null, hostages])
		items.append(["Devenir vassal" if giver == "proposer" else "Vassalité", {"kind": "vassalage", "giver": giver}])
	var id := 0
	var lookup := {}
	for item in items:
		if item.size() > 2:
			var sub := PopupMenu.new()
			for child in item[2]:
				sub.add_item(str(child[0]), id)
				lookup[id] = child[1]
				id += 1
			sub.id_pressed.connect(func(pressed: int) -> void: _add_article(lookup.get(pressed, {})))
			popup.add_submenu_node_item(str(item[0]), sub)
		else:
			popup.add_item(str(item[0]), id)
			lookup[id] = item[1]
			id += 1
	popup.set_meta("lookup", lookup)
	popup.id_pressed.connect(_on_menu_id.bind(popup))


func _on_menu_id(pressed: int, popup: PopupMenu) -> void:
	var lookup: Dictionary = popup.get_meta("lookup", {})
	_add_article(lookup.get(pressed, {}))


func _add_article(article: Dictionary) -> void:
	if article.is_empty():
		return
	for existing in _articles:
		if JSON.stringify(existing) == JSON.stringify(article):
			return
	# Un seul versement d'or et un seul tribut par camp : le nouveau remplace l'ancien.
	if str(article["kind"]) in ["gold", "tribute"]:
		for index in range(_articles.size() - 1, -1, -1):
			var old: Dictionary = _articles[index]
			if old.get("kind") == article["kind"] and old.get("giver") == article.get("giver"):
				_articles.remove_at(index)
	_articles.append(article)
	_render_draft()


func _ask_counter() -> void:
	if sim == null or not sim.has_method("counter_treaty"):
		return
	var answer: Dictionary = sim.call("counter_treaty", _selected, _articles)
	if bool(answer.get("ok", false)):
		_articles = answer.get("articles", [])
		_render_draft()
	else:
		_chance_label.text = str(answer.get("error", "Aucune contre-proposition."))


func _send_treaty() -> void:
	if _articles.is_empty():
		return
	order_requested.emit({"type": "propose_treaty", "target": _selected, "articles": _articles}, "Le traité est signé.")


## Actions sans négociation (guerre, embargo, présents, ruptures, médiation). Survol = conséquences.
func _render_actions(entry: Dictionary) -> void:
	for child in _actions.get_children():
		child.queue_free()
	_unilateral_hint.text = ""
	var id := str(entry["id"])
	var status := str(entry["status"])
	if status == "war":
		_add_action("Médiation pontificale (%s)" % Money.amount(int(RuleValues.value("mediation_cost", 0.0))), {"type": "request_papal_mediation", "target": id}, "Le pape obtient une trêve.", false)
	elif status != "alliance" and status != "vassal" and status != "suzerain":
		_add_action("Déclarer la guerre", {"type": "declare_war", "target": id}, "La guerre est déclarée.", true)
	if status == "alliance":
		_add_action("Rompre l'alliance", {"type": "break_alliance", "target": id}, "Alliance rompue.", true)
	if status == "vassal":
		_add_action("Libérer le vassal", {"type": "release_vassal", "target": id}, "Vassal libéré.", true)
	var embargo := bool(entry.get("embargo_by_us", false))
	_add_action("Lever l'embargo" if embargo else "Imposer un embargo",
		{"type": "set_embargo", "target": id, "active": not embargo},
		"Embargo levé." if embargo else "Embargo imposé.", true)
	_add_action("Présents (%s)" % Money.amount(GIFT_AMOUNT), {"type": "send_gift", "target": id, "amount": GIFT_AMOUNT}, "Présents envoyés.", true)
	# C5 : l'accord commercial se conclut par un article de traité ; la rupture est unilatérale.
	if bool(entry.get("trade_agreement", false)):
		_add_action("Rompre l'accord commercial", {"type": "break_trade_agreement", "target": id}, "Accord commercial rompu.", true)


## Lot DP2 : position diplomatique, droit de passage et intrusions entre nous et `id`.
func _passage_facts(id: String) -> PackedStringArray:
	var facts := PackedStringArray()
	if sim == null:
		return facts
	if sim.has_method("get_faction_stance"):
		var stance: Dictionary = sim.call("get_faction_stance", id)
		if not stance.is_empty():
			facts.append("Position : %s" % stance.get("label", ""))
	if not sim.has_method("get_trespass"):
		return facts
	var info: Dictionary = sim.call("get_trespass", id)
	if info.is_empty():
		return facts
	var theirs: Dictionary = info.get("theirs", {})
	var ours: Dictionary = info.get("ours", {})
	if bool(info.get("access_given", false)):
		facts.append("Droit de passage donné")
	if int(theirs.get("seasons", 0)) > 0:
		facts.append("Leurs armées campent sur nos terres (%s)" % FrText.count(int(theirs["seasons"]), "saison", "saisons"))
	if bool(theirs.get("grievance", false)):
		facts.append("Casus belli : violation de nos frontières")
	if int(ours.get("seasons", 0)) > 0:
		facts.append("Nos armées campent chez eux sans droit de passage (%s)" % FrText.count(int(ours["seasons"]), "saison", "saisons"))
	if bool(ours.get("grievance", false)):
		facts.append("Ils tiennent un casus belli contre nous (intrusion)")
	return facts


## Lot C5 : routes commerciales entre nous et cette faction (revenu par saison, biens, coupure).
func _render_trade_routes(parent: Control, id: String) -> void:
	if sim == null or not sim.has_method("get_trade_routes"):
		return
	var lines := PackedStringArray()
	for route_variant in sim.call("get_trade_routes"):
		var route: Dictionary = route_variant
		var from_f := str(route.get("from_faction", ""))
		var to_f := str(route.get("to_faction", ""))
		if not ((from_f == player_faction and to_f == id) or (from_f == id and to_f == player_faction)):
			continue
		if bool(route.get("cut", false)):
			lines.append("%s ↔ %s : coupée (%s)" % [route["from_hub_name"], route["to_hub_name"], route["cut_reason"]])
		else:
			var goods: PackedStringArray = route.get("goods", PackedStringArray())
			lines.append("%s ↔ %s : %s/saison (%s)" % [route["from_hub_name"], route["to_hub_name"],
				Money.amount(int(route["total_value"])), ", ".join(goods)])
	if lines.is_empty():
		return
	var label := _label("Commerce — " + " · ".join(lines), UiType.CAPTION, HudStyle.INK_SOFT)
	label.name = "TradeRoutes"
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	parent.add_child(label)


func _add_action(label: String, order: Dictionary, success_text: String, unilateral: bool) -> void:
	var button := Button.new()
	button.text = label
	button.focus_mode = Control.FOCUS_NONE
	button.mouse_entered.connect(func() -> void: _show_consequences(order, unilateral))
	button.pressed.connect(func() -> void: order_requested.emit(order, success_text))
	_actions.add_child(button)


func _show_consequences(order: Dictionary, unilateral: bool) -> void:
	if sim == null:
		return
	var type := str(order.get("type", ""))
	if unilateral and type != "declare_war":
		_unilateral_hint.text = ""
		return
	var verdict: Dictionary = sim.call("evaluate_proposal", order)
	var text := "[b]Conséquences :[/b] " if type == "declare_war" else ("[b][color=#2a6a2a]Accepterait[/color][/b] : " if bool(verdict.get("accept", false)) else "[b][color=#8b1a1a]Refuserait[/color][/b] : ")
	var parts := PackedStringArray()
	for reason in verdict.get("reasons", []):
		var v := int(reason["value"])
		var reason_text := CodexText.format(str(reason["text"]), true)
		parts.append(("[color=%s]%+d[/color] %s" % ["#2a6a2a" if v >= 0 else "#8b1a1a", v, reason_text]) if v != 0 else reason_text)
	_unilateral_hint.text = text + " · ".join(parts)
	if type == "declare_war":  # FE6 : chaîne d'escalade avant la déclaration
		var player := str(sim.call("get_player_faction")) if sim.has_method("get_player_faction") else ""
		var chain := EscalationPreview.bbcode(sim, player, str(order.get("target", "")))
		if chain != "":
			_unilateral_hint.text += "\n" + chain


# ----- guerre et traités --------------------------------------------------------------------------

func _render_war(entry: Dictionary) -> void:
	for child in _war_page.get_children():
		child.queue_free()
	var status := str(entry["status"])
	if str(entry.get("casus_belli", "")) != "":
		_war_page.add_child(_label("Casus belli : %s" % entry["casus_belli"], UiType.BODY, HudStyle.INK))
	for claim in entry.get("claims", []):
		var claim_label := _label("Prétention : %s" % claim, UiType.BODY, HudStyle.INK_SOFT)
		claim_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		_war_page.add_child(claim_label)
	if status != "war" or not sim.has_method("get_war_summary"):
		_war_page.add_child(_label("Pas de guerre en cours avec cette faction.", UiType.BODY, HudStyle.INK_FADED))
		return
	var summary: Dictionary = sim.call("get_war_summary", _selected)
	var score := int(summary.get("war_score", 0))
	_war_page.add_child(_section("Score de guerre : %+d" % score))
	var bar := ProgressBar.new()
	bar.min_value = -100
	bar.max_value = 100
	bar.value = score
	bar.show_percentage = false
	bar.custom_minimum_size = Vector2(0, 16)
	var fill := StyleBoxFlat.new()
	fill.bg_color = HudStyle.GOOD if score >= 0 else HudStyle.POOR
	bar.add_theme_stylebox_override("fill", fill)
	_war_page.add_child(bar)
	_war_page.add_child(_label("Batailles, sièges et provinces occupées remplissent le score ; les buts de guerre tenus comptent double. Plus il est haut, plus l'ennemi cédera de terres.", UiType.CAPTION, HudStyle.INK_FADED))
	(_war_page.get_child(_war_page.get_child_count() - 1) as Label).autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_war_page.add_child(_label("Fatigue de guerre : nous %d/100, eux %d/100" % [int(summary.get("weariness_ours", 0)), int(summary.get("weariness_theirs", 0))], UiType.BODY, HudStyle.INK))
	for pair in [["Nos buts de guerre", "goals_ours"], ["Leurs buts de guerre", "goals_theirs"]]:
		_war_page.add_child(_label(str(pair[0]), UiType.HEADING, HudStyle.RUBRIC))
		var goals: Array = summary.get(pair[1], [])
		if goals.is_empty():
			_war_page.add_child(_label("—", UiType.BODY, HudStyle.INK_FADED))
		for goal in goals:
			var held := bool(goal.get("held", false))
			_war_page.add_child(_label("%s %s%s" % ["★" if held else "☆", goal["name"], " (tenue)" if held else ""], UiType.BODY, HudStyle.INK))


func _render_history() -> void:
	for child in _history_page.get_children():
		child.queue_free()
	if sim == null or not sim.has_method("get_treaty_history"):
		return
	var shown := 0
	for record in sim.call("get_treaty_history", player_faction):
		if str(record.get("with", "")) != _selected and shown >= 0 and _filter_history_to_selected():
			continue
		var accepted := bool(record.get("accepted", false))
		var head := "%s — %s, %s" % [record.get("date", ""), record.get("with_name", ""), "signé" if accepted else "refusé"]
		_history_page.add_child(_label(head, UiType.BODY, HudStyle.INK if accepted else HudStyle.RUBRIC))
		var body := _label(str(record.get("text", "")), UiType.CAPTION, HudStyle.INK_SOFT)
		body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		_history_page.add_child(body)
		shown += 1
	if shown == 0:
		_history_page.add_child(_label("Aucun traité avec cette faction.", UiType.BODY, HudStyle.INK_FADED))


## Captures et smoke : un brouillon de paix type (province exigée, or offert).
func stage_example() -> void:
	var theirs: Array = (_options.get("theirs", {}) as Dictionary).get("provinces", [])
	_articles = [{"kind": "peace"}, {"kind": "gold", "giver": "proposer", "amount": 5000}]
	for province in theirs:
		if not bool(province.get("capital", false)):
			_articles.append({"kind": "cede_province", "giver": "recipient", "province": province["id"]})
			break
	_articles.append({"kind": "trade_agreement"})
	_render_draft()


## DP2 (captures) : une offre généreuse gâchée par une seule exigence d'or excessive.
func stage_counter_example() -> void:
	_articles = [{"kind": "trade_agreement"}, {"kind": "gold", "giver": "proposer", "amount": 2500},
		{"kind": "gold", "giver": "recipient", "amount": 10000}]
	_render_draft()


func _filter_history_to_selected() -> bool:
	return true


## Jauge verticale d'attitude : trait médian, remplissage vert vers le haut, rouge vers le bas.
class AttitudeGauge:
	extends Control

	var attitude := 0

	func _init() -> void:
		custom_minimum_size = Vector2(8, 36)
		size_flags_vertical = Control.SIZE_SHRINK_CENTER
		mouse_filter = Control.MOUSE_FILTER_IGNORE

	func _draw() -> void:
		var mid := size.y * 0.5
		draw_rect(Rect2(Vector2.ZERO, size), HudStyle.PARCHMENT_DARK)
		var h := mid * clampf(absf(attitude) / 100.0, 0.0, 1.0)
		if attitude >= 0:
			draw_rect(Rect2(0, mid - h, size.x, h), HudStyle.GOOD)
		else:
			draw_rect(Rect2(0, mid, size.x, h), HudStyle.POOR)
		draw_line(Vector2(0, mid), Vector2(size.x, mid), HudStyle.INK, 1.0)
		draw_rect(Rect2(Vector2.ZERO, size), HudStyle.INK_SOFT, false, 1.0)
