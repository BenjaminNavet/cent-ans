class_name TechPanel
extends PanelContainer

## Panneau « Technologies » (bouton de la barre, touche T) : onglets Militaire, Civil et Médecine
## (`TechTreeView`), recherche en cours et points par tour. Clic sur une technologie
## disponible = ordre `research` (soumis par `CampaignMap`). Aucune règle ici : états,
## coûts effectifs et refus viennent de `CampaignSim` (`get_tech_tree`, `get_research`).

signal research_requested(technology_id: String)
## A6-L4 : mettre en file (Maj+clic ou case « Mettre en file »).
signal queue_requested(technology_id: String)
signal closed

const BRANCHES := ["military", "civil", "medicine"]
const SLOT_COUNT := 3
const BRANCH_LABELS := {"military": "Militaire", "civil": "Civil", "medicine": "Médecine"}

@onready var title_label: Label = %TechTitle
@onready var research_label: Label = %ResearchStatus
@onready var research_bar: ProgressBar = %ResearchStatusBar
@onready var tabs: TabContainer = %TechTabs
@onready var military_view: TechTreeView = %MilitaryTree
@onready var civil_view: TechTreeView = %CivilTree
@onready var medicine_view: TechTreeView = %MedicineTree
@onready var close_button: Button = %TechCloseButton

var _queue_toggle: CheckBox
## UX5-T3 : recherche et bascules de filtre.
var search_edit: LineEdit
var reach_toggle: CheckBox
var unlock_toggle: CheckBox
var fade_known_toggle: CheckBox
## UX5-T4 : ruban de file (3 emplacements) et fin de file ; UX5-T5 : mention d'inactivité.
var ribbon: HBoxContainer
var ribbon_end_label: Label
var idle_label: Label
var _slots: Array = []
var _last_tree: Array = []


func _ready() -> void:
	# PO phase 2 (P2b, ADR 0097) : tailles (`UiType`) et ouverture/fermeture (`UiMotion`). P2g :
	# `map_ui` le réclame dans la zone `MODAL` de `UiLayout` (`claim_modal_panel`).
	Lettrine.attach(title_label)  # Titre à lettrine enluminée
	tabs.set_tab_title(0, BRANCH_LABELS["military"])
	tabs.set_tab_title(1, BRANCH_LABELS["civil"])
	tabs.set_tab_title(2, BRANCH_LABELS["medicine"])
	# Icônes des familles de technologies sur les onglets.
	tabs.add_theme_constant_override("icon_max_width", 20)
	tabs.set_tab_icon(0, IconLibrary.get_icon("tech_branch_military"))
	tabs.set_tab_icon(1, IconLibrary.get_icon("tech_branch_civil"))
	tabs.set_tab_icon(2, IconLibrary.get_icon("tech_branch_medicine"))
	# A6-L4 : case « Mettre en file » (équivalent du Maj+clic) sous la barre de recherche.
	_queue_toggle = CheckBox.new()
	_queue_toggle.text = "Mettre en file (ou Maj+clic)"
	TooltipHost.attach_plain(_queue_toggle, "tech_queue_toggle")
	research_bar.get_parent().add_child(_queue_toggle)
	research_bar.get_parent().move_child(_queue_toggle, research_bar.get_index() + 1)
	_queue_toggle.hide()
	_build_ribbon()
	_build_filters()
	for view: TechTreeView in [military_view, civil_view, medicine_view]:
		view.research_requested.connect(_on_view_research_requested)
		view.queue_requested.connect(func(id: String) -> void: queue_requested.emit(id))
	close_button.pressed.connect(func() -> void:
		UiMotion.fade_out(self)  # PO phase 2 (P2b) : fermeture animée (`UiMotion`)
		closed.emit())


func _on_view_research_requested(id: String) -> void:
	if _queue_toggle != null and _queue_toggle.button_pressed:
		queue_requested.emit(id)
	else:
		research_requested.emit(id)


## UX5-T4 : ruban de file (nom, barre d'encre, « n tours ») sous la barre de recherche.
func _build_ribbon() -> void:
	var parent := research_bar.get_parent()
	idle_label = Label.new()
	idle_label.text = "Aucun savoir à l'étude"
	idle_label.add_theme_color_override("font_color", HudStyle.RUBRIC)
	UiType.apply(idle_label, UiType.CAPTION)
	idle_label.hide()
	ribbon = HBoxContainer.new()
	ribbon.name = "QueueRibbon"
	ribbon.add_theme_constant_override("separation", 8)
	for slot_index in SLOT_COUNT:
		var card := PanelContainer.new()
		card.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		card.add_theme_stylebox_override("panel", HudStyle.card_box(HudStyle.PARCHMENT_LIGHT, HudStyle.INK_SOFT, 1))
		var box := VBoxContainer.new()
		box.add_theme_constant_override("separation", 2)
		card.add_child(box)
		var name_label := HudStyle.label("", UiType.size(UiType.CAPTION), HudStyle.INK)
		name_label.clip_text = true
		name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
		box.add_child(name_label)
		var bar := ProgressBar.new()
		bar.custom_minimum_size = Vector2(0, 8)
		bar.show_percentage = false
		bar.add_theme_stylebox_override("background", HudStyle.card_box(HudStyle.PARCHMENT_DARK, HudStyle.INK_FADED, 1))
		bar.add_theme_stylebox_override("fill", HudStyle.card_box(HudStyle.INK_SOFT, HudStyle.INK, 0))
		box.add_child(bar)
		var turns_label := HudStyle.label("", UiType.size(UiType.CAPTION), HudStyle.INK_SOFT)
		box.add_child(turns_label)
		ribbon.add_child(card)
		_slots.append({"card": card, "name": name_label, "bar": bar, "turns": turns_label})
	ribbon_end_label = HudStyle.label("", UiType.size(UiType.CAPTION), HudStyle.INK_SOFT)
	var anchor := research_bar.get_index() + 1
	for node: Control in [idle_label, ribbon, ribbon_end_label]:
		parent.add_child(node)
		parent.move_child(node, anchor)
		anchor += 1


## UX5-T3 : champ de recherche et bascules, juste au-dessus des onglets.
func _build_filters() -> void:
	var row := HFlowContainer.new()
	row.name = "TechFilters"
	row.add_theme_constant_override("h_separation", 10)
	search_edit = LineEdit.new()
	search_edit.placeholder_text = "Chercher un savoir"
	search_edit.clear_button_enabled = true
	search_edit.custom_minimum_size = Vector2(220, 0)
	search_edit.text_changed.connect(func(_text: String) -> void: _apply_filters())
	row.add_child(search_edit)
	reach_toggle = _filter_toggle(row, "À portée")
	unlock_toggle = _filter_toggle(row, "Débloque une unité ou un bâtiment")
	fade_known_toggle = _filter_toggle(row, "Estomper les acquis")
	var parent := tabs.get_parent()
	parent.add_child(row)
	parent.move_child(row, tabs.get_index())


func _filter_toggle(row: Control, text: String) -> CheckBox:
	var toggle := CheckBox.new()
	toggle.text = text
	toggle.toggled.connect(func(_on: bool) -> void: _apply_filters())
	row.add_child(toggle)
	return toggle


func _all_views() -> Array:
	return [military_view, civil_view, medicine_view]


func filter_state() -> Dictionary:
	return {"query": search_edit.text, "reach": reach_toggle.button_pressed,
		"unlock": unlock_toggle.button_pressed, "fade_known": fade_known_toggle.button_pressed}


func _apply_filters() -> void:
	var filter := filter_state()
	for view: TechTreeView in _all_views():
		view.set_filter(filter)


func _turns_for(remaining: int, points: int) -> int:
	return -1 if points <= 0 else int(ceil(float(remaining) / float(points)))


func _turns_text(turns: int) -> String:
	return "jamais" if turns < 0 else "%d tour%s" % [turns, "s" if turns > 1 else ""]


## Remplit le ruban (affichage seul : calcul à partir de cost/progress/points). Renvoie le
## nombre de tours jusqu'à la fin de la file (-1 : jamais, 0 : rien).
func _fill_ribbon(research: Dictionary, queue: Array, points_per_turn: int, current_turn: int) -> int:
	var entries: Array = []
	var points := int(research.get("points_per_turn", points_per_turn)) if not research.is_empty() else points_per_turn
	var elapsed := 0
	var never := false
	if not research.is_empty():
		var turns := int(research.get("turns_left", _turns_for(int(research.get("cost", 0)) - int(research.get("progress", 0)), points)))
		never = turns < 0
		elapsed = maxi(turns, 0)
		entries.append({"name": str(research.get("name", "")), "max": maxi(1, int(research.get("cost", 1))),
			"value": int(research.get("progress", 0)), "turns": turns})
	for entry in queue:
		var cost := int(entry.get("cost", 0))
		var turns := _turns_for(cost, points)
		if turns < 0 or never:
			never = true
			elapsed = 0
		else:
			elapsed += turns
		entries.append({"name": str(entry.get("name", "")), "max": maxi(1, cost), "value": 0, "turns": -1 if never else elapsed})
	for index in _slots.size():
		var slot: Dictionary = _slots[index]
		(slot["card"] as Control).visible = index < entries.size() or index == 0
		var name_label: Label = slot["name"]
		var bar: ProgressBar = slot["bar"]
		var turns_label: Label = slot["turns"]
		if index < entries.size():
			var item: Dictionary = entries[index]
			var numeral := ["I", "II", "III"][index] if index < 3 else str(index + 1)
			name_label.text = "%s. %s" % [numeral, item["name"]]
			bar.max_value = item["max"]
			bar.value = item["value"]
			turns_label.text = _turns_text(int(item["turns"])) if index == 0 else "prêt dans %s" % _turns_text(int(item["turns"]))
		else:
			name_label.text = "—"
			bar.max_value = 1
			bar.value = 0
			turns_label.text = "emplacement libre"
	var visible_entries := entries.size() > 0
	ribbon.visible = visible_entries
	ribbon_end_label.visible = visible_entries
	if visible_entries:
		if never:
			ribbon_end_label.text = "Fin de la file : jamais (aucun point de recherche)"
		elif current_turn > 0:
			ribbon_end_label.text = "Fin de la file : tour %d" % (current_turn + elapsed)
		else:
			ribbon_end_label.text = "Fin de la file : dans %s" % _turns_text(elapsed)
	return -1 if never else elapsed


## `queue` : `get_research_queue(faction)` ; `reserve` : `get_research_reserve(faction)` (A6-L4).
func show_tree(tree: Array, research: Dictionary, points_per_turn: int, faction_label: String, faction_color: Color,
		queue: Array = [], reserve: Dictionary = {}, current_turn: int = 0) -> void:
	# PO phase 2 (P2b) : n'ouvrir en fondu (`UiMotion`) que si le panneau était fermé — les
	# rafraîchissements (recherche en cours, changement de tour) rappellent `show_tree` sans
	# rejouer l'animation d'ouverture.
	var was_visible := visible
	title_label.text = "Technologies — %s" % faction_label
	title_label.add_theme_color_override("font_color", faction_color.darkened(0.3))
	var by_branch := {"military": [], "civil": [], "medicine": []}
	for node in tree:
		var branch: String = str(node.get("branch", "civil"))
		if by_branch.has(branch):
			(by_branch[branch] as Array).append(node)
	_last_tree = tree
	military_view.show_tree(by_branch["military"])
	civil_view.show_tree(by_branch["civil"])
	medicine_view.show_tree(by_branch["medicine"])
	_apply_filters()
	_fill_ribbon(research, queue, points_per_turn, current_turn)
	idle_label.visible = research.is_empty()
	research_label.visible = research.is_empty()
	research_bar.visible = research.is_empty()
	_queue_toggle.show()
	_queue_toggle.disabled = research.is_empty() and queue.is_empty()
	if research.is_empty():
		var banked := int(reserve.get("points", 0))
		research_label.text = "%d points par tour s’accumulent en réserve (%d / %d) : choisissez un savoir." % [
			points_per_turn, banked, int(reserve.get("cap", 0))]
		research_bar.max_value = maxi(1, int(reserve.get("cap", 1)))
		research_bar.value = banked
	show()
	if not was_visible:
		UiMotion.fade_in(self)
		focus_start(research)


## UX5-T5 : onglet de la recherche en cours (ou du premier savoir disponible), défilement centré
## sur ce nœud. Renvoie l'id visé ("" si rien).
func focus_start(research: Dictionary) -> String:
	var target := str(research.get("technology", ""))
	if target == "":
		for branch in BRANCHES:
			for node in _last_tree:
				if str(node.get("branch", "")) == branch and str(node.get("state", "")) == "available":
					target = str(node["id"])
					break
			if target != "":
				break
	if target == "":
		return ""
	for node in _last_tree:
		if str(node["id"]) == target:
			select_branch(str(node.get("branch", "")))
	_center_on.call_deferred(target)
	return target


func _center_on(technology_id: String) -> void:
	await get_tree().process_frame
	var button := tech_button(technology_id)
	if button == null or not is_instance_valid(button):
		return
	var scroll := button.get_parent().get_parent() as ScrollContainer
	if scroll == null:
		return
	scroll.scroll_horizontal = maxi(0, int(button.position.x + button.size.x * 0.5 - scroll.size.x * 0.5))
	scroll.scroll_vertical = maxi(0, int(button.position.y + button.size.y * 0.5 - scroll.size.y * 0.5))


## Onglet de la branche de `technology_id`, si elle est connue du panneau.
func select_branch(branch: String) -> void:
	var index := BRANCHES.find(branch)
	if index >= 0:
		tabs.current_tab = index


func tech_button(technology_id: String) -> Button:
	if military_view.buttons.has(technology_id):
		return military_view.buttons[technology_id]
	if civil_view.buttons.has(technology_id):
		return civil_view.buttons[technology_id]
	return medicine_view.buttons.get(technology_id)
