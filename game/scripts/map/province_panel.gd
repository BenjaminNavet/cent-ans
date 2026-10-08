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
## FE6 : clic sur un maillon du fil d'Ariane féodal (détenteur du titre).
signal breadcrumb_clicked(faction_id: String)

const TERRAIN_LABELS := {
	"plains": "Plaines", "hills": "Collines", "mountains": "Montagnes",
	"forest": "Forêt", "marsh": "Marais", "coast": "Littoral", "highlands": "Hautes terres",
	"heath": "Lande", "bocage": "Bocage", "steppe": "Steppe", "desert": "Désert",
}
const STANCE_LABELS := {"normal": "Normale", "raid": "Chevauchée", "siege": "Siège"}
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
@onready var actions: HFlowContainer = %Actions  # Q6 : boutons à la ligne si la zone est étroite
@onready var recruit_button: Button = %RecruitButton
@onready var create_army_button: Button = %CreateArmyButton
@onready var recruit_panel: VBoxContainer = %RecruitPanel
@onready var recruit_list: VBoxContainer = %RecruitList
@onready var close_button: Button = %CloseButton
@onready var tabs: TabContainer = %Tabs
@onready var resources_list: HFlowContainer = %ResourcesList
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
## Classes de population de l'onglet Ville (remplace le conteneur `ClassesList` de la scène).
var classes_list: ClassesSection
var edict_section: EdictSection  # lot C4
## Lot C5 : onglet « Colonies » (construit en code). `settlement_rows_provider(province_id)`
## renvoie les lignes `[{id, name, kind, controller, owner, garrison_units, garrison_strength,
## siege, is_city}]` (fourni par `SettlementController`) ; `label_of` nomme les factions.
var settlements_list: SettlementsSection
var settlement_rows_provider: Callable = Callable()
## RJ-c (ADR 0175) : `possession_provider(province_id)` renvoie la possession décrite
## (`PossessionText.describe` de `province_possession`) ; {} : lignes propriétaire d'origine.
var possession_provider: Callable = Callable()
var possession: Dictionary = {}
## FE6 : fil d'Ariane des titres (« Royaume de France › Duché de Bourgogne › Comté de Charolais »).
var breadcrumb: HFlowContainer


func _ready() -> void:
	Lettrine.attach(name_label, 0.0, true)  # UI1 : titre à lettrine enluminée (Q6 : ajusté à la zone)
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
	IconLibrary.decorate_button(recruit_button, "act_recruit", int(ROW_ICON))
	IconLibrary.decorate_medallion(recruit_button, "recruit", PanelWidgets.MEDALLION_SIZE)  # DA5
	IconLibrary.decorate_button(create_army_button, "act_form_army", int(ROW_ICON))
	IconLibrary.decorate_medallion(create_army_button, "form_army", PanelWidgets.MEDALLION_SIZE)  # DA5
	IconLibrary.decorate_button(governor_court_button, "hud_court", int(ROW_ICON))
	IconLibrary.decorate_button(cancel_build_button, "act_cancel_build", int(ROW_ICON))
	tabs.add_theme_constant_override("icon_max_width", 18)
	tabs.set_tab_icon(0, IconLibrary.get_icon("hud_army"))
	if tabs.get_tab_count() > 1:
		tabs.set_tab_icon(1, IconLibrary.get_icon("cat_class"))
	var classes_slot: Node = %ClassesList
	classes_list = ClassesSection.new()
	classes_slot.replace_by(classes_list)
	classes_slot.queue_free()
	classes_list.name = "ClassesList"
	classes_list.add_theme_constant_override("separation", 3)
	table_section = TableSection.new()  # H9
	classes_list.add_sibling(table_section)
	edict_section = EdictSection.new()  # lot C4
	# Q2 : en tête de l'onglet Ville (en bas, il fallait défiler pour le trouver) ; la liste
	# des édits reste repliée derrière « Changer d'édit ».
	var city_box := classes_list.get_parent()
	city_box.add_child(edict_section)
	city_box.move_child(edict_section, 0)
	var edict_rule := HSeparator.new()
	edict_rule.name = "EdictRule"
	city_box.add_child(edict_rule)
	city_box.move_child(edict_rule, 1)
	edict_section.visibility_changed.connect(func() -> void: edict_rule.visible = edict_section.visible)
	_build_settlements_tab()  # C5
	get_viewport().size_changed.connect(queue_fit_height)  # Q6
	minimum_size_changed.connect(queue_fit_height)  # VN : l'en-tête grandit (fil d'Ariane…) après coup
	breadcrumb = HFlowContainer.new()  # FE6
	breadcrumb.name = "FeudalBreadcrumb"
	breadcrumb.add_theme_constant_override("h_separation", 2)
	# Q6 : sur sa propre ligne sous le titre (dans l'en-tête, titre + fil + × dépassaient la
	# zone `SIDE_PANEL` de 384 px en vue 1280×720).
	name_label.get_parent().add_sibling(breadcrumb)
	_compact_header()  # NT6b


## NT6b : en-tête compacté à 5 lignes en 1280×720 (titre, fil d'Ariane, propriétaire, capitale,
## jauges en une ligne) pour ne plus écraser les onglets. « Aux mains de » et le terrain sont
## fondus dans le propriétaire et la capitale ; le siège n'a sa ligne que s'il y en a un ;
## le gouverneur passe en tête de l'onglet Ville. Les séparateurs disparaissent.
var gauges_row: HFlowContainer
var siege_key: Label
var _siege_key_visible := false


func _compact_header() -> void:
	var box: VBoxContainer = get_node("VBox")
	var grid: GridContainer = box.get_node("Grid")
	for hidden in ["ControllerKey", "ControllerValue", "CapitalKey", "CapitalValue", "TerrainKey", "TerrainValue", "IdKey", "IdValue"]:
		(grid.get_node(hidden) as Control).hide()
	for separator in ["Separator", "Separator2"]:
		(box.get_node(separator) as Control).hide()
	box.add_theme_constant_override("separation", 3)
	grid.add_theme_constant_override("v_separation", 1)
	gauges_row = HFlowContainer.new()
	gauges_row.name = "GaugesRow"
	gauges_row.add_theme_constant_override("h_separation", 4)  # LR-09 : tient sur une ligne en vue étroite
	gauges_row.add_theme_constant_override("v_separation", 0)
	grid.add_sibling(gauges_row)
	var chips := [["PopulationKey", population_value, "Pop."], ["UnrestKey", unrest_value, "Mécont."], ["DevastationKey", devastation_value, "Dévast."]]
	for chip in chips:
		var key: Label = grid.get_node(chip[0])
		var value: Label = chip[1]
		var full_name := key.text
		grid.remove_child(key)
		grid.remove_child(value)
		var holder := HBoxContainer.new()
		holder.add_theme_constant_override("separation", 2)
		key.text = chip[2]
		key.tooltip_text = full_name
		key.add_theme_font_size_override("font_size", 14)  # LR-09 : intitulé abrégé, taille Caption (plancher PO2)
		holder.add_child(key)
		holder.add_child(value)
		gauges_row.add_child(holder)
	siege_key = grid.get_node("SiegeKey")
	# LR-09 : propriétaire et capitale sur une seule ligne (tronquée avec points de suspension) ;
	# le détail (occupant, capitale, terrain) est dans l'infobulle.
	owner_value.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	owner_value.clip_text = true
	owner_value.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	owner_value.custom_minimum_size.x = 0.0
	owner_value.mouse_filter = Control.MOUSE_FILTER_STOP
	(grid.get_node("OwnerKey") as Label).text = "Seigneur"
	# Gouverneur : en tête de l'onglet Ville.
	var governor_row: Control = box.get_node("GovernorRow")
	box.remove_child(governor_row)
	var city_box := classes_list.get_parent()
	city_box.add_child(governor_row)
	city_box.move_child(governor_row, 0)


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
	var owner_label := PanelWidgets.faction_label(owner, province.get("owner_display_name", ""), label_of)
	var controller: String = str(state.get("controller", owner))
	var controller_label := PanelWidgets.faction_label(controller, "", label_of) if controller != owner else "—"
	controller_value.text = controller_label
	var capital: String = str(province.get("capital", province.get("capital_name", "")))
	capital_value.text = capital if capital != "" else "—"
	var terrain: String = str(province.get("terrain", ""))
	terrain_value.text = TERRAIN_LABELS.get(terrain, terrain if terrain != "" else "—")
	# LR-09 : une ligne « Seigneur · cap. Ville · terrain », détail en infobulle.
	var short_line := owner_label
	var detail := "Propriétaire : %s" % owner_label
	if controller != owner:
		short_line += " (occ. %s)" % controller_label
		detail += "\nAux mains de : %s" % controller_label
	if capital != "":
		short_line += " · cap. %s" % capital
		detail += "\nCapitale : %s" % capital
	if terrain != "":
		short_line += " · %s" % str(terrain_value.text).to_lower()
		detail += "\nTerrain : %s" % terrain_value.text
	owner_value.text = short_line
	owner_value.tooltip_text = detail
	_fill_possession(detail)  # RJ-c, après la ligne compacte de LR-09 qu'il remplace
	var population := int(state.get("population_total", province.get("population_total", 0)))
	population_value.text = Money.digits(population) if population > 0 else "—"
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
		siege_value.text = "%s, %s" % [PanelWidgets.faction_label(str(siege.get("attacker", "")), "", label_of), FrText.count(int(siege.get("turns_left", 0)), "tour")]
	if siege_key != null:
		siege_key.visible = not siege.is_empty()
		siege_value.visible = not siege.is_empty()
	id_value.text = "%s (index %d)" % [province_id, int(province.get("index", 0))]
	var governor_name: String = str(state.get("governor_name", ""))
	governor_label.text = "Gouverneur : %s" % (governor_name if governor_name != "" else "—")  # NT6b : ligne de l'onglet Ville
	governor_court_button.visible = state.has("governor")
	_fill_garrison(state.get("garrison", []), is_player_owner)
	_fill_recruitable(recruitable)
	actions.visible = is_player_owner and not state.is_empty()
	if not actions.visible:
		recruit_panel.hide()
	_fill_city(city, is_player_owner)
	table_section.show_for(province_id, is_player_owner and not state.is_empty())  # H9
	edict_section.show_for(province_id, is_player_owner and not state.is_empty())  # lot C4
	_fill_settlements(label_of, is_player_owner)  # C5
	_fill_breadcrumb()  # FE6
	show()
	queue_fit_height()


## Onglet « Ville » : classes de population, bâtiments, construction, constructible,
## ressources. `city` vide (simulation sans `get_province_city`) → onglet quasi vide.
func _fill_city(city: Dictionary, is_player_owner: bool) -> void:
	_fill_resources(city.get("resources", []))
	classes_list.show_for(province_id, is_player_owner, null, {"classes": city.get("classes", {})})
	var buildings: Array = city.get("buildings", [])
	_fill_buildings(buildings)
	_fill_construction(city.get("construction", {}), is_player_owner, (city.get("build_queue", []) as Array).size())
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


func _fill_buildings(buildings: Array) -> void:
	PanelWidgets.fill_buildings(buildings_list, buildings)


func _fill_construction(construction: Dictionary, is_player_owner: bool, queued: int = 0) -> void:
	construction_box.visible = not construction.is_empty()
	if construction.is_empty():
		return
	construction_label.text = "%s — %s restant%s" % [str(construction.get("name", "?")), FrText.count(int(construction.get("turns_left", 0)), "tour"), FrText.s(int(construction.get("turns_left", 0)))]
	if queued > 0:  # M8 : la file se gère dans le panneau de la colonie
		construction_label.text += " (+ %d en file)" % queued
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
	PanelWidgets.bind_army_cap(create_army_button, _garrison_checks, SimFacade.army_unit_cap())  # NT5


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


## Q6 : hauteur de conception des onglets (scène) ; ils rétrécissent si la zone manque.
const TABS_HEIGHT := 343.0
var _fit_queued := false


func _notification(what: int) -> void:
	if what == NOTIFICATION_VISIBILITY_CHANGED and visible:
		queue_fit_height()


## Q6 : le panneau tient dans la zone `SIDE_PANEL` (voir `PanelWidgets.fit_tabs_to_side_zone`).
func queue_fit_height() -> void:
	if _fit_queued:
		return
	_fit_queued = true
	_fit_height.call_deferred()


func _fit_height() -> void:
	_fit_queued = false
	PanelWidgets.fit_tabs_to_side_zone(self, tabs, TABS_HEIGHT)


func show_ville_tab() -> void:
	tabs.current_tab = 1


# --- Lot C5 : onglet « Colonies » ----------------------------------------------------


func _build_settlements_tab() -> void:
	var scroll := ScrollContainer.new()
	scroll.name = "Colonies"
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	tabs.add_child(scroll)
	settlements_list = SettlementsSection.new()
	settlements_list.settlement_requested.connect(settlement_requested.emit)
	scroll.add_child(settlements_list)
	tabs.set_tab_icon(tabs.get_tab_count() - 1, IconLibrary.get_icon("cat_building"))


## RJ-c (ADR 0175) : statut clair de la province pour le joueur (« À vous — occupée par X »…),
## coloré selon la position (ADR 0155). Il remplace la ligne compacte de LR-09 ; l'explication et
## les places tenues passent dans l'infobulle (en-tête court) et en tête de l'onglet des colonies.
func _fill_possession(detail: String) -> void:
	possession = possession_provider.call(province_id) if possession_provider.is_valid() else {}
	var owner_key: Label = get_node_or_null("VBox/Grid/OwnerKey")
	if possession.is_empty():
		if owner_key != null:
			owner_key.text = "Propriétaire"
		owner_value.remove_theme_color_override("font_color")
		return
	if owner_key != null:
		owner_key.text = "Statut"
	owner_value.text = str(possession.get("line", owner_value.text))
	owner_value.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	owner_value.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	owner_value.add_theme_color_override("font_color", PossessionText.ink(str(possession.get("cue", "")), RichTooltip.INK))
	owner_value.mouse_filter = Control.MOUSE_FILTER_PASS
	TooltipHost.attach_plain(owner_value, "province_possession", {"body": "%s\n%s\n%s" % [possession_help(), held_text(), detail]})


## RJ-c : phrase d'aide (la cité donne le contrôle, le traité la possession) et rappel de la
## restitution à la paix d'une place occupée.
func possession_help() -> String:
	var help := PossessionText.province_help(str(possession.get("city_name", "")))
	if bool(possession.get("occupied", false)):
		help += " À la paix, une place occupée et non cédée revient à son propriétaire."
	return help


## RJ-c : « Places tenues : 2 sur 5 — … » (bonus de province complète).
func held_text() -> String:
	return PossessionText.held_line(int(possession.get("held", 0)), int(possession.get("total", 0)), str(possession.get("whole_holder", "")), str(possession.get("player", "")))


func show_settlements_tab() -> void:
	tabs.current_tab = tabs.get_tab_count() - 1


func _fill_settlements(label_of: Callable, is_player_owner: bool) -> void:
	var rows: Array = settlement_rows_provider.call(province_id) if settlement_rows_provider.is_valid() else []
	var help := "%s\n%s" % [possession_help(), held_text()] if not possession.is_empty() else ""
	settlements_list.show_for(province_id, is_player_owner, null, {"rows": rows, "label_of": label_of, "help": help})


## Nom d'unité : `name` si la simulation le fournit, sinon l'id rendu lisible.
static func unit_label(unit: Dictionary) -> String:
	return PanelWidgets.unit_label(unit)


## FE6 : titres de la province, du royaume au comté, chacun cliquable (ouvre l'arbre féodal sur
## son détenteur). Lu dans `CampaignSim.get_province_breadcrumb`.
func _fill_breadcrumb() -> void:
	for child in breadcrumb.get_children():
		breadcrumb.remove_child(child)
		child.queue_free()
	var facade := get_node_or_null("/root/SimFacade")
	var sim: Object = facade.get("sim") if facade != null else null
	var links: Array = sim.call("get_province_breadcrumb", province_id) if sim != null and sim.has_method("get_province_breadcrumb") else []
	breadcrumb.visible = not links.is_empty()
	for index in links.size():
		var link: Dictionary = links[index]
		if index > 0:
			var sep := Label.new()
			sep.text = "›"
			sep.add_theme_color_override("font_color", HudStyle.INK_FADED)
			breadcrumb.add_child(sep)
		var crumb := LinkButton.new()
		crumb.name = "Crumb%d" % index
		crumb.text = str(link.get("title_name", ""))
		crumb.underline = LinkButton.UNDERLINE_MODE_ON_HOVER
		crumb.add_theme_color_override("font_color", HudStyle.RUBRIC)
		var holder := str(link.get("holder", ""))
		var holder_name := str(link.get("holder_name", ""))
		TooltipHost.attach_plain(crumb, "feudal_title_crumb", {"title": "Tenu par %s — ouvrir l'arbre féodal" % holder_name if holder != "" else "Titre vacant"})
		crumb.pressed.connect(func() -> void: breadcrumb_clicked.emit(holder))
		breadcrumb.add_child(crumb)
