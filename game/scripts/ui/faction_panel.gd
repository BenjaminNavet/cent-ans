class_name FactionPanel
extends PanelContainer

## Panneau parchemin de la faction (clic sur le blason/nom de la barre supérieure) :
## trésor, revenu du dernier tour, revenu prévisionnel, entretien armées/bâtiments,
## sélecteur d'impôt (Bas/Normal/Haut) et biens par catégorie. Les valeurs viennent de
## `CampaignSim.get_faction_economy` ; onglet vide si la simulation ne l'expose pas encore.

signal tax_rate_changed(faction_id: String, rate: String)
signal closed

const TAX_RATES := ["low", "normal", "high"]
const TAX_LABELS := {"low": "Bas", "normal": "Normal", "high": "Haut"}
const CATEGORY_LABELS := {
	"food": "Nourriture", "luxury": "Luxe", "raw_material": "Matières premières",
	"manufactured": "Manufacturé", "textile": "Textile", "metal": "Métal",
}

@onready var swatch: ColorRect = %Swatch
@onready var title_label: Label = %TitleLabel
@onready var treasury_value: Label = %TreasuryValue
@onready var income_value: Label = %IncomeValue
@onready var projected_value: Label = %ProjectedValue
@onready var army_upkeep_value: Label = %ArmyUpkeepValue
@onready var building_upkeep_value: Label = %BuildingUpkeepValue
@onready var administration_value: Label = %AdministrationValue
@onready var tax_low: Button = %TaxLow
@onready var tax_normal: Button = %TaxNormal
@onready var tax_high: Button = %TaxHigh
@onready var tax_note: Label = %TaxNote
@onready var goods_list: VBoxContainer = %GoodsList
@onready var close_button: Button = %CloseButton

var faction_id: String = ""
## H9 : ligne « Table » (régimes des provinces) ajoutée en code après l'entretien des bâtiments.
var table_upkeep_value: Label
## C5 : revenu des routes commerciales (n'entre pas dans `income`/`projected_income`, réglé
## après l'impôt) ajoutée en code après la Table.
var trade_income_value: Label
## H11 : postes de la monnaie (budget), sections Monnaie et Ordre de chevalerie, fenêtre des rançons.
var seigniorage_value: Label
var recoinage_value: Label
var coinage_section: CoinageSection
var chivalry_section: ChivalrySection
var feudal_section: FeudalSection  # FE6 : obligations et objectifs féodaux
## JR3 : section « Ferveur » (faction croisée seulement), posée sous le trésor.
var crusade_section: CrusadeSection
var ransom_button: Button
var ransom_panel: RansomPanel
## Audit A3 E1 : « Solde prévu » (le chiffre de la barre du haut, calculé par `core/`).
var net_value: Label
## Lot U3 : tableau recettes / dépenses / solde (prévu, saison passée, écart) et courbe du trésor.
var budget_table: BudgetTable
var treasury_chart: TreasuryChart
var _tax_buttons: Dictionary = {}
var _updating := false


func _ready() -> void:
	Lettrine.attach(title_label)  # UI1 : titre à lettrine enluminée
	_tax_buttons = {"low": tax_low, "normal": tax_normal, "high": tax_high}
	for rate in _tax_buttons:
		var button: Button = _tax_buttons[rate]
		button.pressed.connect(_on_tax_pressed.bind(rate))
	close_button.pressed.connect(func() -> void:
		hide()
		closed.emit())
	_add_table_row()
	_add_trade_row()
	_add_h11_sections()
	_arrange_budget()
	_build_budget_view()
	_add_crusade_section()
	_wrap_in_scroll()
	visibility_changed.connect(func() -> void:
		if not visible and ransom_panel != null:
			ransom_panel.hide())
	# F2 : infobulles du trésor et du revenu.
	for pair in [[treasury_value, "hud_treasury"], [income_value, ""], [projected_value, ""]]:
		var label: Label = pair[0]
		var tooltip := label.tooltip_text
		label.set_script(RichLabel)
		label.mouse_filter = Control.MOUSE_FILTER_PASS
		label.tooltip_text = RichTooltip.hud(pair[1]) if pair[1] != "" else tooltip


## `label` / `color` : identité de la faction (`SimFacade.faction_short_name` / `faction_color`).
## `economy` : `CampaignSim.get_faction_economy` (vide si absent de la simulation active).
func show_faction(id: String, label: String, color: Color, economy: Dictionary) -> void:
	faction_id = id
	title_label.text = label
	swatch.color = color
	if economy.is_empty():
		treasury_value.text = "—"
		income_value.text = "—"
		net_value.text = "—"
		projected_value.text = "—"
		army_upkeep_value.text = "—"
		building_upkeep_value.text = "—"
		administration_value.text = "—"
		table_upkeep_value.text = "—"
		trade_income_value.text = "—"
		seigniorage_value.text = "—"
		recoinage_value.text = "—"
		budget_table.show_budget({})
		treasury_chart.set_history([], 0, 0)
		_show_h11()
		tax_note.text = "Non disponible avec cette simulation."
		_set_tax_buttons_disabled(true)
		_fill_goods({}, [])
		feudal_section.refresh(_sim(), id)
		show()
		return
	treasury_value.text = Money.amount(int(economy.get("treasury", 0)))
	budget_table.show_budget(economy)
	var sim := _sim()
	var turn := int(sim.call("get_turn")) if sim != null and sim.has_method("get_turn") else 0
	treasury_chart.set_history(economy.get("budget_history", []), int(economy.get("treasury", 0)), turn)
	# Budget signé : recettes (+), charges (−), solde = barre du haut (tout vient de `core/`).
	var last_turn := int(economy.get("net_income_last_turn", 0))
	income_value.text = _signed(last_turn) if int(economy.get("income", 0)) != 0 or last_turn != 0 else "—"
	projected_value.text = _signed(int(economy.get("projected_income", 0)))
	army_upkeep_value.text = _charge(int(economy.get("army_upkeep", 0)))
	building_upkeep_value.text = _charge(int(economy.get("building_upkeep", 0)))
	administration_value.text = _charge(int(economy.get("administration_upkeep", 0)))
	var net := int(economy.get("net_income", 0))
	net_value.text = _signed(net)
	net_value.add_theme_color_override("font_color", Money.color_of(net))
	_show_table_upkeep(economy)
	_show_trade_income(economy)
	_show_h11(economy)
	_set_tax_buttons_disabled(false)
	var rate: String = str(economy.get("tax_rate", "normal"))
	_updating = true
	for r in _tax_buttons:
		(_tax_buttons[r] as Button).button_pressed = r == rate
	_updating = false
	# SV4 : multiplicateurs lus dans le cœur (`TaxRate::multiplier`).
	tax_note.text = RuleValues.format({
		"low": "×{rule.tax_multiplier_low:1} sur le revenu fiscal ; apaise le mécontentement.",
		"normal": "×{rule.tax_multiplier_normal:1} sur le revenu fiscal.",
		"high": "×{rule.tax_multiplier_high:1} sur le revenu fiscal ; augmente le mécontentement.",
	}.get(rate, ""))
	_fill_goods(economy.get("goods", {}), economy.get("goods_categories", []))
	feudal_section.refresh(_sim(), id)
	show()


func _add_table_row() -> void:
	var key := Label.new()
	key.text = "Table des provinces"
	table_upkeep_value = RichLabel.new()
	table_upkeep_value.text = "—"
	table_upkeep_value.mouse_filter = Control.MOUSE_FILTER_PASS
	building_upkeep_value.add_sibling(key)
	key.add_sibling(table_upkeep_value)


func _add_trade_row() -> void:
	var key := Label.new()
	key.text = "Commerce"
	trade_income_value = RichLabel.new()
	trade_income_value.text = "—"
	trade_income_value.mouse_filter = Control.MOUSE_FILTER_PASS
	table_upkeep_value.add_sibling(key)
	key.add_sibling(trade_income_value)


## Revenu des routes commerciales (projection courante) ; détail des routes en infobulle.
func _show_trade_income(economy: Dictionary) -> void:
	trade_income_value.text = "%s ℔" % Money.digits(int(economy.get("trade_income", 0)))
	var lines := PackedStringArray(["[b]Commerce[/b]", "Revenu des routes commerciales, réglé après l'impôt (lot C5).",
		"Saison passée : %s ℔" % Money.digits(int(economy.get("trade_income_last_turn", 0)))])
	var facade := get_node_or_null("/root/SimFacade")
	var sim: Object = facade.get("sim") if facade != null else null
	if sim != null and sim.has_method("get_trade_routes"):
		var routes: Array = sim.call("get_trade_routes")
		for route_variant in routes:
			var route: Dictionary = route_variant
			if str(route.get("from_faction", "")) != faction_id and str(route.get("to_faction", "")) != faction_id:
				continue
			if bool(route.get("cut", false)):
				lines.append("• %s ↔ %s : coupée (%s)" % [route["from_hub_name"], route["to_hub_name"], route["cut_reason"]])
			else:
				lines.append("• %s ↔ %s : %s ℔" % [route["from_hub_name"], route["to_hub_name"], Money.digits(int(route["total_value"]))])
	RichTooltip.attach_plain(trade_income_value, "trade_income_detail", {"body": "\n".join(lines)})


## `table_upkeep` (projection) ; détail par province (`get_table_budget`) en infobulle.
func _show_table_upkeep(economy: Dictionary) -> void:
	table_upkeep_value.text = _charge(int(economy.get("table_upkeep", 0)))
	var lines := PackedStringArray(["Régimes alimentaires payés chaque saison (inclus dans l'entretien).",
		"Saison passée : %s" % Money.amount(int(economy.get("table_upkeep_last_turn", 0)))])
	var facade := get_node_or_null("/root/SimFacade")
	var sim: Object = facade.get("sim") if facade != null else null
	if sim != null and sim.has_method("get_table_budget"):
		var budget: Dictionary = sim.call("get_table_budget", faction_id)
		var store: Object = facade.get("store")
		for row in budget.get("provinces", []):
			var province := str(row.get("province", ""))
			var info: Dictionary = store.call("get_province", province) if store != null else {}
			var diet: Dictionary = sim.call("get_province_diet", province)
			lines.append("• %s : %s — %s" % [str(info.get("display_name", province)), str(diet.get("name", row.get("diet", ""))), Money.amount(int(row.get("cost", 0)))])
	RichTooltip.attach_plain(table_upkeep_value, "table_upkeep_detail", {"title": "La Table", "body": "\n".join(lines)})


## H11 : lignes Seigneuriage / Refonte après la Table ; Monnaie, Ordre et bouton des rançons
## sous les biens. Tout est construit en code (la scène n'est pas modifiée).
func _add_h11_sections() -> void:
	var anchor: Control = trade_income_value
	for pair in [["Seigneuriage (revenu)", "seigniorage_value"], ["Refonte (administration)", "recoinage_value"]]:
		var key := Label.new()
		key.text = pair[0]
		var value := RichLabel.new()
		value.text = "—"
		value.mouse_filter = Control.MOUSE_FILTER_PASS
		anchor.add_sibling(key)
		key.add_sibling(value)
		set(pair[1], value)
		anchor = value
	coinage_section = CoinageSection.new()
	chivalry_section = ChivalrySection.new()
	ransom_button = RichButton.new()
	ransom_button.text = "Captifs et rançons"
	RichTooltip.attach_plain(ransom_button, "captives_and_ransoms")
	ransom_button.pressed.connect(toggle_ransoms)
	feudal_section = FeudalSection.new()
	for node in [HSeparator.new(), coinage_section, HSeparator.new(), chivalry_section, ransom_button, HSeparator.new(), feudal_section]:
		goods_list.get_parent().add_child(node)


## Audit A3 E1 : ordre du budget — recettes (dont seigneuriage), charges signées (dont
## refonte sous l'administration), puis le solde prévu et celui de la saison passée.
func _arrange_budget() -> void:
	var grid := income_value.get_parent()
	var net_key := Label.new()
	net_key.text = "Solde prévu de la saison"
	net_key.add_theme_font_size_override("font_size", UiType.size(UiType.BODY))
	net_value = RichLabel.new()
	net_value.text = "—"
	net_value.add_theme_font_size_override("font_size", UiType.size(UiType.BODY))
	net_value.mouse_filter = Control.MOUSE_FILTER_PASS
	RichTooltip.attach_plain(net_value, "budget_total_hint", {"title": "Solde"})
	grid.add_child(net_key)
	grid.add_child(net_value)
	(grid.get_node("ProjectedKey") as Label).text = "Recettes prévues"
	RichTooltip.attach_plain(projected_value, "budget_projected_income_hint")
	(grid.get_node("IncomeKey") as Label).text = "Solde de la saison passée"
	RichTooltip.attach_plain(income_value, "budget_last_income_hint")
	var order: Array[Control] = []
	for value: Control in [treasury_value, projected_value, seigniorage_value, army_upkeep_value,
			building_upkeep_value, table_upkeep_value, administration_value, recoinage_value,
			net_value, income_value]:
		order.append(grid.get_child(value.get_index() - 1))
		order.append(value)
	for index in order.size():
		grid.move_child(order[index], index)
	var seign_key: Label = grid.get_child(seigniorage_value.get_index() - 1)
	seign_key.text = "   dont seigneuriage"
	var recoin_key: Label = grid.get_child(recoinage_value.get_index() - 1)
	recoin_key.text = "   dont refonte des monnaies"


## Lot U3 : le budget passe en tableau (`BudgetTable`) suivi de la courbe du trésor ; la grille
## de la scène ne garde que la ligne « Trésor » (ses autres valeurs restent tenues à jour pour les
## tests et le tutoriel). Le panneau s'élargit pour les quatre colonnes.
func _build_budget_view() -> void:
	var grid: GridContainer = income_value.get_parent()
	for child in grid.get_children():
		if child != treasury_value and child != grid.get_child(treasury_value.get_index() - 1):
			(child as Control).hide()
	var treasury_key: Label = grid.get_child(treasury_value.get_index() - 1)
	treasury_key.add_theme_font_size_override("font_size", UiType.size(UiType.HEADING))
	treasury_value.add_theme_font_size_override("font_size", UiType.size(UiType.HEADING))
	var anchor: Control = grid
	var budget_title := _section_title("Budget de la saison")
	anchor.add_sibling(budget_title)
	budget_table = BudgetTable.new()
	budget_title.add_sibling(budget_table)
	var chart_title := _section_title("Trésor sur douze saisons")
	budget_table.add_sibling(chart_title)
	treasury_chart = TreasuryChart.new()
	treasury_chart.custom_minimum_size = Vector2(0, 104)
	chart_title.add_sibling(treasury_chart)
	custom_minimum_size.x = maxf(custom_minimum_size.x, PANEL_WIDTH)
	offset_left = offset_right - PANEL_WIDTH


const PANEL_WIDTH := 500.0


## JR3 : la Ferveur tient lieu d'assise à la faction croisée ; sa section vient juste sous le
## trésor, avant le budget (masquée pour toute autre faction).
func _add_crusade_section() -> void:
	var grid: Control = income_value.get_parent()
	crusade_section = CrusadeSection.new()
	grid.add_sibling(crusade_section)
	var rule := HSeparator.new()
	rule.name = "CrusadeRule"
	crusade_section.add_sibling(rule)
	rule.visible = false
	crusade_section.visibility_changed.connect(func() -> void: rule.visible = crusade_section.visible)


func _section_title(text: String) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", UiType.size(UiType.BODY))
	label.add_theme_color_override("font_color", Color(0.40, 0.22, 0.10))
	return label


static func _charge(value: int) -> String:
	return Money.charge(value)


## H11 : le contenu (scène `VBox`) passe dans un défilement vertical borné à la hauteur de
## l'écran, les sections Monnaie et Ordre allongeant le panneau.
var _scroll: ScrollContainer
## CV3-0 (#9) : réserve en bas d'écran (HUD bas : cloche de fin de saison, sceau...) posée par
## `MapUI.layout_hud` (`update_bottom_reserve`) ; 24 par défaut si personne ne la pose (tests).
var bottom_reserved_px: float = 24.0


func _wrap_in_scroll() -> void:
	var body: Control = get_node("VBox")
	remove_child(body)
	_scroll = ScrollContainer.new()
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	add_child(_scroll)
	_scroll.add_child(body)
	body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	visibility_changed.connect(_fit_height)
	# CV3-0 (#9) : le panneau débordait de l'écran après un redimensionnement de la fenêtre
	# (la hauteur n'était recalculée qu'à l'ouverture / au rafraîchissement du contenu).
	get_viewport().size_changed.connect(_fit_height)


## CV3-0 (#9) : posé par `MapUI` à chaque `layout_hud` (résultat de `EndTurnCluster.bell_height`,
## qui grandit avec les alertes) ; recalcule aussitôt si le panneau est ouvert.
func update_bottom_reserve(px: float) -> void:
	if is_equal_approx(bottom_reserved_px, px):
		return
	bottom_reserved_px = px
	_fit_height()


func _fit_height() -> void:
	if _scroll == null or not visible:
		return
	var body: Control = _scroll.get_child(0)
	# CV3-0 (#9) : les marges du stylebox (haut + bas du cadre parchemin) s'ajoutent à la
	# hauteur du défilement pour former la hauteur totale du panneau ; il faut les soustraire
	# de la place disponible, sans quoi le panneau déborde d'autant sous l'écran.
	var style := get_theme_stylebox("panel")
	var chrome := (style.get_margin(SIDE_TOP) + style.get_margin(SIDE_BOTTOM)) if style != null else 0.0
	var limit := maxf(get_viewport_rect().size.y - global_position.y - chrome - bottom_reserved_px, 40.0)
	_scroll.custom_minimum_size = Vector2(body.get_combined_minimum_size().x, minf(body.get_combined_minimum_size().y, limit))
	reset_size()


func _show_h11(economy: Dictionary = {}) -> void:
	var player := _player_faction()
	var is_player := faction_id == player or player == ""
	if not economy.is_empty():
		var seigniorage := int(economy.get("seigniorage", 0))
		seigniorage_value.text = Money.signed(seigniorage) if seigniorage > 0 else Money.amount(seigniorage)
		RichTooltip.attach_plain(seigniorage_value, "seigniorage_detail", {"body": "Profit du monnayage prévu cette saison (inclus dans le revenu prévisionnel).\nSaison passée : %s" % Money.amount(int(economy.get("seigniorage_last_turn", 0)))})
		recoinage_value.text = _charge(int(economy.get("recoinage", 0)))
		RichTooltip.attach_plain(recoinage_value, "recoinage_detail", {"body": "Coût de la monnaie forte prévu cette saison (inclus dans l'administration).\nSaison passée : %s" % Money.amount(int(economy.get("recoinage_last_turn", 0)))})
	coinage_section.show_for(faction_id, is_player)
	chivalry_section.show_for(is_player)
	crusade_section.show_for(is_player)
	var sim := _sim()
	ransom_button.visible = is_player and sim != null and sim.has_method("get_ransoms")
	if ransom_button.visible:
		var ransoms: Dictionary = sim.call("get_ransoms")
		var count: int = (ransoms.get("ours", []) as Array).size() + (ransoms.get("held", []) as Array).size()
		ransom_button.text = "Captifs et rançons (%d)" % count if count > 0 else "Captifs et rançons"
	_fit_height.call_deferred()


## Ouvre ou ferme la fenêtre des rançons, posée à gauche du panneau (même calque).
func toggle_ransoms() -> void:
	if ransom_panel == null:
		ransom_panel = RansomPanel.new()
		get_parent().add_child(ransom_panel)
		ransom_panel.ransoms_changed.connect(func() -> void: _show_h11())
	elif ransom_panel.visible:
		ransom_panel.close()
		return
	ransom_panel.refresh(_sim())
	ransom_panel.position = Vector2(maxf(8.0, global_position.x - ransom_panel.size.x - 8.0), global_position.y)


func _sim() -> Object:
	var facade := get_node_or_null("/root/SimFacade")
	return facade.get("sim") if facade != null else null


func _player_faction() -> String:
	var sim := _sim()
	return str(sim.call("get_player_faction")) if sim != null and sim.has_method("get_player_faction") else ""


func _set_tax_buttons_disabled(disabled: bool) -> void:
	for rate in _tax_buttons:
		(_tax_buttons[rate] as Button).disabled = disabled


func _on_tax_pressed(rate: String) -> void:
	if _updating:
		return
	tax_rate_changed.emit(faction_id, rate)


func _fill_goods(goods: Dictionary, categories: Array) -> void:
	for child in goods_list.get_children():
		child.queue_free()
	if categories.is_empty():
		var label := Label.new()
		label.text = "—"
		goods_list.add_child(label)
		return
	for category in categories:
		var line := Label.new()
		line.text = "• %s" % str(CATEGORY_LABELS.get(category, str(category).capitalize()))
		goods_list.add_child(line)
	# F2 : une puce (icône + nom + quantité) par ressource, infobulle riche.
	var flow := HFlowContainer.new()
	flow.add_theme_constant_override("h_separation", 10)
	for res_id in goods:
		var id := str(res_id)
		var text := "%s (%d)" % [GameCatalog.display_name(id), int(goods[res_id])]
		flow.add_child(IconChip.create(id, text, RichTooltip.resource(id, int(goods[res_id])), 20.0, 13, "resource"))
	goods_list.add_child(flow)


static func _signed(value: int) -> String:
	return Money.signed(value)

