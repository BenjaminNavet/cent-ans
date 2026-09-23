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
@onready var general_skills_value: Label = %GeneralSkillsValue
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
	# F2 : infobulles explicatives des valeurs de l'armée.
	for pair in [[movement_value, "movement"], [supply_value, "supply"]]:
		var label: Label = pair[0]
		label.set_script(RichLabel)
		label.mouse_filter = Control.MOUSE_FILTER_PASS
		label.tooltip_text = RichTooltip.gauge(pair[1])
	general_skills_value.set_script(RichLabel)
	general_skills_value.mouse_filter = Control.MOUSE_FILTER_PASS
	general_skills_value.tooltip_text = "\n".join([RichTooltip.branch("command"), RichTooltip.branch("governance"), RichTooltip.branch("court")])


## `province_name_of(id) -> String` traduit les ids de province en noms affichables.
## `general_skills` : `CampaignSim.get_character(army.general).skills` si disponible
## (§ 3), vide sinon (« Compétences du général » reste à « — »).
func show_army(id: String, army: Dictionary, faction_label: String, color: Color, is_player: bool, province_name_of: Callable, general_skills: Dictionary = {}) -> void:
	if army.is_empty():
		hide()
		return
	army_id = id
	_updating = true
	swatch.color = color
	var general_name: String = str(army.get("general_name", ""))
	title_label.text = "Armée de %s" % faction_label
	general_value.text = general_name if general_name != "" else "Aucun"
	if general_skills.is_empty():
		general_skills_value.text = "—"
	else:
		general_skills_value.text = "Cdt %d / Gouv %d / Cour %d" % [
			int(general_skills.get("command", 0)), int(general_skills.get("governance", 0)), int(general_skills.get("court", 0))]
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
	# F2 : unités en cartes (icône, nom, effectif, barres effectif/moral, infobulle riche).
	var flow := HFlowContainer.new()
	flow.add_theme_constant_override("h_separation", 6)
	flow.add_theme_constant_override("v_separation", 6)
	units_list.add_child(flow)
	for unit in units:
		total += int(unit.get("strength", 0))
		flow.add_child(make_unit_card(unit))
	units_header.text = "Unités (%d, %d hommes)" % [units.size(), total]
	_updating = false
	show()


## Carte d'unité (F2) : `unit` de `get_army().units` (`unit_type`, `name`, `strength`,
## `max_strength`, `morale`).
static func make_unit_card(unit: Dictionary) -> Control:
	var unit_type: String = str(unit.get("unit_type", ""))
	var card := RichPanel.new()
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.97, 0.93, 0.82)
	style.border_color = Color(0.42, 0.29, 0.16)
	style.set_border_width_all(1)
	style.set_corner_radius_all(3)
	style.set_content_margin_all(4)
	card.add_theme_stylebox_override("panel", style)
	card.custom_minimum_size = Vector2(118, 0)
	card.mouse_filter = Control.MOUSE_FILTER_STOP
	card.tooltip_text = RichTooltip.unit(unit_type, unit)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 1)
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	card.add_child(box)
	var head := HBoxContainer.new()
	head.mouse_filter = Control.MOUSE_FILTER_IGNORE
	head.add_child(IconLibrary.make_rect(unit_type, 30.0, "unit"))
	var name_label := Label.new()
	name_label.text = ProvincePanel.unit_label(unit)
	name_label.add_theme_font_size_override("font_size", 12)
	name_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	name_label.custom_minimum_size = Vector2(76, 0)
	name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	head.add_child(name_label)
	box.add_child(head)
	var count := Label.new()
	count.text = "%d / %d · moral %d" % [int(unit.get("strength", 0)), int(unit.get("max_strength", 0)), int(unit.get("morale", 0))]
	count.add_theme_font_size_override("font_size", 11)
	box.add_child(count)
	var ratio := float(unit.get("strength", 0)) / maxf(1.0, float(unit.get("max_strength", 1)))
	box.add_child(_mini_bar(ratio, Color(0.55, 0.20, 0.15)))
	box.add_child(_mini_bar(float(unit.get("morale", 0)) / 100.0, Color(0.25, 0.45, 0.8)))
	for child in box.get_children():
		(child as Control).mouse_filter = Control.MOUSE_FILTER_IGNORE
	return card


static func _mini_bar(ratio: float, color: Color) -> Control:
	var track := ColorRect.new()
	track.color = Color(0.55, 0.50, 0.40)
	track.custom_minimum_size = Vector2(108, 4)
	track.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var fill := ColorRect.new()
	fill.color = color
	fill.size = Vector2(108.0 * clampf(ratio, 0.0, 1.0), 4.0)
	fill.mouse_filter = Control.MOUSE_FILTER_IGNORE
	track.add_child(fill)
	return track


func _on_stance_selected(index: int) -> void:
	if _updating or army_id == "":
		return
	stance_changed.emit(army_id, STANCES[clampi(index, 0, STANCES.size() - 1)])
