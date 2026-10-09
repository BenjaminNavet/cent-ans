class_name SettlementPanel
extends PanelContainer

## Panneau parchemin d'une colonie (lot C5) : nom, type, contrôleur et propriétaire, siège,
## fortification, revenu ; onglets Garnison (recrutement, formation d'armée) et Bâtiments
## (construction). Construit en code, mêmes lignes que le panneau de province
## (`PanelWidgets`). Les ordres émis nomment la colonie (`settlement`), jamais la province.
## Aucune règle ici : options, disponibilités et refus viennent de `CampaignSim`.

signal recruit_requested(settlement_id: String, unit_type: String)
signal create_army_requested(settlement_id: String, unit_indices: Array)
signal build_requested(settlement_id: String, building_id: String)
signal cancel_build_requested(settlement_id: String)
## M8 : annule l'entrée `index` de la file (0 = la prochaine) ; la moitié du coût est remboursée.
signal cancel_queued_build_requested(settlement_id: String, index: int)
## RS-N : `preview` vient de `settlement_demolition_preview` (cœur) : `{building, name,
## can_demolish, reason, refund, upkeep_saved}`.
signal raze_requested(settlement_id: String, building_id: String, preview: Dictionary)
signal province_requested(province_id: String)
signal closed

const KIND_LABELS := {
	"city": "Cité", "town": "Ville", "castle": "Château", "abbey": "Abbaye", "village": "Village",
}
const THEME_PATH := "res://scenes/ui/parchment_theme.tres"
const TAB_GARRISON := 0
const TAB_BUILDINGS := 1

var settlement_id: String = ""
var province_id: String = ""
var is_player_owner: bool = false

var name_label: Label
var kind_value: Label
var province_button: Button
var owner_value: Label
var controller_value: Label
var fortification_value: Label
var siege_value: Label
var income_value: Label
## RJ-c (ADR 0175) : aide sous la grille (cité ou place, possession par traité).
var possession_help: Label
var tabs: TabContainer
var garrison_header: Label
var garrison_list: VBoxContainer
var actions: HFlowContainer
var recruit_button: Button
var create_army_button: Button
var recruit_panel: VBoxContainer
var recruit_header: Label
var recruit_list: VBoxContainer
var queue_label: Label
var buildings_list: VBoxContainer
var construction_box: VBoxContainer
var construction_label: Label
var cancel_build_button: Button
var queue_box: VBoxContainer  # M8 : chantiers en attente derrière celui en cours
var buildable_list: VBoxContainer

var _garrison_checks: Array[CheckBox] = []


func _init() -> void:
	name = "SettlementPanel"
	clip_contents = true  # Q6 : largeur donnée par la zone `SIDE_PANEL` (pas de minimum fixe)
	theme_type_variation = &"IlluminatedPanel"  # A6-U14 : même cadre enluminé que le panneau de province
	if ResourceLoader.exists(THEME_PATH):
		theme = load(THEME_PATH)
	_build()


## Même emplacement que le panneau de province (coin inférieur droit).
func place_like(other: Control) -> void:
	if other == null:
		return
	set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
	offset_left = other.offset_left
	offset_top = other.offset_top
	offset_right = other.offset_right
	offset_bottom = other.offset_bottom
	grow_horizontal = Control.GROW_DIRECTION_BEGIN
	grow_vertical = Control.GROW_DIRECTION_BEGIN


func _build() -> void:
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 6)
	add_child(box)
	var header := HBoxContainer.new()
	box.add_child(header)
	name_label = Label.new()
	name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	UiType.apply(name_label, UiType.HEADING)
	name_label.text = "Colonie"
	header.add_child(name_label)
	Lettrine.attach(name_label, 0.0, true)  # A6-U14 : titre à lettrine, comme la province
	var close_button := Button.new()
	close_button.name = "CloseButton"
	close_button.text = "×"
	close_button.pressed.connect(func() -> void:
		hide()
		closed.emit())
	header.add_child(close_button)
	box.add_child(HSeparator.new())
	var grid := GridContainer.new()
	grid.columns = 2
	grid.add_theme_constant_override("h_separation", 12)
	grid.add_theme_constant_override("v_separation", 3)
	box.add_child(grid)
	kind_value = _grid_row(grid, "Type")
	var province_key := Label.new()
	province_key.text = "Province"
	grid.add_child(province_key)
	province_button = Button.new()
	province_button.name = "ProvinceButton"
	province_button.flat = true
	province_button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	TooltipHost.attach_plain(province_button, "province_panel_open")
	province_button.pressed.connect(func() -> void: province_requested.emit(province_id))
	grid.add_child(province_button)
	owner_value = _grid_row(grid, "Propriétaire")
	controller_value = _grid_row(grid, "Aux mains de")
	fortification_value = _grid_row(grid, "Fortification")
	siege_value = _grid_row(grid, "Siège")
	income_value = _grid_row(grid, "Revenu")
	possession_help = Label.new()
	possession_help.name = "PossessionHelp"
	possession_help.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	possession_help.add_theme_color_override("font_color", Color(RichTooltip.MUTED))
	UiType.apply(possession_help, UiType.CAPTION)
	possession_help.hide()
	box.add_child(possession_help)
	box.add_child(HSeparator.new())
	tabs = TabContainer.new()
	tabs.name = "Tabs"
	tabs.custom_minimum_size = Vector2(0, 240)
	box.add_child(tabs)
	_build_garrison_tab()
	_build_buildings_tab()
	tabs.add_theme_constant_override("icon_max_width", 18)
	tabs.set_tab_icon(TAB_GARRISON, IconLibrary.get_icon("hud_army"))
	tabs.set_tab_icon(TAB_BUILDINGS, IconLibrary.get_icon("cat_building"))


func _grid_row(grid: GridContainer, key: String) -> Label:
	var key_label := Label.new()
	key_label.text = key
	grid.add_child(key_label)
	var value := Label.new()
	value.set_meta(&"row_key", key_label)
	value.text = "—"
	value.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	value.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	grid.add_child(value)
	return value


## RJ-c (ADR 0175) : statut de la place pour le joueur (« À vous — occupée par X »…), coloré
## selon la position, et phrase d'aide ; sans `possession` (simulation ancienne), lignes d'origine.
func _fill_possession(possession: Dictionary) -> void:
	var owner_key: Label = owner_value.get_meta(&"row_key")
	if possession.is_empty():
		owner_key.text = "Propriétaire"
		owner_value.remove_theme_color_override("font_color")
		possession_help.hide()
		return
	owner_key.text = "Statut"
	owner_value.text = str(possession.get("line", owner_value.text))
	owner_value.add_theme_color_override("font_color", PossessionText.ink(str(possession.get("cue", "")), RichTooltip.INK))
	_show_row(controller_value, false)
	var help := PossessionText.settlement_help(bool(possession.get("is_city", false)), str(possession.get("province_name", "")), str(possession.get("city_name", "")))
	if bool(possession.get("occupied", false)):
		help += " Occupée : elle revient à son propriétaire à la paix si le traité ne la cède pas."
	possession_help.text = help
	possession_help.show()


## VN : affiche ou masque une ligne (clé et valeur) de la grille d'informations.
func _show_row(value: Label, shown: bool) -> void:
	value.visible = shown
	(value.get_meta(&"row_key") as Control).visible = shown


func _tab_box(title: String) -> VBoxContainer:
	# A6-U11 : un seul défilement, celui de l'enveloppe de la zone `SIDE_PANEL` (`UiLayout`) ;
	# les pages d'onglet ne défilent plus (il y avait un défilement dans un défilement).
	var inner := VBoxContainer.new()
	inner.name = title
	inner.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	inner.add_theme_constant_override("separation", 4)
	tabs.add_child(inner)
	return inner


func _header(parent: Container, text: String) -> Label:
	var label := Label.new()
	label.text = text
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART  # Q6
	UiType.apply(label, UiType.HEADING)
	parent.add_child(label)
	return label


func _build_garrison_tab() -> void:
	var inner := _tab_box("Garnison")
	garrison_header = _header(inner, "Garnison")
	garrison_list = VBoxContainer.new()
	inner.add_child(garrison_list)
	actions = HFlowContainer.new()  # Q6 : boutons à la ligne si la zone est étroite
	actions.add_theme_constant_override("v_separation", 4)
	inner.add_child(actions)
	recruit_button = Button.new()
	recruit_button.name = "RecruitButton"
	recruit_button.text = "Recruter"
	recruit_button.pressed.connect(func() -> void: recruit_panel.visible = not recruit_panel.visible)
	IconLibrary.decorate_button(recruit_button, "act_recruit", int(PanelWidgets.ROW_ICON))
	IconLibrary.decorate_medallion(recruit_button, "recruit", PanelWidgets.MEDALLION_SIZE)  # DA5
	actions.add_child(recruit_button)
	create_army_button = Button.new()
	create_army_button.name = "CreateArmyButton"
	create_army_button.text = "Former une armée"
	create_army_button.pressed.connect(_on_create_army)
	IconLibrary.decorate_button(create_army_button, "act_form_army", int(PanelWidgets.ROW_ICON))
	IconLibrary.decorate_medallion(create_army_button, "form_army", PanelWidgets.MEDALLION_SIZE)  # DA5
	actions.add_child(create_army_button)
	queue_label = Label.new()
	queue_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	UiType.apply(queue_label, UiType.BODY)
	inner.add_child(queue_label)
	recruit_panel = VBoxContainer.new()
	recruit_panel.visible = false
	inner.add_child(recruit_panel)
	recruit_panel.add_child(HSeparator.new())
	recruit_header = _header(recruit_panel, "Recrutement")
	recruit_list = VBoxContainer.new()
	recruit_panel.add_child(recruit_list)


func _build_buildings_tab() -> void:
	var inner := _tab_box("Bâtiments")
	_header(inner, "Bâtiments")
	buildings_list = VBoxContainer.new()
	inner.add_child(buildings_list)
	construction_box = VBoxContainer.new()
	inner.add_child(construction_box)
	_header(construction_box, "Chantier")
	var row := HBoxContainer.new()
	construction_box.add_child(row)
	construction_label = Label.new()
	construction_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	construction_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	row.add_child(construction_label)
	cancel_build_button = Button.new()
	cancel_build_button.name = "CancelBuildButton"
	cancel_build_button.text = "Annuler"
	cancel_build_button.pressed.connect(func() -> void: cancel_build_requested.emit(settlement_id))
	IconLibrary.decorate_button(cancel_build_button, "act_cancel_build", int(PanelWidgets.ROW_ICON))
	row.add_child(cancel_build_button)
	queue_box = VBoxContainer.new()
	queue_box.name = "BuildQueue"
	construction_box.add_child(queue_box)
	_header(inner, "Construire")
	buildable_list = VBoxContainer.new()
	inner.add_child(buildable_list)


## `detail` : `CampaignSim.settlement_detail(id)`. `recruitable` : `get_recruitable(id)`,
## `buildable` : `settlement_buildable(id)`, `demolition` : `settlement_demolition_preview(id)`
## (vides si la colonie n'est pas au joueur). `label_of(faction_id) -> String` traduit un id
## de faction en nom court.
func show_settlement(detail: Dictionary, recruitable: Array = [], buildable: Array = [], player_owner: bool = false, label_of: Callable = Callable(), demolition: Array = []) -> void:
	if detail.is_empty():
		hide()
		return
	settlement_id = str(detail.get("id", ""))
	province_id = str(detail.get("province", ""))
	is_player_owner = player_owner
	name_label.text = str(detail.get("name", settlement_id))
	var kind := str(detail.get("kind", ""))
	kind_value.text = str(KIND_LABELS.get(kind, kind)) + (" (port)" if bool(detail.get("port", false)) else "")
	province_button.text = str(detail.get("province_name", province_id))
	var owner := str(detail.get("owner", ""))
	var controller := str(detail.get("controller", owner))
	owner_value.text = _faction(owner, label_of)
	controller_value.text = _faction(controller, label_of) if controller != owner else "— (le propriétaire)"
	_show_row(controller_value, controller != owner)  # VN : lignes sans information masquées (place à la garnison)
	_fill_possession(detail.get("possession", {}) if detail.get("possession") is Dictionary else {})  # RJ-c
	fortification_value.text = "Niveau %d" % int(detail.get("fortification_level", 0))
	var siege: Dictionary = detail.get("siege", {}) if detail.get("siege") is Dictionary else {}
	if siege.is_empty():
		siege_value.text = "Aucun"
		_show_row(siege_value, false)
	else:
		_show_row(siege_value, true)
		siege_value.text = "%s, %s, brèche %d" % [_faction(str(siege.get("attacker", "")), label_of), FrText.count(int(siege.get("turns_left", 0)), "tour"), int(siege.get("breach", 0))]
	income_value.text = "%s / saison (%d %% de la province)" % [Money.amount(int(detail.get("income", 0))), int(round(float(detail.get("weight_share", 0.0)) * 100.0))]
	# Garnison et recrutement.
	var garrison: Array = detail.get("garrison", [])
	garrison_header.text = "Garnison (%d unité%s, %s hommes)" % [garrison.size(), "s" if garrison.size() > 1 else "", Money.digits(int(detail.get("garrison_strength", 0)))]
	_garrison_checks = PanelWidgets.fill_garrison(garrison_list, garrison, player_owner)
	if garrison.is_empty():
		PanelWidgets.placeholder(garrison_list, "Aucune garnison.")
	create_army_button.disabled = garrison.is_empty()
	PanelWidgets.bind_army_cap(create_army_button, _garrison_checks, SimFacade.army_unit_cap())  # NT5
	actions.visible = player_owner
	recruit_button.disabled = recruitable.is_empty()
	if not player_owner:
		recruit_panel.hide()
	var queue: Array = Array(detail.get("recruit_queue", PackedStringArray()))
	var queue_turns: Array = Array(detail.get("recruit_queue_turns", PackedInt32Array()))
	var queue_names: Array = []
	for index in queue.size():
		var queue_name := GameCatalog.display_name(str(queue[index]))
		# B7b : une recrue longue à lever reste plusieurs tours dans la file.
		var turns_left := int(queue_turns[index]) if index < queue_turns.size() else 1
		if turns_left > 1:
			queue_name += " (%s)" % FrText.count(turns_left, "tour")
		queue_names.append(queue_name)
	queue_label.visible = player_owner or not queue.is_empty()
	var free_slots := int(detail.get("recruit_slots_free", 0))
	queue_label.text = "Recrues attendues : %s (%s libre%s ce tour sur %d)" % [
		", ".join(queue_names) if not queue_names.is_empty() else "aucune",
		FrText.count(free_slots, "place"), FrText.s(free_slots), int(detail.get("recruit_slots", 0))]
	PanelWidgets.fill_recruitable(recruit_list, recruitable,
		func(unit_type: String) -> void: recruit_requested.emit(settlement_id, unit_type))
	# Bâtiments et construction.
	var buildings: Array = detail.get("buildings_info", [])
	var demolition_by_id: Dictionary = {}
	for row in demolition:
		demolition_by_id[str(row.get("building", ""))] = row
	PanelWidgets.fill_buildings(buildings_list, buildings, demolition_by_id, player_owner,
		func(building_id: String, preview: Dictionary) -> void: raze_requested.emit(settlement_id, building_id, preview))
	var construction: Dictionary = detail.get("construction", {}) if detail.get("construction") is Dictionary else {}
	construction_box.visible = not construction.is_empty()
	if not construction.is_empty():
		construction_label.text = "%s — %s restant%s" % [str(construction.get("name", "?")), FrText.count(int(construction.get("turns_left", 0)), "tour"), FrText.s(int(construction.get("turns_left", 0)))]
	cancel_build_button.visible = player_owner
	_fill_build_queue(detail.get("build_queue", []) if detail.get("build_queue") is Array else [], player_owner)
	var built_ids: Array = Array(detail.get("buildings", PackedStringArray()))
	PanelWidgets.fill_buildable(buildable_list, buildable, player_owner, built_ids,
		func(building_id: String) -> void: build_requested.emit(settlement_id, building_id))
	if not player_owner:
		PanelWidgets.placeholder(buildable_list, "Colonie hors de votre contrôle.")
	show()
	queue_fit_height()


## M8 : file de construction (chantiers payés et en attente), une ligne par entrée avec « Annuler ».
func _fill_build_queue(queued: Array, player_owner: bool) -> void:
	PanelWidgets.clear(queue_box)
	queue_box.visible = not queued.is_empty()
	for index in queued.size():
		var entry: Dictionary = queued[index]
		var row := HBoxContainer.new()
		row.name = "Queued%d" % index
		var label := Label.new()
		label.text = "%d. %s — %s" % [index + 2, str(entry.get("name", "?")), FrText.count(int(entry.get("turns_left", 0)), "tour")]
		label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		row.add_child(label)
		if player_owner:
			var cancel := Button.new()
			cancel.name = "CancelQueuedButton"
			cancel.text = "Annuler"
			var queued_index := index
			cancel.pressed.connect(func() -> void: cancel_queued_build_requested.emit(settlement_id, queued_index))
			IconLibrary.decorate_button(cancel, "act_cancel_build", int(PanelWidgets.ROW_ICON))
			row.add_child(cancel)
		queue_box.add_child(row)


## A6-U11 : le panneau n'ajuste plus la hauteur de ses onglets ; la zone latérale défile seule.
## Conservé pour les appelants existants.
func queue_fit_height() -> void:
	pass


func show_buildings_tab() -> void:
	tabs.current_tab = TAB_BUILDINGS


func _on_create_army() -> void:
	var indices: Array = []
	for i in _garrison_checks.size():
		if _garrison_checks[i].button_pressed:
			indices.append(i)
	if indices.is_empty():
		return
	create_army_requested.emit(settlement_id, indices)


static func _faction(faction_id: String, label_of: Callable) -> String:
	if faction_id == "":
		return "—"
	if label_of.is_valid():
		return str(label_of.call(faction_id))
	return faction_id
