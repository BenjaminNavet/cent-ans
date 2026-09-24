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
## H11 : postes de la monnaie (budget), sections Monnaie et Ordre de chevalerie, fenêtre des rançons.
var seigniorage_value: Label
var recoinage_value: Label
var coinage_section: CoinageSection
var chivalry_section: ChivalrySection
var ransom_button: Button
var ransom_panel: RansomPanel
## Audit A3 E1 : « Solde prévu » (le chiffre de la barre du haut, calculé par `core/`).
var net_value: Label
var _tax_buttons: Dictionary = {}
var _updating := false


func _ready() -> void:
	_tax_buttons = {"low": tax_low, "normal": tax_normal, "high": tax_high}
	for rate in _tax_buttons:
		var button: Button = _tax_buttons[rate]
		button.pressed.connect(_on_tax_pressed.bind(rate))
	close_button.pressed.connect(func() -> void:
		hide()
		closed.emit())
	_add_table_row()
	_add_h11_sections()
	_arrange_budget()
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
		seigniorage_value.text = "—"
		recoinage_value.text = "—"
		_show_h11()
		tax_note.text = "Non disponible avec cette simulation."
		_set_tax_buttons_disabled(true)
		_fill_goods({}, [])
		show()
		return
	treasury_value.text = "%s ℔" % _thousands(int(economy.get("treasury", 0)))
	# Budget signé : recettes (+), charges (−), solde = barre du haut (tout vient de `core/`).
	var last_turn := int(economy.get("net_income_last_turn", 0))
	income_value.text = _signed(last_turn) if int(economy.get("income", 0)) != 0 or last_turn != 0 else "—"
	projected_value.text = _signed(int(economy.get("projected_income", 0)))
	army_upkeep_value.text = _charge(int(economy.get("army_upkeep", 0)))
	building_upkeep_value.text = _charge(int(economy.get("building_upkeep", 0)))
	administration_value.text = _charge(int(economy.get("administration_upkeep", 0)))
	var net := int(economy.get("net_income", 0))
	net_value.text = _signed(net)
	net_value.add_theme_color_override("font_color", Color(0.55, 0.12, 0.10) if net < 0 else Color(0.22, 0.14, 0.07))
	_show_table_upkeep(economy)
	_show_h11(economy)
	_set_tax_buttons_disabled(false)
	var rate: String = str(economy.get("tax_rate", "normal"))
	_updating = true
	for r in _tax_buttons:
		(_tax_buttons[r] as Button).button_pressed = r == rate
	_updating = false
	tax_note.text = {
		"low": "×0,7 sur le revenu fiscal ; apaise le mécontentement.",
		"normal": "×1,0 sur le revenu fiscal.",
		"high": "×1,4 sur le revenu fiscal ; augmente le mécontentement.",
	}.get(rate, "")
	_fill_goods(economy.get("goods", {}), economy.get("goods_categories", []))
	show()


func _add_table_row() -> void:
	var key := Label.new()
	key.text = "Table des provinces"
	table_upkeep_value = RichLabel.new()
	table_upkeep_value.text = "—"
	table_upkeep_value.mouse_filter = Control.MOUSE_FILTER_PASS
	building_upkeep_value.add_sibling(key)
	key.add_sibling(table_upkeep_value)


## `table_upkeep` (projection) ; détail par province (`get_table_budget`) en infobulle.
func _show_table_upkeep(economy: Dictionary) -> void:
	table_upkeep_value.text = _charge(int(economy.get("table_upkeep", 0)))
	var lines := PackedStringArray(["[b]La Table[/b]", "Régimes alimentaires payés chaque saison (inclus dans l'entretien).",
		"Saison passée : %s ℔" % _thousands(int(economy.get("table_upkeep_last_turn", 0)))])
	var facade := get_node_or_null("/root/SimFacade")
	var sim: Object = facade.get("sim") if facade != null else null
	if sim != null and sim.has_method("get_table_budget"):
		var budget: Dictionary = sim.call("get_table_budget", faction_id)
		var store: Object = facade.get("store")
		for row in budget.get("provinces", []):
			var province := str(row.get("province", ""))
			var info: Dictionary = store.call("get_province", province) if store != null else {}
			var diet: Dictionary = sim.call("get_province_diet", province)
			lines.append("• %s : %s — %s ℔" % [str(info.get("display_name", province)), str(diet.get("name", row.get("diet", ""))), _thousands(int(row.get("cost", 0)))])
	table_upkeep_value.tooltip_text = "\n".join(lines)


## H11 : lignes Seigneuriage / Refonte après la Table ; Monnaie, Ordre et bouton des rançons
## sous les biens. Tout est construit en code (la scène n'est pas modifiée).
func _add_h11_sections() -> void:
	var anchor: Control = table_upkeep_value
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
	ransom_button.tooltip_text = "Nos captifs, nos prisonniers et les dettes de rançon."
	ransom_button.pressed.connect(toggle_ransoms)
	for node in [HSeparator.new(), coinage_section, HSeparator.new(), chivalry_section, ransom_button]:
		goods_list.get_parent().add_child(node)


## Audit A3 E1 : ordre du budget — recettes (dont seigneuriage), charges signées (dont
## refonte sous l'administration), puis le solde prévu et celui de la saison passée.
func _arrange_budget() -> void:
	var grid := income_value.get_parent()
	var net_key := Label.new()
	net_key.text = "Solde prévu de la saison"
	net_key.add_theme_font_size_override("font_size", 17)
	net_value = RichLabel.new()
	net_value.text = "—"
	net_value.add_theme_font_size_override("font_size", 17)
	net_value.mouse_filter = Control.MOUSE_FILTER_PASS
	net_value.tooltip_text = "Recettes moins toutes les charges : le « Solde » de la barre du haut, ajouté au trésor en fin de tour."
	grid.add_child(net_key)
	grid.add_child(net_value)
	(grid.get_node("ProjectedKey") as Label).text = "Recettes prévues"
	projected_value.tooltip_text = "Impôts, commerce et seigneuriage attendus à la prochaine fin de tour, avant les charges."
	(grid.get_node("IncomeKey") as Label).text = "Solde de la saison passée"
	income_value.tooltip_text = "Ce qui a réellement été ajouté au trésor (ou retiré) à la dernière fin de tour."
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


static func _charge(value: int) -> String:
	return "−%s ℔" % _thousands(absi(value)) if value != 0 else "0 ℔"


## H11 : le contenu (scène `VBox`) passe dans un défilement vertical borné à la hauteur de
## l'écran, les sections Monnaie et Ordre allongeant le panneau.
var _scroll: ScrollContainer


func _wrap_in_scroll() -> void:
	var body: Control = get_node("VBox")
	remove_child(body)
	_scroll = ScrollContainer.new()
	_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	add_child(_scroll)
	_scroll.add_child(body)
	body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	visibility_changed.connect(_fit_height)


func _fit_height() -> void:
	if _scroll == null or not visible:
		return
	var body: Control = _scroll.get_child(0)
	var limit := get_viewport_rect().size.y - position.y - 24.0
	_scroll.custom_minimum_size = Vector2(body.get_combined_minimum_size().x, minf(body.get_combined_minimum_size().y, limit))
	reset_size()


func _show_h11(economy: Dictionary = {}) -> void:
	var player := _player_faction()
	var is_player := faction_id == player or player == ""
	if not economy.is_empty():
		var seigniorage := int(economy.get("seigniorage", 0))
		seigniorage_value.text = "%s%s ℔" % ["+" if seigniorage > 0 else "", _thousands(seigniorage)]
		seigniorage_value.tooltip_text = "[b]Seigneuriage[/b]\nProfit du monnayage prévu cette saison (inclus dans le revenu prévisionnel).\nSaison passée : %s ℔" % _thousands(int(economy.get("seigniorage_last_turn", 0)))
		recoinage_value.text = _charge(int(economy.get("recoinage", 0)))
		recoinage_value.tooltip_text = "[b]Refonte des espèces[/b]\nCoût de la monnaie forte prévu cette saison (inclus dans l'administration).\nSaison passée : %s ℔" % _thousands(int(economy.get("recoinage_last_turn", 0)))
	coinage_section.show_for(faction_id, is_player)
	chivalry_section.show_for(is_player)
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
	return "%s%s ℔" % ["+" if value >= 0 else "−", _thousands(absi(value))]


static func _thousands(value: int) -> String:
	var text := str(absi(value))
	var out := ""
	while text.length() > 3:
		out = " " + text.substr(text.length() - 3) + out
		text = text.substr(0, text.length() - 3)
	return ("-" if value < 0 else "") + text + out
