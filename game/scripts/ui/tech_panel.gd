class_name TechPanel
extends PanelContainer

## Panneau « Technologies » (bouton de la barre, touche T) : onglets Militaire, Civil et Médecine
## (`TechTreeView`), recherche en cours et points par tour. Clic sur une technologie
## disponible = ordre `research` (soumis par `CampaignMap`). Aucune règle ici : états,
## coûts effectifs et refus viennent de `CampaignSim` (`get_tech_tree`, `get_research`).

signal research_requested(technology_id: String)
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


func _ready() -> void:
	# PO phase 2 (P2b, ADR 0097) : fenêtre centrale plein écran — voir `docs/wip/p2b-tech-diplo.md`
	# « Point ouvert » : ne rejoint volontairement pas de zone `UiLayout` (`SIDE_PANEL` est trop
	# étroit ; `MODAL` reparente hors de `map_ui`, ce qui casse `_keep_on_screen`/`PanelStack`
	# côté `map_ui.gd`, hors lot — régressions constatées sur `smoke.gd`). Migré : tailles
	# (`UiType`) et animations d'ouverture/fermeture (`UiMotion`).
	Lettrine.attach(title_label)  # UI1 : titre à lettrine enluminée
	tabs.set_tab_title(0, BRANCH_LABELS["military"])
	tabs.set_tab_title(1, BRANCH_LABELS["civil"])
	tabs.set_tab_title(2, BRANCH_LABELS["medicine"])
	# F2 : icônes des familles de technologies sur les onglets.
	tabs.add_theme_constant_override("icon_max_width", 20)
	tabs.set_tab_icon(0, IconLibrary.get_icon("tech_branch_military"))
	tabs.set_tab_icon(1, IconLibrary.get_icon("tech_branch_civil"))
	tabs.set_tab_icon(2, IconLibrary.get_icon("tech_branch_medicine"))
	military_view.research_requested.connect(func(id: String) -> void: research_requested.emit(id))
	civil_view.research_requested.connect(func(id: String) -> void: research_requested.emit(id))
	medicine_view.research_requested.connect(func(id: String) -> void: research_requested.emit(id))
	close_button.pressed.connect(func() -> void:
		UiMotion.fade_out(self)  # PO phase 2 (P2b) : fermeture animée (`UiMotion`)
		closed.emit())


## `tree` : `get_tech_tree(faction)` ; `research` : `get_research(faction)` (vide si aucune) ;
## `points_per_turn` : `get_research_points(faction)`.
func show_tree(tree: Array, research: Dictionary, points_per_turn: int, faction_label: String, faction_color: Color) -> void:
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
	if research.is_empty():
		research_label.text = "Aucune recherche en cours — %d points par tour perdus : choisissez une technologie disponible." % points_per_turn
		research_bar.value = 0
	else:
		var turns := int(research.get("turns_left", -1))
		research_label.text = "Recherche : %s — %d / %d points (+%d par tour, %s)" % [
			str(research.get("name", "")), int(research.get("progress", 0)), int(research.get("cost", 0)),
			int(research.get("points_per_turn", points_per_turn)),
			"%d tour%s" % [turns, "s" if turns > 1 else ""] if turns >= 0 else "jamais"]
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
