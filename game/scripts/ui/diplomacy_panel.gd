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
## U16 : ouvre la couche des routes commerciales (même vue que la touche V).
signal trade_view_requested

## VN lot 3 : largeur plancher de la colonne de négociation (elle grandit avec l'écran ; la colonne
## des puissances est `DiplomacyFactionList.MIN_WIDTH`, le panneau tient dans une vue de 1138 px).
const DETAIL_COLUMN_MIN_WIDTH := 380.0
const DONATION_AMOUNT := 1000
const TAB_NEGOTIATION := 0

var sim: Object = null
var player_faction: String = ""
var map_data: MapData = null

var negotiation: DiplomacyNegotiationTab
var map_view: DiplomacyMapView
var _entries: Array = []
var _selected: String = ""
var _tab := TAB_NEGOTIATION
var _religion_label: Label
var _offers: DiplomacyOffersSection
var _faction_list: DiplomacyFactionList
var _head: DiplomacyHeadSection
var _war: DiplomacyWarTab
var _history: DiplomacyHistoryTab
var _tab_buttons: Array[Button] = []
var _pages: Array[Control] = []
var _target_size := Vector2.ZERO


func _ready() -> void:
	theme = load("res://scenes/ui/parchment_theme.tres")
	add_theme_stylebox_override("panel", _page_box())
	set_anchors_preset(Control.PRESET_TOP_LEFT)
	visibility_changed.connect(_fit_to_viewport)
	get_viewport().size_changed.connect(_fit_to_viewport)
	var root := UiBuild.vbox(6)
	add_child(root)
	root.add_child(_build_header())
	_offers = DiplomacyOffersSection.new()
	_offers.offer_answered.connect(offer_answered.emit)
	_offers.arbitration_requested.connect(arbitration_requested.emit)
	root.add_child(_offers)
	root.add_child(DiplomacyView._rule())
	var body := UiBuild.hbox(14)
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	root.add_child(body)
	_faction_list = DiplomacyFactionList.new()
	_faction_list.faction_chosen.connect(select_faction)
	map_view = DiplomacyMapView.new()
	map_view.host = self
	map_view.faction_clicked.connect(_on_map_clicked)
	# Q6 : un contenu qui grandit après l'ajustement (avis reçus, fiche) faisait déborder le
	# panneau sous l'écran, la carte gardant sa taille : on la réduit de l'excédent.
	resized.connect(map_view.queue_fit)
	body.add_child(_faction_list)
	body.add_child(map_view)
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
	map_view.target_size = _target_size
	map_view.refit_later()


func _page_box() -> StyleBoxFlat:
	var box := HudStyle.card_box(HudStyle.PARCHMENT, HudStyle.INK, 2)
	box.content_margin_left = 18
	box.content_margin_right = 18
	box.content_margin_top = 12
	box.content_margin_bottom = 12
	box.shadow_color = HudStyle.SHADOW
	box.shadow_size = 8
	return box


func _build_header() -> Control:
	var header := UiBuild.hbox(12)
	var titles := UiBuild.vbox(0)
	titles.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	# CV3-0 (#10) : lettrine du kit partagé (UI1, `Lettrine.attach`) au lieu de l'ancien
	# `DropCap` local, qui figeait le titre affiché à "iplomatie" (le "D" retiré à la main pour
	# lui faire de la place) au lieu de réserver la place comme le fait le kit.
	var title := HudStyle.label("Diplomatie", UiType.size(UiType.TITLE), HudStyle.INK)
	UiType.apply(title, UiType.TITLE)
	Lettrine.attach(title)
	titles.add_child(title)
	_religion_label = DiplomacyView._label("", UiType.BODY, HudStyle.INK_SOFT)
	titles.add_child(_religion_label)
	header.add_child(titles)
	var donate := UiBuild.button("Don à l'Église (%s)" % Money.amount(DONATION_AMOUNT))
	donate.tooltip_text = RuleValues.format("Augmente la faveur pontificale (+1 par {rule.donation_livres_per_favor} livres).")
	donate.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	donate.pressed.connect(func() -> void:
		order_requested.emit({"type": "donate_to_church", "amount": DONATION_AMOUNT}, "Don versé à l'Église."))
	header.add_child(donate)
	var trade := UiBuild.button("Commerce")
	trade.name = "TradeButton"
	trade.tooltip_text = "Affiche les routes commerciales sur la carte (touche V)."
	trade.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	trade.pressed.connect(func() -> void:
		trade_view_requested.emit())
	header.add_child(trade)
	var close := UiBuild.button("×")
	TooltipHost.attach_plain(close, "close_escape")
	close.custom_minimum_size = Vector2(36, 36)
	close.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	close.pressed.connect(func() -> void:
		UiMotion.fade_out(self)  # PO phase 2 (P2b) : fermeture animée (`UiMotion`)
		closed.emit())
	header.add_child(close)
	return header


func _build_detail_column() -> Control:
	var column := UiBuild.vbox(6)
	column.custom_minimum_size = Vector2(DETAIL_COLUMN_MIN_WIDTH, 0)
	_head = DiplomacyHeadSection.new()
	column.add_child(_head)
	var tabs := UiBuild.hbox(4)
	for index in 3:
		var button := UiBuild.button(["Négociation", "Guerre", "Traités"][index], func() -> void: _show_tab(index))
		button.toggle_mode = true
		button.focus_mode = Control.FOCUS_NONE
		tabs.add_child(button)
		_tab_buttons.append(button)
	column.add_child(tabs)
	column.add_child(DiplomacyView._rule())
	var stack := Control.new()
	stack.size_flags_vertical = Control.SIZE_EXPAND_FILL
	column.add_child(stack)
	negotiation = DiplomacyNegotiationTab.new()
	negotiation.order_requested.connect(order_requested.emit)
	_war = DiplomacyWarTab.new()
	_history = DiplomacyHistoryTab.new()
	column.add_child(negotiation.treaty_buttons)  # Q6 : hors de la page défilante
	for view in [negotiation, _war, _history]:
		var page := _scroll_page(view)
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
	_offers.show_for(sim, player_faction, _selected)
	_render_selection()
	if opening:
		show()
		UiMotion.fade_in(self)


func select_faction(faction: String) -> void:
	_selected = faction
	_render_selection()


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


func _show_tab(index: int) -> void:
	_tab = index
	for i in _pages.size():
		_pages[i].visible = i == index
	if negotiation != null:
		negotiation.treaty_buttons.visible = index == TAB_NEGOTIATION
	for i in _tab_buttons.size():
		_tab_buttons[i].set_pressed_no_signal(i == index)


## Montre la faction choisie dans la liste, la carte et la fiche (en-tête, trois onglets).
func _render_selection() -> void:
	_faction_list.show_entries(_entries, _selected)
	map_view.map_data = map_data
	map_view.entries = _entries
	map_view.show_for(sim, player_faction, _selected)
	var entry := _entry(_selected)
	_head.show_for(sim, player_faction, _selected, entry)
	if entry.is_empty():
		return
	for view in [negotiation, _war, _history]:
		view.show_for(sim, player_faction, _selected, entry)


func _on_map_clicked(owner_id: String) -> void:
	if not _entry(owner_id).is_empty():
		select_faction(owner_id)


## Captures et smoke : un brouillon de paix type.
func stage_example() -> void:
	negotiation.stage_example()


## DP2 (captures) : une offre généreuse gâchée par une seule exigence d'or excessive.
func stage_counter_example() -> void:
	negotiation.stage_counter_example()
