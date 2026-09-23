class_name ProvincePanel
extends PanelContainer

## Panneau parchemin de la province sélectionnée : identité (`GameDataStore`), état de
## campagne (`CampaignSim` : propriétaire réel, contrôleur, garnison, siège, mécontentement,
## dévastation) et actions du joueur (recrutement, formation d'une armée depuis la garnison).
## Aucune règle ici : les listes recrutables et les refus viennent de la simulation.

signal recruit_requested(province_id: String, unit_type: String)
signal create_army_requested(province_id: String, unit_indices: Array)
signal build_requested(province_id: String, building_id: String)
signal cancel_build_requested(province_id: String)
signal court_requested
signal closed

const TERRAIN_LABELS := {
	"plains": "Plaines", "hills": "Collines", "mountains": "Montagnes",
	"forest": "Forêt", "marsh": "Marais", "coast": "Littoral", "highlands": "Hautes terres",
}
const STANCE_LABELS := {"normal": "Normale", "raid": "Chevauchée", "siege": "Siège"}
const CLASS_LABELS := {"peasants": "Paysans", "burghers": "Bourgeois", "clergy": "Clergé", "nobility": "Noblesse"}
## Jauge → (nom court, vrai si une valeur haute est mauvaise : mécontentement).
const GAUGE_SPECS := [
	["unrest", "Mécont.", true],
	["health", "Santé", false],
	["wealth", "Richesse", false],
	["goods_satisfaction", "Biens", false],
]
const RESOURCE_CATEGORY_LABELS := {
	"food": "Nourriture", "luxury": "Luxe", "raw_material": "Matières premières",
	"manufactured": "Manufacturé", "textile": "Textile", "metal": "Métal",
}
## `get_province_city().resources` ne donne que des ids (§ 2) : noms français locaux pour
## les ressources connues (`data/resources/*.json`), repli sur l'id mis en forme sinon.
const RESOURCE_NAMES := {
	"res_wheat": "Blé", "res_wine": "Vin", "res_stone": "Pierre", "res_wood": "Bois",
	"res_wool": "Laine", "res_iron": "Fer", "res_salt": "Sel", "res_fish": "Poisson",
	"res_cloth": "Drap",
}

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
@onready var tabs: TabContainer = %Tabs
@onready var resources_list: HFlowContainer = %ResourcesList
@onready var classes_list: VBoxContainer = %ClassesList
@onready var buildings_list: VBoxContainer = %BuildingsList
@onready var construction_box: VBoxContainer = %ConstructionBox
@onready var construction_label: Label = %ConstructionLabel
@onready var cancel_build_button: Button = %CancelBuildButton
@onready var buildable_list: VBoxContainer = %BuildableList
@onready var governor_label: Label = %GovernorLabel
@onready var governor_court_button: Button = %GovernorCourtButton

var province_id: String = ""
var _garrison_checks: Array[CheckBox] = []


func _ready() -> void:
	recruit_button.pressed.connect(func() -> void: recruit_panel.visible = not recruit_panel.visible)
	create_army_button.pressed.connect(_on_create_army)
	cancel_build_button.pressed.connect(func() -> void: cancel_build_requested.emit(province_id))
	close_button.pressed.connect(func() -> void:
		hide()
		closed.emit())
	governor_court_button.pressed.connect(func() -> void: court_requested.emit())


## `province` : entrée MapData fusionnée avec `GameDataStore.get_province` (display_name,
## capital, terrain, owner_display_name…). `state` : `CampaignSim.get_province_state`.
## `recruitable` : `get_recruitable` (vide si la province n'est pas au joueur). `city` :
## `CampaignSim.get_province_city` (vide si la simulation ne l'expose pas encore).
## `label_of(faction_id) -> String` traduit un id de faction en nom court.
func show_province(province: Dictionary, state: Dictionary = {}, recruitable: Array = [], is_player_owner: bool = false, label_of: Callable = Callable(), city: Dictionary = {}) -> void:
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
	var governor_name: String = str(state.get("governor_name", ""))
	governor_label.text = "Gouverneur : %s" % (governor_name if governor_name != "" else "—")
	governor_court_button.visible = state.has("governor")
	_fill_garrison(state.get("garrison", []), is_player_owner)
	_fill_recruitable(recruitable)
	actions.visible = is_player_owner and not state.is_empty()
	if not actions.visible:
		recruit_panel.hide()
	_fill_city(city, is_player_owner)
	show()


## Onglet « Ville » : classes de population, bâtiments, construction, constructible,
## ressources. `city` vide (simulation sans `get_province_city`) → onglet quasi vide.
func _fill_city(city: Dictionary, is_player_owner: bool) -> void:
	_fill_resources(city.get("resources", []))
	_fill_classes(city.get("classes", {}))
	var buildings: Array = city.get("buildings", [])
	_fill_buildings(buildings)
	_fill_construction(city.get("construction", {}), is_player_owner)
	var built_ids: Array = []
	for entry in buildings:
		built_ids.append(str(entry.get("id", "")))
	_fill_buildable(city.get("buildable", []), is_player_owner, built_ids)


func _fill_resources(resources: Array) -> void:
	for child in resources_list.get_children():
		child.queue_free()
	if resources.is_empty():
		var label := Label.new()
		label.text = "—"
		resources_list.add_child(label)
		return
	for res in resources:
		var chip := Label.new()
		chip.text = "◆ %s" % str(RESOURCE_NAMES.get(res, str(res).trim_prefix("res_").capitalize()))
		chip.add_theme_font_size_override("font_size", 13)
		resources_list.add_child(chip)


func _fill_classes(classes: Dictionary) -> void:
	for child in classes_list.get_children():
		child.queue_free()
	if classes.is_empty():
		var label := Label.new()
		label.text = "Données de ville indisponibles."
		classes_list.add_child(label)
		return
	for class_id in ["peasants", "burghers", "clergy", "nobility"]:
		if not classes.has(class_id):
			continue
		classes_list.add_child(_make_class_row(class_id, classes[class_id]))


func _make_class_row(class_id: String, data: Dictionary) -> Control:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 8)
	var name_label := Label.new()
	name_label.text = str(CLASS_LABELS.get(class_id, class_id))
	name_label.custom_minimum_size = Vector2(78, 0)
	row.add_child(name_label)
	var count_label := Label.new()
	count_label.text = _thousands(int(data.get("count", 0)))
	count_label.custom_minimum_size = Vector2(70, 0)
	row.add_child(count_label)
	for spec in GAUGE_SPECS:
		var key: String = spec[0]
		var invert: bool = spec[2]
		var value := float(data.get(key, 0))
		row.add_child(_make_gauge(value, invert, spec[1]))
	return row


## Petite jauge colorée (fond gris, remplissage vert → rouge selon `invert`).
func _make_gauge(value: float, invert: bool, label_text: String) -> Control:
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 0)
	box.custom_minimum_size = Vector2(46, 0)
	var caption := Label.new()
	caption.text = label_text
	caption.add_theme_font_size_override("font_size", 10)
	caption.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(caption)
	var track := ColorRect.new()
	track.color = Color(0.55, 0.50, 0.40)
	track.custom_minimum_size = Vector2(44, 10)
	var fill := ColorRect.new()
	var ratio := clampf(value / 100.0, 0.0, 1.0)
	fill.color = _gauge_color(ratio, invert)
	fill.size = Vector2(44.0 * ratio, 10.0)
	fill.position = Vector2.ZERO
	track.add_child(fill)
	box.add_child(track)
	var value_label := Label.new()
	value_label.text = "%d" % int(round(value))
	value_label.add_theme_font_size_override("font_size", 10)
	value_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	box.add_child(value_label)
	return box


static func _gauge_color(ratio01: float, invert: bool) -> Color:
	var good := Color(0.28, 0.55, 0.22)
	var bad := Color(0.70, 0.16, 0.12)
	return good.lerp(bad, ratio01) if invert else bad.lerp(good, ratio01)


func _fill_buildings(buildings: Array) -> void:
	for child in buildings_list.get_children():
		child.queue_free()
	if buildings.is_empty():
		var label := Label.new()
		label.text = "Aucun bâtiment."
		buildings_list.add_child(label)
		return
	for entry in buildings:
		var line := Label.new()
		line.text = "• %s (entretien %s ℔)" % [str(entry.get("name", entry.get("id", "?"))), _thousands(int(entry.get("upkeep", 0)))]
		line.add_theme_font_size_override("font_size", 14)
		buildings_list.add_child(line)


func _fill_construction(construction: Dictionary, is_player_owner: bool) -> void:
	construction_box.visible = not construction.is_empty()
	if construction.is_empty():
		return
	construction_label.text = "%s — %d tour(s) restant(s)" % [str(construction.get("name", "?")), int(construction.get("turns_left", 0))]
	cancel_build_button.visible = is_player_owner


## `built_ids` filtre les entrées déjà construites (la vraie simulation les inclut dans
## `buildable` avec `available=false, reason="déjà construit"` ; déjà visibles dans la liste
## des bâtiments, inutile de les répéter ici).
func _fill_buildable(buildable: Array, is_player_owner: bool, built_ids: Array = []) -> void:
	for child in buildable_list.get_children():
		child.queue_free()
	if not is_player_owner:
		return
	var rows: Array = []
	for row in buildable:
		if not built_ids.has(str(row.get("building", ""))):
			rows.append(row)
	if rows.is_empty():
		var label := Label.new()
		label.text = "—"
		buildable_list.add_child(label)
		return
	for row in rows:
		var line := HBoxContainer.new()
		line.add_theme_constant_override("separation", 8)
		var button := Button.new()
		button.text = "%s — %s ℔ / %d tour(s)" % [str(row.get("name", row.get("building", "?"))), _thousands(int(row.get("cost", 0))), int(row.get("turns", 1))]
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		var available: bool = bool(row.get("available", false))
		button.disabled = not available
		var building_id: String = str(row.get("building", ""))
		button.pressed.connect(func() -> void: build_requested.emit(province_id, building_id))
		line.add_child(button)
		if not available:
			var reason := Label.new()
			reason.text = str(row.get("reason", "Indisponible"))
			reason.add_theme_font_size_override("font_size", 13)
			reason.add_theme_color_override("font_color", Color(0.55, 0.20, 0.15))
			button.tooltip_text = reason.text
			line.add_child(reason)
		buildable_list.add_child(line)


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


func show_ville_tab() -> void:
	tabs.current_tab = 1


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
