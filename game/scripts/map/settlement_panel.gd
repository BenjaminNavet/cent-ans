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
var buildable_list: VBoxContainer

var _garrison_checks: Array[CheckBox] = []


func _init() -> void:
	name = "SettlementPanel"
	clip_contents = true  # Q6 : largeur donnée par la zone `SIDE_PANEL` (pas de minimum fixe)
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
	RichTooltip.attach_plain(province_button, "province_panel_open")
	province_button.pressed.connect(func() -> void: province_requested.emit(province_id))
	grid.add_child(province_button)
	owner_value = _grid_row(grid, "Propriétaire")
	controller_value = _grid_row(grid, "Aux mains de")
	fortification_value = _grid_row(grid, "Fortification")
	siege_value = _grid_row(grid, "Siège")
	income_value = _grid_row(grid, "Revenu")
	box.add_child(HSeparator.new())
	tabs = TabContainer.new()
	tabs.name = "Tabs"
	tabs.custom_minimum_size = Vector2(0, 360)
	tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
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
	value.text = "—"
	value.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	value.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	grid.add_child(value)
	return value


func _tab_box(title: String) -> VBoxContainer:
	var scroll := ScrollContainer.new()
	scroll.name = title
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	tabs.add_child(scroll)
	var inner := VBoxContainer.new()
	inner.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	inner.add_theme_constant_override("separation", 4)
	scroll.add_child(inner)
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
	fortification_value.text = "Niveau %d" % int(detail.get("fortification_level", 0))
	var siege: Dictionary = detail.get("siege", {}) if detail.get("siege") is Dictionary else {}
	if siege.is_empty():
		siege_value.text = "Aucun"
	else:
		siege_value.text = "%s, %s, brèche %d" % [_faction(str(siege.get("attacker", "")), label_of), FrText.count(int(siege.get("turns_left", 0)), "tour"), int(siege.get("breach", 0))]
	income_value.text = "%s / saison (%d %% de la province)" % [Money.amount(int(detail.get("income", 0))), int(round(float(detail.get("weight_share", 0.0)) * 100.0))]
	# Garnison et recrutement.
	var garrison: Array = detail.get("garrison", [])
	garrison_header.text = "Garnison (%d unité%s, %s hommes)" % [garrison.size(), "s" if garrison.size() > 1 else "", PanelWidgets.thousands(int(detail.get("garrison_strength", 0)))]
	_garrison_checks = PanelWidgets.fill_garrison(garrison_list, garrison, player_owner)
	if garrison.is_empty():
		PanelWidgets.placeholder(garrison_list, "Aucune garnison.")
	create_army_button.disabled = garrison.is_empty()
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
	var built_ids: Array = Array(detail.get("buildings", PackedStringArray()))
	PanelWidgets.fill_buildable(buildable_list, buildable, player_owner, built_ids,
		func(building_id: String) -> void: build_requested.emit(settlement_id, building_id))
	if not player_owner:
		PanelWidgets.placeholder(buildable_list, "Colonie hors de votre contrôle.")
	show()
	queue_fit_height()


## Q6 : hauteur de conception des onglets ; ils rétrécissent si la zone `SIDE_PANEL` manque.
const TABS_HEIGHT := 360.0
var _fit_queued := false


func _enter_tree() -> void:
	if not get_viewport().size_changed.is_connected(queue_fit_height):
		get_viewport().size_changed.connect(queue_fit_height)


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


func show_recruit() -> void:
	tabs.current_tab = TAB_GARRISON
	recruit_panel.show()


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
