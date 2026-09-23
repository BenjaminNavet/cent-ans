class_name ProvincePanel
extends PanelContainer

## Panneau parchemin de la province sélectionnée : identité (`GameDataStore`), état de
## campagne (`CampaignSim` : propriétaire réel, contrôleur, garnison, siège, mécontentement,
## dévastation) et actions du joueur (recrutement, formation d'une armée depuis la garnison).
## Aucune règle ici : les listes recrutables et les refus viennent de la simulation.

signal recruit_requested(province_id: String, unit_type: String)
signal create_army_requested(province_id: String, unit_indices: Array)
signal closed

const TERRAIN_LABELS := {
	"plains": "Plaines", "hills": "Collines", "mountains": "Montagnes",
	"forest": "Forêt", "marsh": "Marais", "coast": "Littoral", "highlands": "Hautes terres",
}
const STANCE_LABELS := {"normal": "Normale", "raid": "Chevauchée", "siege": "Siège"}

@onready var name_label: Label = %NameLabel
@onready var owner_value: Label = %OwnerValue
@onready var controller_value: Label = %ControllerValue
@onready var capital_value: Label = %CapitalValue
@onready var terrain_value: Label = %TerrainValue
@onready var population_value: Label = %PopulationValue
@onready var unrest_value: Label = %UnrestValue
@onready var devastation_value: Label = %DevastationValue
@onready var siege_value: Label = %SiegeValue
@onready var id_value: Label = %IdValue
@onready var garrison_header: Label = %GarrisonHeader
@onready var garrison_list: VBoxContainer = %GarrisonList
@onready var actions: HBoxContainer = %Actions
@onready var recruit_button: Button = %RecruitButton
@onready var create_army_button: Button = %CreateArmyButton
@onready var recruit_panel: VBoxContainer = %RecruitPanel
@onready var recruit_list: VBoxContainer = %RecruitList
@onready var close_button: Button = %CloseButton

var province_id: String = ""
var _garrison_checks: Array[CheckBox] = []


func _ready() -> void:
	recruit_button.pressed.connect(func() -> void: recruit_panel.visible = not recruit_panel.visible)
	create_army_button.pressed.connect(_on_create_army)
	close_button.pressed.connect(func() -> void:
		hide()
		closed.emit())


## `province` : entrée MapData fusionnée avec `GameDataStore.get_province` (display_name,
## capital, terrain, owner_display_name…). `state` : `CampaignSim.get_province_state`.
## `recruitable` : `get_recruitable` (vide si la province n'est pas au joueur).
## `label_of(faction_id) -> String` traduit un id de faction en nom court.
func show_province(province: Dictionary, state: Dictionary = {}, recruitable: Array = [], is_player_owner: bool = false, label_of: Callable = Callable()) -> void:
	if province.is_empty():
		hide()
		return
	province_id = str(province.get("id", ""))
	name_label.text = str(province.get("display_name", province.get("name", "?")))
	var owner: String = str(state.get("owner", province.get("owner", "")))
	owner_value.text = _faction_label(owner, province.get("owner_display_name", ""), label_of)
	var controller: String = str(state.get("controller", owner))
	controller_value.text = _faction_label(controller, "", label_of) if controller != owner else "—"
	var capital: String = str(province.get("capital", province.get("capital_name", "")))
	capital_value.text = capital if capital != "" else "—"
	var terrain: String = str(province.get("terrain", ""))
	terrain_value.text = TERRAIN_LABELS.get(terrain, terrain if terrain != "" else "—")
	var population := int(state.get("population_total", province.get("population_total", 0)))
	population_value.text = _thousands(population) if population > 0 else "—"
	unrest_value.text = ("%d %%" % int(state["unrest"])) if state.has("unrest") else "—"
	devastation_value.text = ("%d %%" % int(state["devastation"])) if state.has("devastation") else "—"
	var siege: Dictionary = state.get("siege", {}) if state.get("siege") is Dictionary else {}
	if siege.is_empty():
		siege_value.text = "Aucun"
	else:
		siege_value.text = "%s, %d tour(s)" % [_faction_label(str(siege.get("attacker", "")), "", label_of), int(siege.get("turns_left", 0))]
	id_value.text = "%s (index %d)" % [province_id, int(province.get("index", 0))]
	_fill_garrison(state.get("garrison", []), is_player_owner)
	_fill_recruitable(recruitable)
	actions.visible = is_player_owner and not state.is_empty()
	if not actions.visible:
		recruit_panel.hide()
	show()


func _fill_garrison(garrison: Array, selectable: bool) -> void:
	for child in garrison_list.get_children():
		child.queue_free()
	_garrison_checks.clear()
	garrison_header.text = "Garnison (%d unité%s)" % [garrison.size(), "s" if garrison.size() > 1 else ""]
	for i in garrison.size():
		var unit: Dictionary = garrison[i]
		var text := "%s — %d/%d, moral %d" % [
			unit_label(unit), int(unit.get("strength", 0)),
			int(unit.get("max_strength", 0)), int(unit.get("morale", 0))]
		if selectable:
			var check := CheckBox.new()
			check.text = text
			check.button_pressed = true
			garrison_list.add_child(check)
			_garrison_checks.append(check)
		else:
			var label := Label.new()
			label.text = "• " + text
			garrison_list.add_child(label)
	create_army_button.disabled = garrison.is_empty()


func _fill_recruitable(recruitable: Array) -> void:
	for child in recruit_list.get_children():
		child.queue_free()
	recruit_button.disabled = recruitable.is_empty()
	for row in recruitable:
		var line := HBoxContainer.new()
		var button := Button.new()
		button.text = "%s — %s ℔ / %s ℔" % [str(row.get("name", row.get("unit_type", "?"))), _thousands(int(row.get("cost", 0))), _thousands(int(row.get("upkeep", 0)))]
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		var available: bool = bool(row.get("available", false))
		button.disabled = not available
		var unit_type: String = str(row.get("unit_type", ""))
		button.pressed.connect(func() -> void: recruit_requested.emit(province_id, unit_type))
		line.add_child(button)
		if not available:
			var reason := Label.new()
			reason.text = str(row.get("reason", "Indisponible"))
			reason.add_theme_font_size_override("font_size", 13)
			reason.add_theme_color_override("font_color", Color(0.55, 0.20, 0.15))
			button.tooltip_text = reason.text
			line.add_child(reason)
		recruit_list.add_child(line)


func _on_create_army() -> void:
	var indices: Array = []
	for i in _garrison_checks.size():
		if _garrison_checks[i].button_pressed:
			indices.append(i)
	if indices.is_empty():
		return
	create_army_requested.emit(province_id, indices)


## Nom d'unité : `name` si la simulation le fournit, sinon l'id rendu lisible.
static func unit_label(unit: Dictionary) -> String:
	var name: String = str(unit.get("name", ""))
	if name != "":
		return name
	return str(unit.get("unit_type", "?")).trim_prefix("unit_").capitalize()


static func _faction_label(faction_id: String, display_name: String, label_of: Callable) -> String:
	if faction_id == "":
		return "—"
	if display_name != "":
		return display_name
	if label_of.is_valid():
		return str(label_of.call(faction_id))
	return faction_id


static func _thousands(value: int) -> String:
	var text := str(absi(value))
	var out := ""
	while text.length() > 3:
		out = " " + text.substr(text.length() - 3) + out
		text = text.substr(0, text.length() - 3)
	return ("-" if value < 0 else "") + text + out
