class_name ArmyPanel
extends PanelContainer

## Panneau parchemin de l'armée sélectionnée : général, position, points de mouvement,
## ravitaillement, ordre en cours, posture (Normale / Chevauchée / Siège) et liste des
## unités (nom, effectif/max, moral). Les valeurs viennent de `CampaignSim.get_army`.

signal stance_changed(army_id: String, stance: String)
signal closed

const STANCES := ["normal", "raid", "siege"]
const STANCE_LABELS := ["Normale", "Chevauchée", "Siège"]

@onready var swatch: ColorRect = %Swatch
@onready var title_label: Label = %TitleLabel
@onready var general_value: Label = %GeneralValue
@onready var location_value: Label = %LocationValue
@onready var movement_value: Label = %MovementValue
@onready var supply_value: Label = %SupplyValue
@onready var path_value: Label = %PathValue
@onready var stance_option: OptionButton = %StanceOption
@onready var units_header: Label = %UnitsHeader
@onready var units_list: VBoxContainer = %UnitsList
@onready var hint: Label = %Hint
@onready var close_button: Button = %CloseButton

var army_id: String = ""
var _updating := false


func _ready() -> void:
	for label in STANCE_LABELS:
		stance_option.add_item(label)
	stance_option.item_selected.connect(_on_stance_selected)
	close_button.pressed.connect(func() -> void:
		hide()
		closed.emit())


## `province_name_of(id) -> String` traduit les ids de province en noms affichables.
func show_army(id: String, army: Dictionary, faction_label: String, color: Color, is_player: bool, province_name_of: Callable) -> void:
	if army.is_empty():
		hide()
		return
	army_id = id
	_updating = true
	swatch.color = color
	var general_name: String = str(army.get("general_name", ""))
	title_label.text = "Armée de %s" % faction_label
	general_value.text = general_name if general_name != "" else "Aucun"
	location_value.text = str(province_name_of.call(str(army.get("location", ""))))
	movement_value.text = "%d point(s)" % int(army.get("movement_points", 0))
	supply_value.text = "%d %%" % int(army.get("supply", 0))
	var path: Array = army.get("path", [])
	if path.is_empty():
		path_value.text = "Aucun"
	else:
		var names := PackedStringArray()
		for step in path:
			names.append(str(province_name_of.call(str(step))))
		path_value.text = "→ " + " → ".join(names)
	var stance_index := STANCES.find(str(army.get("stance", "normal")))
	stance_option.select(maxi(stance_index, 0))
	stance_option.disabled = not is_player
	hint.visible = is_player
	for child in units_list.get_children():
		child.queue_free()
	var units: Array = army.get("units", [])
	var total := 0
	for unit in units:
		total += int(unit.get("strength", 0))
		var label := Label.new()
		label.text = "• %s — %d/%d, moral %d" % [
			str(unit.get("name", unit.get("unit_type", "?"))), int(unit.get("strength", 0)),
			int(unit.get("max_strength", 0)), int(unit.get("morale", 0))]
		units_list.add_child(label)
	units_header.text = "Unités (%d, %d hommes)" % [units.size(), total]
	_updating = false
	show()


func _on_stance_selected(index: int) -> void:
	if _updating or army_id == "":
		return
	stance_changed.emit(army_id, STANCES[clampi(index, 0, STANCES.size() - 1)])
