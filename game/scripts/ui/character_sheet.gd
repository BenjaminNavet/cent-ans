class_name CharacterSheet
extends PanelContainer

## Fiche personnage : compétences/XP/points, traits (info-bulle), famille, actions
## (gouverneur/commandement/mariage) et arbre de compétences à trois colonnes (tiers en
## lignes, verrouillé/disponible/appris). Aucune règle ici : les listes de candidats et les
## refus viennent de `CampaignSim` (`get_learnable`, `get_marriage_candidates`, ordres).

signal closed
signal character_requested(character_id: String)
signal governor_requested(character_id: String, province_id: String)
signal general_requested(character_id: String, army_id: String)
signal marriage_requested(character_id: String, spouse_id: String)
signal learn_skill_requested(character_id: String, skill_id: String)

const BRANCH_LABELS := {"command": "Commandement", "governance": "Gouvernance", "court": "Cour"}
const BRANCH_ORDER := ["command", "governance", "court"]
const SEX_LABELS := {"male": "Homme", "female": "Femme"}

@onready var swatch: ColorRect = %Swatch
@onready var name_label: Label = %NameLabel
@onready var subtitle_label: Label = %SubtitleLabel
@onready var role_label: Label = %RoleLabel
@onready var command_value: Label = %CommandValue
@onready var governance_value: Label = %GovernanceValue
@onready var court_value: Label = %CourtValue
@onready var xp_value: Label = %XpValue
@onready var points_value: Label = %PointsValue
@onready var traits_list: HFlowContainer = %TraitsList
@onready var family_list: VBoxContainer = %FamilyList
@onready var governor_button: Button = %GovernorButton
@onready var general_button: Button = %GeneralButton
@onready var marry_button: Button = %MarryButton
@onready var picker_panel: VBoxContainer = %PickerPanel
@onready var picker_title: Label = %PickerTitle
@onready var picker_list: VBoxContainer = %PickerList
@onready var picker_close: Button = %PickerClose
@onready var skill_columns: HBoxContainer = %SkillColumns
@onready var close_button: Button = %CloseButton

var character_id: String = ""
var _skill_tree: Array = []
var _learnable: Array = []
var _skills_learned: Array = []


func _ready() -> void:
	close_button.pressed.connect(func() -> void:
		hide()
		closed.emit())
	governor_button.pressed.connect(func() -> void: _open_picker("governor", "Nommer gouverneur de…", governable_provinces))
	general_button.pressed.connect(func() -> void: _open_picker("general", "Donner le commandement de…", commandable_armies))
	marry_button.pressed.connect(func() -> void: _open_picker("marry", "Marier avec…", marriage_candidates))
	picker_close.pressed.connect(func() -> void: picker_panel.hide())

var governable_provinces: Array = []
var commandable_armies: Array = []
var marriage_candidates: Array = []


## `character` : `CampaignSim.get_character(id)`. `skill_tree` : `get_skill_tree()`.
## `learnable` : `get_learnable(id)`. `governable_provinces` : `[{id, name}]` (provinces
## contrôlées par la faction). `commandable_armies` : `[{id, name}]` (armées dans la même
## province, sans général). `marriage_candidates` : `get_marriage_candidates(id)`.
func show_character(character: Dictionary, skill_tree: Array, learnable: Array, governable: Array, commandable: Array, candidates: Array) -> void:
	if character.is_empty():
		hide()
		return
	character_id = str(character["id"])
	_skill_tree = skill_tree
	_learnable = learnable
	_skills_learned = character.get("skills_learned", [])
	governable_provinces = governable
	commandable_armies = commandable
	marriage_candidates = candidates
	picker_panel.hide()

	swatch.color = SimFacade.faction_color(str(character.get("faction", "")))
	var epithet: String = str(character.get("epithet", ""))
	name_label.text = "%s%s" % [str(character.get("name", "?")), " « %s »" % epithet if epithet != "" else ""]
	subtitle_label.text = "%s — %s ans — %s — Maison %s" % [
		str(character.get("title", "")), int(character.get("age", 0)),
		str(SEX_LABELS.get(character.get("sex", ""), "")), str(character.get("house", "")),
	]
	role_label.text = "Statut actuel : %s" % str(character.get("role", "à la cour"))
	command_value.text = str(int((character.get("skills", {}) as Dictionary).get("command", 0)))
	governance_value.text = str(int((character.get("skills", {}) as Dictionary).get("governance", 0)))
	court_value.text = str(int((character.get("skills", {}) as Dictionary).get("court", 0)))
	xp_value.text = str(int(character.get("experience", 0)))
	points_value.text = str(int(character.get("skill_points", 0)))

	_fill_traits(character.get("traits", []))
	_fill_family(character)

	var alive: bool = bool(character.get("alive", true))
	var captive: bool = bool(character.get("captive", false))
	var governable_ok: bool = alive and not captive and str(character.get("army", "")) == "" and not governable.is_empty()
	var commandable_ok: bool = alive and not captive and str(character.get("governor_of", "")) == "" and not commandable.is_empty()
	governor_button.disabled = not governable_ok
	general_button.disabled = not commandable_ok
	marry_button.disabled = not (alive and not captive and str(character.get("spouse", "")) == "" and not candidates.is_empty())

	_fill_skill_tree()
	show()


func _fill_traits(traits: Array) -> void:
	for child in traits_list.get_children():
		child.queue_free()
	if traits.is_empty():
		var label := Label.new()
		label.text = "Aucun trait."
		traits_list.add_child(label)
		return
	for trait_entry in traits:
		var chip := Label.new()
		chip.text = "◆ %s" % str(trait_entry.get("name", trait_entry.get("id", "?")))
		chip.add_theme_font_size_override("font_size", 13)
		var category: String = str(trait_entry.get("category", ""))
		chip.tooltip_text = "%s%s" % [str(trait_entry.get("name", "")), " (%s)" % category if category != "" else ""]
		traits_list.add_child(chip)


func _fill_family(character: Dictionary) -> void:
	for child in family_list.get_children():
		child.queue_free()
	var spouse_id: String = str(character.get("spouse", ""))
	if spouse_id != "":
		family_list.add_child(_family_row("Conjoint(e)", spouse_id, str(character.get("spouse_name", spouse_id))))
	for child_entry in character.get("children", []):
		family_list.add_child(_family_row("Enfant", str(child_entry.get("id", "")), "%s (%d ans)" % [str(child_entry.get("name", "?")), int(child_entry.get("age", 0))]))
	var father_id: String = str(character.get("father", ""))
	if father_id != "":
		family_list.add_child(_family_row("Père", father_id, father_id))
	var mother_id: String = str(character.get("mother", ""))
	if mother_id != "":
		family_list.add_child(_family_row("Mère", mother_id, mother_id))
	if family_list.get_child_count() == 0:
		var label := Label.new()
		label.text = "Famille inconnue."
		family_list.add_child(label)


func _family_row(role_text: String, id: String, label_text: String) -> Control:
	var row := HBoxContainer.new()
	var role_label_node := Label.new()
	role_label_node.text = role_text
	role_label_node.custom_minimum_size = Vector2(80, 0)
	row.add_child(role_label_node)
	var link := Button.new()
	link.text = label_text
	link.flat = true
	link.pressed.connect(func() -> void: character_requested.emit(id))
	row.add_child(link)
	return row


func _open_picker(kind: String, title: String, entries: Array) -> void:
	picker_title.text = title
	for child in picker_list.get_children():
		child.queue_free()
	if entries.is_empty():
		var label := Label.new()
		label.text = "Aucune option disponible."
		picker_list.add_child(label)
	for entry in entries:
		var button := Button.new()
		button.text = str(entry.get("name", entry.get("id", "?")))
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		var target_id: String = str(entry.get("id", ""))
		button.pressed.connect(_on_picker_entry_pressed.bind(kind, target_id))
		picker_list.add_child(button)
	picker_panel.show()


func _on_picker_entry_pressed(kind: String, target_id: String) -> void:
	picker_panel.hide()
	match kind:
		"governor":
			governor_requested.emit(character_id, target_id)
		"general":
			general_requested.emit(character_id, target_id)
		"marry":
			marriage_requested.emit(character_id, target_id)


func _fill_skill_tree() -> void:
	for child in skill_columns.get_children():
		child.queue_free()
	for branch in BRANCH_ORDER:
		var column := VBoxContainer.new()
		column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		column.add_theme_constant_override("separation", 6)
		var header := Label.new()
		header.text = str(BRANCH_LABELS.get(branch, branch))
		header.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		header.add_theme_font_size_override("font_size", 15)
		column.add_child(header)
		var nodes: Array = []
		for node in _skill_tree:
			if str(node.get("branch", "")) == branch:
				nodes.append(node)
		nodes.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return int(a["tier"]) < int(b["tier"]))
		var by_tier: Dictionary = {}
		for node in nodes:
			var tier := int(node["tier"])
			if not by_tier.has(tier):
				by_tier[tier] = []
			(by_tier[tier] as Array).append(node)
		var tiers: Array = by_tier.keys()
		tiers.sort()
		for tier in tiers:
			var tier_label := Label.new()
			tier_label.text = "Rang %d" % tier
			tier_label.add_theme_font_size_override("font_size", 11)
			tier_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
			column.add_child(tier_label)
			for node in by_tier[tier]:
				column.add_child(_make_skill_button(node))
		skill_columns.add_child(column)


func _make_skill_button(node: Dictionary) -> Control:
	var id: String = str(node["id"])
	var button := Button.new()
	var learned: bool = _skills_learned.has(id)
	var available: bool = _learnable.has(id)
	var state_text := "Appris" if learned else ("Disponible" if available else "Verrouillé")
	button.text = "%s (%d)\n%s" % [str(node["name"]), int(node["cost"]), state_text]
	button.autowrap_mode = TextServer.AUTOWRAP_WORD
	button.custom_minimum_size = Vector2(0, 48)
	button.disabled = not available
	var prereq_names: Array = []
	for prereq in node.get("prerequisites", []):
		prereq_names.append(str(prereq))
	button.tooltip_text = "%s\n%s%s" % [
		str(node.get("description", "")),
		"Prérequis : %s\n" % ", ".join(prereq_names) if not prereq_names.is_empty() else "",
		"Coût : %d point(s)" % int(node["cost"]),
	]
	if learned:
		button.add_theme_color_override("font_color", Color(0.15, 0.45, 0.15))
	elif not available:
		button.add_theme_color_override("font_color", Color(0.45, 0.42, 0.38))
	button.pressed.connect(func() -> void: learn_skill_requested.emit(character_id, id))
	return button
