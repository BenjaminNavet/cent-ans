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


func _ready() -> void:
	# PO phase 2 (P2b, ADR 0097) : tailles (`UiType`) et ouverture/fermeture (`UiMotion`). P2g :
	# `map_ui` le réclame dans la zone `MODAL` de `UiLayout` (`claim_modal_panel`).
	Lettrine.attach(title_label)  # UI1 : titre à lettrine enluminée
	tabs.set_tab_title(0, BRANCH_LABELS["military"])
	tabs.set_tab_title(1, BRANCH_LABELS["civil"])
	tabs.set_tab_title(2, BRANCH_LABELS["medicine"])
	# F2 : icônes des familles de technologies sur les onglets.
	tabs.add_theme_constant_override("icon_max_width", 20)
	tabs.set_tab_icon(0, IconLibrary.get_icon("tech_branch_military"))
	tabs.set_tab_icon(1, IconLibrary.get_icon("tech_branch_civil"))
	tabs.set_tab_icon(2, IconLibrary.get_icon("tech_branch_medicine"))
	# A6-L4 : case « Mettre en file » (équivalent du Maj+clic) sous la barre de recherche.
	_queue_toggle = CheckBox.new()
	_queue_toggle.text = "Mettre en file (ou Maj+clic)"
	_queue_toggle.tooltip_text = "Les technologies cliquées attendent la fin de la recherche en cours, puis démarrent d'elles-mêmes."
	research_bar.get_parent().add_child(_queue_toggle)
	research_bar.get_parent().move_child(_queue_toggle, research_bar.get_index() + 1)
	_queue_toggle.hide()
	for view: TechTreeView in [military_view, civil_view, medicine_view]:
		view.research_requested.connect(_on_view_research_requested)
		view.queue_requested.connect(func(id: String) -> void: queue_requested.emit(id))
	close_button.pressed.connect(func() -> void:
		UiMotion.fade_out(self)  # PO phase 2 (P2b) : fermeture animée (`UiMotion`)
		closed.emit())


## `tree` : `get_tech_tree(faction)` ; `research` : `get_research(faction)` (vide si aucune) ;
## `points_per_turn` : `get_research_points(faction)`.
func _on_view_research_requested(id: String) -> void:
	if _queue_toggle != null and _queue_toggle.button_pressed:
		queue_requested.emit(id)
	else:
		research_requested.emit(id)


## `queue` : `get_research_queue(faction)` ; `reserve` : `get_research_reserve(faction)` (A6-L4).
func show_tree(tree: Array, research: Dictionary, points_per_turn: int, faction_label: String, faction_color: Color,
		queue: Array = [], reserve: Dictionary = {}) -> void:
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
	military_view.show_tree(by_branch["military"])
	civil_view.show_tree(by_branch["civil"])
	medicine_view.show_tree(by_branch["medicine"])
	var queue_text := ""
	if not queue.is_empty():
		var names := PackedStringArray()
		for entry in queue:
			names.append(str(entry.get("name", "")))
		queue_text = "\nEn file : %s" % " → ".join(names)
	_queue_toggle.show()
	_queue_toggle.disabled = research.is_empty() and queue.is_empty()
	if research.is_empty():
		var banked := int(reserve.get("points", 0))
		research_label.text = "Aucune recherche en cours — %d points par tour s'accumulent en réserve (%d / %d) : choisissez une technologie." % [
			points_per_turn, banked, int(reserve.get("cap", 0))] + queue_text
		research_bar.max_value = maxi(1, int(reserve.get("cap", 1)))
		research_bar.value = banked
	else:
		var turns := int(research.get("turns_left", -1))
		research_label.text = "Recherche : %s — %d / %d points (+%d par tour, %s)" % [
			str(research.get("name", "")), int(research.get("progress", 0)), int(research.get("cost", 0)),
			int(research.get("points_per_turn", points_per_turn)),
			"%d tour%s" % [turns, "s" if turns > 1 else ""] if turns >= 0 else "jamais"] + queue_text
		research_bar.max_value = maxi(1, int(research.get("cost", 1)))
		research_bar.value = int(research.get("progress", 0))
	show()
	if not was_visible:
		UiMotion.fade_in(self)


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
