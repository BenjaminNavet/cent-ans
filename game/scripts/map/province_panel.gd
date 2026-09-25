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
## Lot C5 : clic sur une colonie de l'onglet « Colonies ».
signal settlement_requested(settlement_id: String)
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
## F2 : taille des icônes des lignes du panneau.
const ROW_ICON := 20.0

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
## H9 : section « La Table » (onglet Ville, sous les classes), construite en code.
var table_section: TableSection
var edict_section: EdictSection  # lot C4
## Lot C5 : onglet « Colonies » (construit en code). `settlement_rows_provider(province_id)`
## renvoie les lignes `[{id, name, kind, controller, owner, garrison_units, garrison_strength,
## siege, is_city}]` (fourni par `SettlementController`) ; `label_of` nomme les factions.
var settlements_list: VBoxContainer
var settlement_rows_provider: Callable = Callable()
var _label_of: Callable = Callable()


func _ready() -> void:
	recruit_button.pressed.connect(func() -> void: recruit_panel.visible = not recruit_panel.visible)
	create_army_button.pressed.connect(_on_create_army)
	cancel_build_button.pressed.connect(func() -> void: cancel_build_requested.emit(province_id))
	close_button.pressed.connect(func() -> void:
		hide()
		closed.emit())
	governor_court_button.pressed.connect(func() -> void: court_requested.emit())
	# F2 : infobulles des jauges de la grille, icônes des boutons d'action.
	for pair in [[population_value, "population"], [unrest_value, "unrest"], [devastation_value, "devastation"]]:
		var label: Label = pair[0]
		label.set_script(RichLabel)
		label.mouse_filter = Control.MOUSE_FILTER_PASS
		label.tooltip_text = RichTooltip.gauge(pair[1])
	IconLibrary.decorate_button(recruit_button, "cat_unit", int(ROW_ICON))
	IconLibrary.decorate_button(create_army_button, "hud_army", int(ROW_ICON))
	IconLibrary.decorate_button(governor_court_button, "hud_court", int(ROW_ICON))
	IconLibrary.decorate_button(cancel_build_button, "cat_building", int(ROW_ICON))
	tabs.add_theme_constant_override("icon_max_width", 18)
	tabs.set_tab_icon(0, IconLibrary.get_icon("hud_army"))
	if tabs.get_tab_count() > 1:
		tabs.set_tab_icon(1, IconLibrary.get_icon("cat_class"))
	table_section = TableSection.new()  # H9
	classes_list.add_sibling(table_section)
	edict_section = EdictSection.new()  # lot C4
	table_section.add_sibling(edict_section)
	_build_settlements_tab()  # C5


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
	# EQ1: the revolt countdown, as soon as unrest is above the threshold.
	var revolt_seasons := int(state.get("revolt_seasons", 0))
	if revolt_seasons > 0:
		var left := maxi(int(state.get("revolt_seasons_needed", 3)) - revolt_seasons, 1)
		unrest_value.text += " — révolte dans %s" % FrText.count(left, "saison")
	devastation_value.text = ("%d %%" % int(state["devastation"])) if state.has("devastation") else "—"
	unrest_value.tooltip_text = RichTooltip.gauge("unrest", float(state.get("unrest", -1)))
	devastation_value.tooltip_text = RichTooltip.gauge("devastation", float(state.get("devastation", -1)))
	var siege: Dictionary = state.get("siege", {}) if state.get("siege") is Dictionary else {}
	if siege.is_empty():
		siege_value.text = "Aucun"
	else:
		siege_value.text = "%s, %s" % [_faction_label(str(siege.get("attacker", "")), "", label_of), FrText.count(int(siege.get("turns_left", 0)), "tour")]
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
	table_section.show_for(province_id, is_player_owner and not state.is_empty())  # H9
	edict_section.show_for(province_id, is_player_owner and not state.is_empty())  # lot C4
	_label_of = label_of
	_fill_settlements()  # C5
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
		var res_id := str(res)
		resources_list.add_child(IconChip.create(res_id, GameCatalog.display_name(res_id), RichTooltip.resource(res_id), ROW_ICON, 13))


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
	var name_chip := IconChip.create("class_" + class_id, str(CLASS_LABELS.get(class_id, class_id)), RichTooltip.population_class(class_id, data), ROW_ICON, 14)
	name_chip.custom_minimum_size = Vector2(104, 0)
	row.add_child(name_chip)
	var count_label := Label.new()
	count_label.text = _thousands(int(data.get("count", 0)))
	count_label.custom_minimum_size = Vector2(62, 0)
	row.add_child(count_label)
	for spec in GAUGE_SPECS:
		var key: String = spec[0]
		var invert: bool = spec[2]
		var value := float(data.get(key, 0))
		row.add_child(_make_gauge(value, invert, spec[1], key))
	return row


## Petite jauge colorée (fond gris, remplissage vert → rouge selon `invert`), icône et
## infobulle d'explication (F2).
func _make_gauge(value: float, invert: bool, label_text: String, key: String = "") -> Control:
	var holder := RichPanel.new()
	holder.add_theme_stylebox_override("panel", StyleBoxEmpty.new())
	holder.mouse_filter = Control.MOUSE_FILTER_STOP
	holder.tooltip_text = RichTooltip.gauge(key, value) if key != "" else label_text
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 0)
	box.custom_minimum_size = Vector2(46, 0)
	box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	holder.add_child(box)
	var caption := HBoxContainer.new()
	caption.alignment = BoxContainer.ALIGNMENT_CENTER
	caption.add_theme_constant_override("separation", 2)
	caption.mouse_filter = Control.MOUSE_FILTER_IGNORE
	if key != "":
		caption.add_child(IconLibrary.make_rect("gauge_" + key, 12.0))
	var caption_label := Label.new()
	caption_label.text = label_text
	caption_label.add_theme_font_size_override("font_size", 10)
	caption.add_child(caption_label)
	box.add_child(caption)
	var track := ColorRect.new()
	track.color = Color(0.55, 0.50, 0.40)
	track.custom_minimum_size = Vector2(44, 10)
	track.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var fill := ColorRect.new()
	fill.mouse_filter = Control.MOUSE_FILTER_IGNORE
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
	return holder


static func _gauge_color(ratio01: float, invert: bool) -> Color:
	var good := Color(0.28, 0.55, 0.22)
	var bad := Color(0.70, 0.16, 0.12)
	return good.lerp(bad, ratio01) if invert else bad.lerp(good, ratio01)


func _fill_buildings(buildings: Array) -> void:
	PanelWidgets.fill_buildings(buildings_list, buildings)


func _fill_construction(construction: Dictionary, is_player_owner: bool) -> void:
	construction_box.visible = not construction.is_empty()
	if construction.is_empty():
		return
	construction_label.text = "%s — %s restant%s" % [str(construction.get("name", "?")), FrText.count(int(construction.get("turns_left", 0)), "tour"), FrText.s(int(construction.get("turns_left", 0)))]
	cancel_build_button.visible = is_player_owner


## `built_ids` filtre les entrées déjà construites (la vraie simulation les inclut dans
## `buildable` avec `available=false, reason="déjà construit"` ; déjà visibles dans la liste
## des bâtiments, inutile de les répéter ici).
func _fill_buildable(buildable: Array, is_player_owner: bool, built_ids: Array = []) -> void:
	PanelWidgets.fill_buildable(buildable_list, buildable, is_player_owner, built_ids,
		func(building_id: String) -> void: build_requested.emit(province_id, building_id))


func _fill_garrison(garrison: Array, selectable: bool) -> void:
	garrison_header.text = "Garnison (%d unité%s)" % [garrison.size(), "s" if garrison.size() > 1 else ""]
	_garrison_checks = PanelWidgets.fill_garrison(garrison_list, garrison, selectable)
	create_army_button.disabled = garrison.is_empty()


func _fill_recruitable(recruitable: Array) -> void:
	recruit_button.disabled = recruitable.is_empty()
	PanelWidgets.fill_recruitable(recruit_list, recruitable,
		func(unit_type: String) -> void: recruit_requested.emit(province_id, unit_type))


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


# --- Lot C5 : onglet « Colonies » ----------------------------------------------------


func _build_settlements_tab() -> void:
	var scroll := ScrollContainer.new()
	scroll.name = "Colonies"
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	tabs.add_child(scroll)
	settlements_list = VBoxContainer.new()
	settlements_list.name = "SettlementsList"
	settlements_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	settlements_list.add_theme_constant_override("separation", 4)
	scroll.add_child(settlements_list)
	tabs.set_tab_icon(tabs.get_tab_count() - 1, IconLibrary.get_icon("cat_building"))


func show_settlements_tab() -> void:
	tabs.current_tab = tabs.get_tab_count() - 1


func _fill_settlements() -> void:
	PanelWidgets.clear(settlements_list)
	var rows: Array = settlement_rows_provider.call(province_id) if settlement_rows_provider.is_valid() else []
	if rows.is_empty():
		PanelWidgets.placeholder(settlements_list, "Colonies indisponibles.")
		return
	for row in rows:
		settlements_list.add_child(_make_settlement_row(row))


func _make_settlement_row(row: Dictionary) -> Control:
	var settlement_id := str(row.get("id", ""))
	var button := RichButton.new()
	button.name = settlement_id
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var kind := str(row.get("kind", ""))
	var controller := _faction_label(str(row.get("controller", "")), "", _label_of)
	var text := "%s — %s, %s" % [str(row.get("name", settlement_id)), str(SettlementPanel.KIND_LABELS.get(kind, kind)), controller]
	text += "\n    garnison : %s, %s hommes" % [FrText.count(int(row.get("garrison_units", 0)), "unité"), _thousands(int(row.get("garrison_strength", 0)))]
	var siege: Dictionary = row.get("siege", {}) if row.get("siege") is Dictionary else {}
	if not siege.is_empty():
		text += " — assiégée par %s (%s)" % [_faction_label(str(siege.get("attacker", "")), "", _label_of), FrText.count(int(siege.get("turns_left", 0)), "tour")]
	button.text = text
	button.tooltip_text = "Ouvrir le panneau de la colonie et centrer la carte"
	button.pressed.connect(func() -> void: settlement_requested.emit(settlement_id))
	return button


## Nom d'unité : `name` si la simulation le fournit, sinon l'id rendu lisible.
static func unit_label(unit: Dictionary) -> String:
	return PanelWidgets.unit_label(unit)


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
