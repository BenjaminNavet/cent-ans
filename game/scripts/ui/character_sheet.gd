class_name CharacterSheet
extends PanelContainer

## Fiche personnage (lot C3) : colonne de gauche = grand portrait encadré
## d'or avec l'écu de la faction, titres, pastilles d'âge/piété/prestige, niveaux des
## domaines, famille, actions (gouverneur/commandement/mariage) ; à droite = description
## (Codex), traits en pastilles illustrées (infobulles riches) et arbre de compétences
## visuel (`SkillTreeView` : bandes par domaine, nœuds appris/disponibles/verrouillés, clic =
## `learn_skill_requested`). C7 : suite du général (`RetinueRow`, vignettes à infobulles ; clic
## = confier le compagnon à un général réuni, ordre `transfer_companion`), dates « 1310–1346 »
## des défunts, repli en une colonne quand la place manque (`fit_beside`).
## Aucune règle ici : les listes de candidats et les refus viennent de `CampaignSim`
## (`get_learnable`, `get_marriage_candidates`, ordres).

signal closed
signal character_requested(character_id: String)
signal governor_requested(character_id: String, province_id: String)
signal general_requested(character_id: String, army_id: String)
signal marriage_requested(character_id: String, spouse_id: String)
signal learn_skill_requested(character_id: String, skill_id: String)

const BRANCH_LABELS := {"command": "Commandement", "governance": "Gouvernance", "court": "Cour"}
const BRANCH_ORDER := ["command", "governance", "court"]
const SEX_LABELS := {"male": "Homme", "female": "Femme"}
const PORTRAIT_SIZE := Vector2(256, 256)
## Couleur de fond des pastilles de trait, par catégorie.
const TRAIT_COLORS := {
	"martial": Color(0.62, 0.13, 0.08), "governance": Color(0.24, 0.34, 0.18),
	"personality": Color(0.42, 0.29, 0.16), "physical": Color(0.55, 0.40, 0.12),
	"acquired": Color(0.20, 0.25, 0.45),
}

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
@onready var portrait_frame: PanelContainer = %PortraitFrame
@onready var titles_label: Label = %TitlesLabel
@onready var stats_row: HFlowContainer = %StatsRow
@onready var skill_points_label: Label = %SkillPointsLabel
@onready var close_button: Button = %CloseButton

var character_id: String = ""
var _description: RichTextLabel  # H2 : description avec mots du Codex
var _skill_tree: Array = []
var _learnable: Array = []
var _skills_learned: Array = []
## C3 : arbre de compétences visuel (dans `%SkillColumns`) et écu posé sur le portrait.
var skill_tree_view: SkillTreeView
var _heraldry: TextureRect
var _branch_values: Dictionary = {}
## C7 : suite du général, compagnon choisi pour un transfert, repli en colonne.
var retinue_row: RetinueRow
var _retinue_header: Label
var _retinue_hint: Label
var _transfer_companion: String = ""
var compact: bool = false
const WIDE_WIDTH := 1000.0
const COMPACT_WIDTH := 600.0
const SCREEN_MARGIN := 16.0
const TALL_VIEW_HEIGHT := 960.0
const DESIGN_HEIGHT := 800.0
const TOP_CLEARANCE := 76.0


func _ready() -> void:
	Lettrine.attach(name_label)  # UI1 : titre à lettrine enluminée
	item_rect_changed.connect(func() -> void: _keep_centered.call_deferred())
	close_button.pressed.connect(func() -> void:
		UiMotion.fade_out(self)  # P2a (ADR 0097, bible DA § 12.4)
		closed.emit())
	governor_button.pressed.connect(func() -> void: _open_picker("governor", "Nommer gouverneur de…", governable_provinces))
	general_button.pressed.connect(func() -> void: _open_picker("general", "Donner le commandement de…", commandable_armies))
	marry_button.pressed.connect(func() -> void: _open_picker("marry", "Marier avec…", marriage_candidates))
	picker_close.pressed.connect(func() -> void: picker_panel.hide())
	_decorate()  # F2
	_add_description()  # H2
	_build_tw_layout()  # C3
	_build_retinue_section()  # C7


# --- F2 : icônes des branches et des actions ---------------------------------------------

## branche → IconChip remplaçant le libellé de la grille des compétences.
var _branch_chips: Dictionary = {}


func _decorate() -> void:
	for pair in [["CommandKey", "command"], ["GovernanceKey", "governance"], ["CourtKey", "court"]]:
		var key := find_child(pair[0], true, false) as Label
		if key == null:
			continue
		var grid := key.get_parent()
		var index := key.get_index()
		var chip := IconChip.create("branch_" + pair[1], key.text, RichTooltip.branch(pair[1]), 20.0, UiType.size(UiType.CAPTION))
		grid.add_child(chip)
		grid.move_child(chip, index)
		grid.remove_child(key)
		key.queue_free()
		_branch_chips[pair[1]] = chip
	IconLibrary.decorate_button(governor_button, "hud_governor", 20)
	IconLibrary.decorate_button(general_button, "hud_army", 20)
	IconLibrary.decorate_button(marry_button, "act_marry", 20)

# --- H2 : description et fiche du Codex --------------------------------------------------


func _add_description() -> void:
	_description = RichTextLabel.new()
	_description.name = "Description"
	_description.bbcode_enabled = true
	_description.fit_content = true
	_description.scroll_active = false
	_description.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	UiType.apply(_description, UiType.CAPTION)
	_description.add_theme_font_size_override("italics_font_size", UiType.size(UiType.CAPTION))
	_description.add_theme_color_override("default_color", Color(0.22, 0.14, 0.07))
	var content := get_node_or_null("VBox/Body/Scroll/Content")
	if content == null:
		return
	content.add_child(_description)
	content.move_child(_description, 0)
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles != null:
		bubbles.call("attach", _description)


func _fill_description(character: Dictionary) -> void:
	if _description == null:
		return
	var definition: Dictionary = GameCatalog.definitions("characters").get(character_id, {})
	var text := CodexText.format(str(character.get("description", definition.get("description", ""))), true)
	if text != "":
		text = "[i]%s[/i]" % text
	var codex := CodexText.store()
	var entry_id := str(codex.call("entry_for_entity", character_id)) if codex != null else ""
	if entry_id != "":
		text += ("\n" if text != "" else "") + "✠ " + CodexText.link(entry_id, "Lire la fiche du Codex")
	_description.text = text
	_description.visible = text != ""

var governable_provinces: Array = []
var commandable_armies: Array = []
var marriage_candidates: Array = []


## `character` : `CampaignSim.get_character(id)`. `skill_tree` : `get_skill_tree()`.
## `learnable` : `get_learnable(id)`. `governable_provinces` : `[{id, name}]` (provinces
## contrôlées par la faction). `commandable_armies` : `[{id, name}]` (armées dans la même
## province, sans général). `marriage_candidates` : `get_marriage_candidates(id)`.
func show_character(character: Dictionary, skill_tree: Array, learnable: Array, governable: Array, commandable: Array, candidates: Array) -> void:
	if character.is_empty():
		UiMotion.fade_out(self)  # P2a (ADR 0097, bible DA § 12.4)
		return
	character_id = str(character["id"])
	_skill_tree = skill_tree
	_learnable = learnable
	_skills_learned = character.get("skills_learned", [])
	governable_provinces = governable
	commandable_armies = commandable
	marriage_candidates = candidates
	picker_panel.hide()

	var faction: String = str(character.get("faction", ""))
	swatch.color = SimFacade.faction_color(faction)
	var house := str(character.get("house", ""))
	# DA2 : portrait vivant (ou armes) à la place du carré de couleur ; l'écu est posé ci-dessous.
	if PortraitLoader.overlay_portrait(swatch, character_id, faction, PORTRAIT_SIZE, character, false):
		swatch.color = Color(0, 0, 0, 0)
	# C3, DA1 : écu de la maison (à défaut de la faction) en bas à droite du portrait peint.
	_heraldry.texture = PortraitLoader.house_heraldry_texture(house, faction)
	var arms_tip := HouseArms.tooltip(house)
	_heraldry.tooltip_text = arms_tip if arms_tip != "" else ("Écu : %s" % SimFacade.faction_short_name(faction) if faction != "" else "")
	swatch.move_child(_heraldry, -1)
	var alive_now: bool = bool(character.get("alive", true))
	var female := str(character.get("sex", "")) == "female"
	swatch.modulate = Color.WHITE if alive_now else Color(0.75, 0.75, 0.75)
	var epithet: String = str(character.get("epithet", ""))
	name_label.text = "%s%s" % [str(character.get("name", "?")), " « %s »" % epithet if epithet != "" else ""]
	subtitle_label.text = "%s — %s — Maison %s%s" % [
		str(character.get("title", "")), _life_text(character),
		str(character.get("house", "")), "" if alive_now else (" — défunte" if female else " — défunt"),
	]
	_fill_titles(character)
	_fill_stats(character)
	role_label.text = "Statut actuel : %s" % str(character.get("role", "à la cour"))
	command_value.text = str(int((character.get("skills", {}) as Dictionary).get("command", 0)))
	governance_value.text = str(int((character.get("skills", {}) as Dictionary).get("governance", 0)))
	court_value.text = str(int((character.get("skills", {}) as Dictionary).get("court", 0)))
	xp_value.text = str(int(character.get("experience", 0)))
	points_value.text = str(int(character.get("skill_points", 0)))
	_branch_values = character.get("skills", {})
	skill_points_label.text = "Points disponibles : %d" % int(character.get("skill_points", 0))
	for branch in _branch_chips:
		(_branch_chips[branch] as Control).tooltip_text = RichTooltip.branch(branch, int((character.get("skills", {}) as Dictionary).get(branch, 0)))

	_fill_description(character)
	_fill_traits(character.get("traits", []))
	_fill_retinue(character)
	_fill_family(character)

	var alive: bool = bool(character.get("alive", true))
	var captive: bool = bool(character.get("captive", false))
	var governable_ok: bool = alive and not captive and str(character.get("army", "")) == "" and not governable.is_empty()
	var commandable_ok: bool = alive and not captive and str(character.get("governor_of", "")) == "" and not commandable.is_empty()
	governor_button.disabled = not governable_ok
	general_button.disabled = not commandable_ok
	marry_button.disabled = not (alive and not captive and str(character.get("spouse", "")) == "" and not candidates.is_empty())
	_show_blockers(character, governable, commandable, candidates)

	_fill_ransom(character)  # G1
	_fill_skill_tree()
	var was_visible := visible
	show()
	if not was_visible:
		UiMotion.fade_in(self)  # P2a (ADR 0097, bible DA § 12.4)


# --- U10 : motifs des actions grisées ------------------------------------------------------

## Libellés des actions de la fiche (motifs des boutons grisés).
const ACTION_NAMES := {"governor": "Nommer gouverneur", "general": "Donner le commandement", "marry": "Marier"}
var _blocker_label: Label


## Motif pour lequel l'action `kind` (« governor », « general », « marry ») est impossible,
## vide si elle est possible. Lecture seule de l'état déjà fourni par `core/`.
static func action_blocker(kind: String, character: Dictionary, entries: Array) -> String:
	var female := str(character.get("sex", "")) == "female"
	if not bool(character.get("alive", true)):
		return "défunte" if female else "défunt"
	if bool(character.get("captive", false)):
		return "retenue captive" if female else "retenu captif"
	match kind:
		"governor":
			if str(character.get("army", "")) != "":
				return "commande déjà une armée"
			if entries.is_empty():
				return "aucune province à gouverner"
		"general":
			if str(character.get("governor_of", "")) != "":
				return "gouverne déjà une province"
			if entries.is_empty():
				return "aucune armée sans chef"
		"marry":
			if str(character.get("spouse", "")) != "":
				return "déjà mariée" if female else "déjà marié"
			if entries.is_empty():
				return "aucun parti disponible"
	return ""


## Infobulle et ligne « Pourquoi ? » sous les boutons grisés (audit A3, P3).
func _show_blockers(character: Dictionary, governable: Array, commandable: Array, candidates: Array) -> void:
	if _blocker_label == null:
		_blocker_label = Label.new()
		_blocker_label.name = "Blockers"
		_blocker_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		UiType.apply(_blocker_label, UiType.CAPTION)
		_blocker_label.add_theme_color_override("font_color", HudStyle.INK_SOFT)
		var actions := governor_button.get_parent()
		actions.get_parent().add_child(_blocker_label)
		actions.get_parent().move_child(_blocker_label, actions.get_index() + 1)
	var lines := PackedStringArray()
	for pair in [["governor", governor_button, governable], ["general", general_button, commandable], ["marry", marry_button, candidates]]:
		var button: Button = pair[1]
		var reason := action_blocker(str(pair[0]), character, pair[2]) if button.disabled else ""
		if reason != "":
			RichTooltip.attach_plain(button, "seat_unavailable", {"body": reason})
		if reason != "":
			lines.append("%s : %s." % [ACTION_NAMES[pair[0]], reason])
	_blocker_label.text = "\n".join(lines)
	_blocker_label.visible = not lines.is_empty()


# --- C3 : présentation ---------------------------------------------------------------------


func _build_tw_layout() -> void:
	var frame_style := StyleBoxFlat.new()
	frame_style.bg_color = HudStyle.INK
	frame_style.border_color = HudStyle.GOLD
	frame_style.set_border_width_all(4)
	frame_style.set_corner_radius_all(3)
	frame_style.set_content_margin_all(4)
	frame_style.shadow_color = HudStyle.SHADOW
	frame_style.shadow_size = 4
	portrait_frame.add_theme_stylebox_override("panel", frame_style)
	_heraldry = TextureRect.new()
	_heraldry.name = "Heraldry"
	_heraldry.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_heraldry.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	_heraldry.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
	_heraldry.offset_left = -58
	_heraldry.offset_top = -66
	_heraldry.offset_right = -6
	_heraldry.offset_bottom = -6
	swatch.add_child(_heraldry)
	skill_tree_view = SkillTreeView.new()
	skill_tree_view.name = "SkillTreeView"
	skill_tree_view.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	skill_tree_view.learn_requested.connect(func(skill_id: String) -> void: learn_skill_requested.emit(character_id, skill_id))
	skill_columns.add_child(skill_tree_view)
	# C7 : l'arbre (~700 px) défile horizontalement dans la colonne repliée au lieu d'élargir
	# la fiche par-dessus la Cour (sans barre quand la place suffit).
	var skill_scroll := ScrollContainer.new()
	skill_scroll.name = "SkillScroll"
	skill_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
	skill_scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	skill_scroll.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var columns_parent := skill_columns.get_parent()
	var columns_index := skill_columns.get_index()
	columns_parent.remove_child(skill_columns)
	skill_scroll.add_child(skill_columns)
	columns_parent.add_child(skill_scroll)
	columns_parent.move_child(skill_scroll, columns_index)
	name_label.add_theme_color_override("font_color", HudStyle.INK)
	skill_points_label.add_theme_color_override("font_color", HudStyle.RUBRIC)


static func _age_text(age: int) -> String:
	return "%d an%s" % [age, "s" if age > 1 else ""]


## « 25 ans » pour un vivant ; « 1310–1346 (36 ans) » pour un défunt (C7).
static func _life_text(character: Dictionary) -> String:
	if bool(character.get("alive", true)):
		return _age_text(int(character.get("age", 0)))
	var birth := int(character.get("birth_year", 0))
	var death := int(character.get("death_year", 0))
	if death > 0 and birth > 0:
		return "%s (%s)" % [FamilyTreeView.life_dates(character), _age_text(death - birth)]
	return FamilyTreeView.life_dates(character)


## Titres en cours (fiche historique : titres sans date de fin), sinon le titre principal.
func _fill_titles(character: Dictionary) -> void:
	var titles := PackedStringArray()
	var main_title := str(character.get("title", ""))
	if main_title != "":
		titles.append(main_title)
	var definition: Dictionary = GameCatalog.definitions("characters").get(character_id, {})
	for entry in definition.get("titles", []):
		var title := str((entry as Dictionary).get("title", ""))
		if title != "" and not (entry as Dictionary).has("to") and not titles.has(title):
			titles.append(title)
	titles_label.text = " · ".join(titles)
	titles_label.visible = not titles.is_empty()


## Pastilles d'âge, de sexe, de piété et de prestige sous le portrait.
func _fill_stats(character: Dictionary) -> void:
	for child in stats_row.get_children():
		stats_row.remove_child(child)
		child.queue_free()
	var age := int(character.get("age", 0))
	var alive := bool(character.get("alive", true))
	var age_pill := _age_text(age) if alive else FamilyTreeView.life_dates(character)
	var age_tip := "Âge : %s (né vers %d)" % [_age_text(age), _birth_year(character)] if alive \
		else "Dates de vie : %s" % _life_text(character)
	var entries := [
		["hud_chronicle", age_pill, age_tip],
		["class_clergy", "Piété %d" % int(character.get("piety", 0)), "Piété : %d / 100" % int(character.get("piety", 0))],
		["class_nobility", "Prestige %d" % int(character.get("prestige", 0)), "Prestige personnel : %d" % int(character.get("prestige", 0))],
	]
	var sex_label := str(SEX_LABELS.get(character.get("sex", ""), ""))
	if sex_label != "":
		entries.insert(1, ["gauge_population", sex_label, sex_label])
	for entry in entries:
		stats_row.add_child(_pill(IconChip.create(entry[0], entry[1], entry[2], 16.0, UiType.size(UiType.CAPTION)), HudStyle.PARCHMENT_DARK))


func _birth_year(character: Dictionary) -> int:
	if int(character.get("birth_year", 0)) > 0:
		return int(character.get("birth_year", 0))
	var facade := get_node_or_null("/root/SimFacade")
	var sim: Object = facade.get("sim") if facade != null else null
	if sim != null and sim.has_method("get_family_tree"):
		var tree: Dictionary = sim.call("get_family_tree", character_id, 0, 0)
		for node in tree.get("nodes", []):
			if str(node.get("id", "")) == character_id:
				return int(node.get("birth_year", 0))
	return 0


## Enveloppe `content` dans une pastille arrondie (fond `color`).
static func _pill(content: Control, color: Color, text_color: Color = HudStyle.INK) -> PanelContainer:
	var pill := PanelContainer.new()
	var style := StyleBoxFlat.new()
	style.bg_color = color
	style.border_color = color.darkened(0.35)
	style.set_border_width_all(1)
	style.set_corner_radius_all(12)
	style.content_margin_left = 6
	style.content_margin_right = 9
	style.content_margin_top = 2
	style.content_margin_bottom = 2
	pill.add_theme_stylebox_override("panel", style)
	pill.mouse_filter = Control.MOUSE_FILTER_PASS
	if content is IconChip:
		(content as IconChip).label.add_theme_color_override("font_color", text_color)
	pill.add_child(content)
	return pill


## Nom affichable d'un personnage (fiche historique, sinon simulation, sinon id).
func _name_of(id: String, fallback: String = "") -> String:
	if fallback != "" and fallback != id:
		return fallback
	var definition: Dictionary = GameCatalog.definitions("characters").get(id, {})
	var display := str((definition.get("name", {}) as Dictionary).get("display", "")) if definition.get("name") is Dictionary else ""
	if display != "":
		return display
	var facade := get_node_or_null("/root/SimFacade")
	var sim: Object = facade.get("sim") if facade != null else null
	if sim != null:
		var other: Dictionary = sim.call("get_character", id)
		if not other.is_empty():
			return str(other.get("name", id))
	return id


# --- G1 : captivité et rançon (ordres `pay_ransom` / `release_captive` soumis à la simulation) ---

var _ransom_row: HBoxContainer
var _ransom_label: Label
var _ransom_button: Button
var _ransom_action: String = ""


func _fill_ransom(character: Dictionary) -> void:
	if _ransom_row == null:
		_ransom_row = HBoxContainer.new()
		_ransom_label = Label.new()
		_ransom_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		_ransom_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		_ransom_button = Button.new()
		_ransom_button.pressed.connect(_on_ransom_pressed)
		_ransom_row.add_child(_ransom_label)
		_ransom_row.add_child(_ransom_button)
		role_label.add_sibling(_ransom_row)
	var captor_name: String = str(character.get("captor_name", ""))
	_ransom_action = str(character.get("ransom_action", ""))
	_ransom_row.visible = bool(character.get("captive", false)) and captor_name != ""
	_ransom_label.text = "Captif de %s — rançon %s" % [captor_name, Money.amount(int(character.get("ransom", 0)))]
	_ransom_button.text = {"pay": "Payer la rançon", "release": "Libérer contre rançon"}.get(_ransom_action, "")
	_ransom_button.visible = _ransom_action != ""
	_ransom_button.tooltip_text = ""


## Soumet l'ordre de rançon ; le refus éventuel (trésor, conditions du geôlier) vient de la
## simulation et s'affiche en info-bulle ; en cas de succès, la fiche est relue.
func _on_ransom_pressed() -> void:
	var facade := get_node_or_null("/root/SimFacade")
	var sim: Object = facade.get("sim") if facade != null else null
	if sim == null or character_id == "":
		return
	var order := {"type": "pay_ransom", "character": character_id, "installments": 1}
	if _ransom_action == "release":
		order = {"type": "release_captive", "character": character_id}
	var result: Dictionary = sim.call("submit_order", order)
	if not bool(result.get("ok", false)):
		RichTooltip.attach_plain(_ransom_button, "ransom_refused", {"body": str(result.get("error", "?"))})
		_ransom_label.text += "\nRefusé : %s" % str(result.get("error", "?"))
		return
	_fill_ransom(sim.call("get_character", character_id))


func _fill_traits(traits: Array) -> void:
	for child in traits_list.get_children():
		child.queue_free()
	if traits.is_empty():
		var label := UiBuild.label("Aucun trait.", 0, null, false, 0.0, traits_list)
		return
	for trait_entry in traits:
		var category: String = str(trait_entry.get("category", ""))
		# DA7c : icône propre au trait, repli catégorie générique (`IconLibrary.resolve`).
		var trait_id: String = str(trait_entry.get("id", ""))
		var chip := IconChip.create(trait_id, str(trait_entry.get("name", trait_entry.get("id", "?"))), RichTooltip.trait_tip(trait_entry), 22.0, UiType.size(UiType.CAPTION), "trait")
		var color: Color = TRAIT_COLORS.get(category, HudStyle.INK_SOFT)
		var pill := _pill(chip, color.lerp(HudStyle.PARCHMENT_LIGHT, 0.68), HudStyle.INK)
		((pill.get_theme_stylebox("panel") as StyleBoxFlat)).border_color = color
		((pill.get_theme_stylebox("panel") as StyleBoxFlat)).set_border_width_all(2)
		traits_list.add_child(pill)


func _fill_family(character: Dictionary) -> void:
	for child in family_list.get_children():
		child.queue_free()
	var spouse_id: String = str(character.get("spouse", ""))
	if spouse_id != "":
		# Audit A3 P4 : le sexe est connu ; les mariages du jeu unissent un homme et une femme.
		var spouse_label := "Épouse" if str(character.get("sex", "")) == "male" else ("Époux" if str(character.get("sex", "")) == "female" else "Conjoint")
		family_list.add_child(_family_row(spouse_label, spouse_id, _name_of(spouse_id, str(character.get("spouse_name", "")))))
	for child_entry in character.get("children", []):
		family_list.add_child(_family_row("Enfant", str(child_entry.get("id", "")), "%s (%d ans)" % [str(child_entry.get("name", "?")), int(child_entry.get("age", 0))]))
	var father_id: String = str(character.get("father", ""))
	if father_id != "":
		family_list.add_child(_family_row("Père", father_id, _name_of(father_id)))
	var mother_id: String = str(character.get("mother", ""))
	if mother_id != "":
		family_list.add_child(_family_row("Mère", mother_id, _name_of(mother_id)))
	if family_list.get_child_count() == 0:
		var label := UiBuild.label("Famille inconnue.", 0, null, false, 0.0, family_list)


func _family_row(role_text: String, id: String, label_text: String) -> Control:
	var row := HBoxContainer.new()
	var role_label_node := UiBuild.label(role_text, 0, null, false, 80, row)
	var link := UiBuild.button(label_text, func() -> void: character_requested.emit(id))
	link.flat = true
	row.add_child(link)
	return row


func _open_picker(kind: String, title: String, entries: Array) -> void:
	picker_title.text = title
	for child in picker_list.get_children():
		child.queue_free()
	if entries.is_empty():
		var label := UiBuild.label("Aucune option disponible.", 0, null, false, 0.0, picker_list)
	for entry in entries:
		var button := UiBuild.button(str(entry.get("name", entry.get("id", "?"))))
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
		"transfer":
			_transfer_to(target_id)


func _fill_skill_tree() -> void:
	skill_tree_view.show_tree(_skill_tree, _skills_learned, _learnable, _branch_values)


# --- C7 : suite du général --------------------------------------------------------------------


func _build_retinue_section() -> void:
	var content := get_node_or_null("VBox/Body/Scroll/Content")
	if content == null:
		return
	var header := HBoxContainer.new()
	header.name = "RetinueHeader"
	_retinue_header = UiBuild.label("Suite")
	_retinue_header.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	UiType.apply(_retinue_header, UiType.BODY)
	header.add_child(_retinue_header)
	_retinue_hint = Label.new()
	UiType.apply(_retinue_hint, UiType.CAPTION)
	_retinue_hint.add_theme_color_override("font_color", HudStyle.INK_SOFT)
	header.add_child(_retinue_hint)
	retinue_row = RetinueRow.new()
	retinue_row.name = "RetinueRow"
	retinue_row.companion_pressed.connect(_on_companion_pressed)
	var after := traits_list.get_index() + 1
	content.add_child(header)
	content.move_child(header, after)
	content.add_child(retinue_row)
	content.move_child(retinue_row, after + 1)


func _fill_retinue(character: Dictionary) -> void:
	if retinue_row == null:
		return
	var companions: Array = character.get("retinue", [])
	var cap := int(character.get("retinue_max", 0))
	_retinue_header.text = "Suite (%d / %d)" % [companions.size(), cap] if cap > 0 else "Suite"
	retinue_row.show_retinue(companions, cap, bool(character.get("alive", true)))
	var sim := _sim()
	var mine := sim == null or not sim.has_method("get_player_faction") \
		or str(character.get("faction", "")) == str(sim.call("get_player_faction"))
	_retinue_hint.text = "Clic : confier à un général réuni" if not companions.is_empty() and mine else ""
	if companions.is_empty():
		_retinue_hint.text = "Aucun compagnon pour l'instant." if bool(character.get("alive", true)) else ""


func _sim() -> Object:
	var facade := get_node_or_null("/root/SimFacade")
	return facade.get("sim") if facade != null else null


## Ouvre le choix du général à qui confier `companion_id` (généraux réunis au même endroit,
## liste fournie par la simulation).
func _on_companion_pressed(companion_id: String) -> void:
	var sim := _sim()
	if sim == null or not sim.has_method("get_retinue_transfer_targets"):
		return
	_transfer_companion = companion_id
	var targets: Array = sim.call("get_retinue_transfer_targets", character_id)
	_open_picker("transfer", "Confier ce compagnon à…", targets)


func _transfer_to(target_id: String) -> void:
	var sim := _sim()
	if sim == null or _transfer_companion == "":
		return
	var result: Dictionary = sim.call("submit_order", {
		"type": "transfer_companion", "from": character_id, "to": target_id, "companion": _transfer_companion})
	_transfer_companion = ""
	if not bool(result.get("ok", false)):
		_retinue_hint.text = "Refusé : %s" % str(result.get("error", "?"))
		return
	_fill_retinue(sim.call("get_character", character_id))


# --- C7 : mise en page (la fiche ne recouvre plus la Cour) ------------------------------------


## Place la fiche à droite de `left_edge` (bord droit du panneau ouvert à gauche, 0 sinon)
## dans une vue de largeur `view_width` : pleine largeur (1000 px, deux colonnes) si la place
## suffit, sinon repli en une colonne défilante, plus étroite.
## VN : `view_height` (0 = inconnue) : sous `TALL_VIEW_HEIGHT` la colonne du portrait passe elle aussi
## dans la page défilante (repli), sinon la fiche (904 px au minimum) dépasse l'écran ; sa hauteur
## est ramenée à la hauteur de conception bornée par l'écran (elle restait gonflée à 2 720 px, la
## hauteur transitoire du premier remplissage, hors de l'écran).
func fit_beside(left_edge: float, view_width: float, view_height: float = 0.0) -> void:
	var available := view_width - left_edge - 2.0 * SCREEN_MARGIN
	var width := WIDE_WIDTH
	if available < WIDE_WIDTH:
		width = maxf(COMPACT_WIDTH, available)
	var short := view_height > 0.0 and view_height < TALL_VIEW_HEIGHT
	set_compact(available < WIDE_WIDTH or short)
	custom_minimum_size.x = width
	offset_right = -SCREEN_MARGIN
	offset_left = -SCREEN_MARGIN - width
	if view_height > 0.0 and visible:
		var height := maxf(get_combined_minimum_size().y, minf(size.y, minf(DESIGN_HEIGHT, view_height - TOP_CLEARANCE)))
		if not is_equal_approx(size.y, height):
			size = Vector2(size.x, height)
		_keep_centered()


## VN : fiche centrée verticalement dans la zone `MODAL` (ancres au centre) : le centrage de la
## zone, fait avec la taille minimale transitoire du premier remplissage (2 720 px), laissait le
## haut de la fiche à -1 360 px, hors de l'écran. Différé depuis `item_rect_changed`.
func _keep_centered() -> void:
	if not is_equal_approx(anchor_top, 0.5) or not is_equal_approx(anchor_bottom, 0.5):
		return
	var half := size.y * 0.5
	var top := -half
	var parent := get_parent() as Control
	if parent != null:
		# Jamais sur la barre du haut : décalée vers le bas si le centre la fait mordre dessus.
		var center_y := parent.global_position.y + parent.size.y * 0.5
		top += maxf(TOP_CLEARANCE - (center_y - half), 0.0)
	if is_equal_approx(offset_top, top) and is_equal_approx(offset_bottom, top + size.y):
		return
	offset_top = top
	offset_bottom = top + size.y


## Deux colonnes (portrait à gauche, contenu à droite) ou une seule colonne défilante.
func set_compact(value: bool) -> void:
	if value == compact:
		return
	compact = value
	var left := find_child("Left", true, false) as Control
	var body := get_node_or_null("VBox/Body") as Control
	var content := get_node_or_null("VBox/Body/Scroll/Content") as Control
	if left == null or body == null or content == null:
		return
	if compact:
		left.reparent(content, false)
		content.move_child(left, 0)
		left.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	else:
		left.reparent(body, false)
		body.move_child(left, 0)
		left.size_flags_horizontal = Control.SIZE_FILL
