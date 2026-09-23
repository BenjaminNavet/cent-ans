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
	# F2 : infobulles du trésor et du revenu.
	for pair in [[treasury_value, "hud_treasury"], [income_value, "hud_income"], [projected_value, "hud_income"]]:
		var label: Label = pair[0]
		label.set_script(RichLabel)
		label.mouse_filter = Control.MOUSE_FILTER_PASS
		label.tooltip_text = RichTooltip.hud(pair[1])


## `label` / `color` : identité de la faction (`SimFacade.faction_short_name` / `faction_color`).
## `economy` : `CampaignSim.get_faction_economy` (vide si absent de la simulation active).
func show_faction(id: String, label: String, color: Color, economy: Dictionary) -> void:
	faction_id = id
	title_label.text = label
	swatch.color = color
	if economy.is_empty():
		treasury_value.text = "—"
		income_value.text = "—"
		projected_value.text = "—"
		army_upkeep_value.text = "—"
		building_upkeep_value.text = "—"
		administration_value.text = "—"
		tax_note.text = "Non disponible avec cette simulation."
		_set_tax_buttons_disabled(true)
		_fill_goods({}, [])
		show()
		return
	treasury_value.text = "%s ℔" % _thousands(int(economy.get("treasury", 0)))
	income_value.text = _signed(int(economy.get("income", 0)))
	projected_value.text = _signed(int(economy.get("projected_income", 0)))
	army_upkeep_value.text = "%s ℔" % _thousands(int(economy.get("army_upkeep", 0)))
	building_upkeep_value.text = "%s ℔" % _thousands(int(economy.get("building_upkeep", 0)))
	administration_value.text = "%s ℔" % _thousands(int(economy.get("administration_upkeep", 0)))
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
		line.text = "• %s" % str(CATEGORY_LABELS.get(category, category)).capitalize()
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
	return "%s%s ℔" % ["+" if value >= 0 else "", _thousands(value)]


static func _thousands(value: int) -> String:
	var text := str(absi(value))
	var out := ""
	while text.length() > 3:
		out = " " + text.substr(text.length() - 3) + out
		text = text.substr(0, text.length() - 3)
	return ("-" if value < 0 else "") + text + out
