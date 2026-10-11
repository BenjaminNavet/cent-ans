class_name EconomyWindow
extends PanelContainer

## RG12 : fenêtre Économie (bouton « Économie » de la barre, touche `map_toggle_economy`).
## Trois onglets : « Vue d'ensemble » (`EconomyOverviewTab`), « Commerce » (`EconomyTradeTab`),
## « Monnaie » (`EconomyCurrencyTab`). Enregistrée comme panneau central (`MapUI.register_panel`) ;
## la poignée de redimensionnement vient de `PanelStack.register` (RG1). Aucune règle ici : tout
## vient de `CampaignSim` ; les ordres repartent par `order_requested`.

signal closed
signal order_requested(order: Dictionary, success_text: String)
signal trade_layer_requested

const TAB_OVERVIEW := "overview"
const TAB_TRADE := "trade"
const TAB_CURRENCY := "currency"
const TAB_ORDER := [TAB_OVERVIEW, TAB_TRADE, TAB_CURRENCY]
const TAB_TITLES := {TAB_OVERVIEW: "Vue d'ensemble", TAB_TRADE: "Commerce", TAB_CURRENCY: "Monnaie"}
const MIN_SIZE := Vector2(640, 460)

var tabs: TabContainer
var overview_tab: EconomyOverviewTab
var trade_tab: EconomyTradeTab
var currency_tab: EconomyCurrencyTab
var title_label: Label
var faction_id := ""
var sim: Object = null


func _init() -> void:
	name = "EconomyWindow"
	custom_minimum_size = MIN_SIZE
	var box := UiBuild.vbox(6)
	add_child(box)
	var header := UiBuild.hbox(8, box)
	title_label = UiBuild.label("Économie", UiType.size(UiType.HEADING))
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title_label)
	var close_button := UiBuild.button("×")
	TooltipHost.attach_plain(close_button, "close")
	close_button.pressed.connect(close)
	header.add_child(close_button)
	box.add_child(HSeparator.new())
	tabs = TabContainer.new()
	tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
	box.add_child(tabs)
	overview_tab = EconomyOverviewTab.new()
	trade_tab = EconomyTradeTab.new()
	currency_tab = EconomyCurrencyTab.new()
	for entry in [[TAB_OVERVIEW, overview_tab], [TAB_TRADE, trade_tab], [TAB_CURRENCY, currency_tab]]:
		var scroll := ScrollContainer.new()
		scroll.name = TAB_TITLES[entry[0]]
		scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
		var margin := UiBuild.margin(8, 8, 8, 8, scroll)
		entry[1].size_flags_horizontal = Control.SIZE_EXPAND_FILL
		margin.add_child(entry[1])
		margin.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		tabs.add_child(scroll)
	tabs.tab_changed.connect(func(_index: int) -> void: refresh())
	trade_tab.order_requested.connect(func(order: Dictionary, text: String) -> void: order_requested.emit(order, text))
	trade_tab.show_on_map_requested.connect(func() -> void: trade_layer_requested.emit())
	hide()


func _ready() -> void:
	if ResourceLoader.exists("res://scenes/ui/parchment_theme.tres"):
		theme = load("res://scenes/ui/parchment_theme.tres")


func setup(simulation: Object, faction: String) -> void:
	sim = simulation
	faction_id = faction


## Ouvre la fenêtre sur `tab` (`TAB_*`, "" = l'onglet courant) et relit les données.
func open(tab: String = "") -> void:
	if tab != "":
		show_tab(tab)
	refresh()
	show()


func close() -> void:
	hide()
	closed.emit()


func show_tab(tab: String) -> void:
	var index := TAB_ORDER.find(tab)
	if index >= 0:
		tabs.current_tab = index


func current_tab() -> String:
	return TAB_ORDER[clampi(tabs.current_tab, 0, TAB_ORDER.size() - 1)]


## Relit les données de l'onglet visible (les autres au moment où on les ouvre).
func refresh() -> void:
	if sim == null or faction_id == "":
		return
	match current_tab():
		TAB_OVERVIEW:
			overview_tab.show_data(sim, faction_id)
		TAB_TRADE:
			trade_tab.show_data(sim, faction_id)
		TAB_CURRENCY:
			currency_tab.show_data(sim, faction_id)
