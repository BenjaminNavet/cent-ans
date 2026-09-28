class_name TableSection
extends VBoxContainer

## H9 — section « La Table » du panneau de province (onglet Ville), construite en code :
## régime actuel (icône, nom, coût par saison, description aux mots du Codex cliquables),
## bandeau « Carême » au printemps, sélecteur des régimes (`get_diet_options`) avec une
## infobulle riche par régime (`RichTooltip.diet`), ordre `set_diet` et message de refus.
## Lecture seule hors des provinces du joueur. Aucune règle ici : disponibilité, coûts,
## raisons et refus viennent de `CampaignSim` (`docs/design/h3-h4-api.md` § 5).

signal diet_changed(province_id: String, diet_id: String)

const ROW_ICON := 20.0
const ERROR_COLOR := Color(0.55, 0.20, 0.15)
const MUTED_COLOR := Color(0.42, 0.33, 0.20)
const LENT_TEXT := "[b]Carême[/b] — quarante jours de [[cdx_careme|jeûne]] avant Pâques : une table de viande ou de laitages coûte {rule.lent_piety_penalty} de piété au souverain et fâche le clergé de la province (+{rule.lent_clergy_unrest} de mécontentement) ; le poisson de carême rapporte +{rule.lent_fish_piety} de piété."

var province_id: String = ""
var is_player_owner: bool = false
## Régimes proposés (dernier `get_diet_options`) et boutons du sélecteur, par id.
var options: Array = []
var option_buttons: Dictionary = {}
var last_result: Dictionary = {}

var header_label: Label
var lent_banner: PanelContainer
var lent_text: RichTextLabel
var current_chip: IconChip
var cost_label: Label
var changed_label: Label
var description_label: RichTextLabel
var choose_button: Button
var options_box: VBoxContainer
var error_label: Label
var _sim: Object = null


func _init() -> void:
	name = "TableSection"
	add_theme_constant_override("separation", 4)
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header_label = Label.new()
	header_label.text = "La Table"
	header_label.add_theme_font_size_override("font_size", UiType.size(UiType.BODY))
	add_child(header_label)

	lent_banner = PanelContainer.new()
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.86, 0.80, 0.66)
	style.border_color = Color(0.45, 0.12, 0.08)
	style.border_width_left = 4
	style.set_content_margin_all(6)
	lent_banner.add_theme_stylebox_override("panel", style)
	lent_text = _rich_text(13)
	lent_banner.add_child(lent_text)
	lent_banner.hide()
	add_child(lent_banner)

	var current_row := HBoxContainer.new()
	current_row.add_theme_constant_override("separation", 8)
	add_child(current_row)
	current_chip = IconChip.create("cat_resource", "—", "", ROW_ICON, 14, "resource")
	current_chip.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	current_row.add_child(current_chip)
	cost_label = Label.new()
	cost_label.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	current_row.add_child(cost_label)

	changed_label = Label.new()
	changed_label.text = "Régime déjà changé ce tour-ci (effet à la fin du tour)."
	changed_label.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	changed_label.add_theme_color_override("font_color", MUTED_COLOR)
	changed_label.hide()
	add_child(changed_label)

	description_label = _rich_text(12)
	add_child(description_label)

	choose_button = RichButton.new()
	choose_button.text = "Changer de régime"
	choose_button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	choose_button.pressed.connect(func() -> void: options_box.visible = not options_box.visible)
	add_child(choose_button)
	options_box = VBoxContainer.new()
	options_box.add_theme_constant_override("separation", 2)
	options_box.hide()
	add_child(options_box)

	error_label = Label.new()
	error_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	error_label.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	error_label.add_theme_color_override("font_color", ERROR_COLOR)
	error_label.hide()
	add_child(error_label)


func _ready() -> void:
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles != null:
		bubbles.call("attach", lent_text)
		bubbles.call("attach", description_label)


func _rich_text(font_size: int) -> RichTextLabel:
	var label := RichTextLabel.new()
	label.bbcode_enabled = true
	label.fit_content = true
	label.scroll_active = false
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.custom_minimum_size = Vector2(300, 0)
	label.add_theme_color_override("default_color", RichTooltip.INK)
	for key in ["normal_font_size", "bold_font_size", "italics_font_size"]:
		label.add_theme_font_size_override(key, font_size)
	return label


## Remplit la section pour `province` ; `sim` : `SimFacade.sim` si null. Section masquée si la
## simulation n'expose pas l'API de la Table (mock).
func show_for(province: String, player_owned: bool, sim: Object = null) -> void:
	province_id = province
	is_player_owner = player_owned
	_sim = sim if sim != null else _facade_sim()
	error_label.hide()
	if _sim == null or not _sim.has_method("get_province_diet") or province == "":
		hide()
		return
	var diet: Dictionary = _sim.call("get_province_diet", province)
	if diet.is_empty():
		hide()
		return
	show()
	options = _sim.call("get_diet_options", province) if _sim.has_method("get_diet_options") else []
	var diet_id := str(diet.get("diet", ""))
	var current := _option(diet_id)
	current_chip.label.text = str(diet.get("name", diet_id))
	var library := RichTooltip.icons()
	if library != null:
		current_chip.icon_rect.texture = library.call("get_icon", diet_id, "resource")
	current_chip.tooltip_text = RichTooltip.diet(current) if not current.is_empty() else ""
	var cost := int(diet.get("cost", 0))
	cost_label.text = "%s %s / saison" % [RichTooltip.thousands(cost), RichTooltip.POUND] if cost > 0 else "gratuit"
	var changed := bool(diet.get("changed_this_turn", false))
	changed_label.visible = changed and player_owned
	description_label.text = CodexText.format("[i]%s[/i]" % str(current.get("description", ""))) if not current.is_empty() else ""
	description_label.visible = description_label.text != ""
	var lent := bool(_sim.call("is_lent")) if _sim.has_method("is_lent") else false
	lent_banner.visible = lent
	if lent:
		lent_text.text = CodexText.format(RuleValues.format(LENT_TEXT))
	choose_button.visible = player_owned
	choose_button.disabled = changed
	choose_button.tooltip_text = "Un seul changement par province et par tour." if changed else "Choisir la table de la province (effet à la fin du tour)."
	if not player_owned:
		options_box.hide()
	_fill_options(changed)


func _fill_options(changed: bool) -> void:
	for child in options_box.get_children():
		options_box.remove_child(child)
		child.queue_free()
	option_buttons.clear()
	if not is_player_owner:
		return
	for option in options:
		var id := str(option.get("id", ""))
		var line := HBoxContainer.new()
		line.add_theme_constant_override("separation", 6)
		var button := RichButton.new()
		var cost := int(option.get("cost", 0))
		button.text = "%s — %s" % [str(option.get("name", id)), "%s %s" % [RichTooltip.thousands(cost), RichTooltip.POUND] if cost > 0 else "gratuit"]
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
		button.clip_text = true
		button.custom_minimum_size = Vector2(0, 26)
		var library := RichTooltip.icons()  # autoload par /root (le smoke est compilé avant)
		if library != null:
			library.call("decorate_button", button, id, int(ROW_ICON), "resource")
		button.tooltip_text = RichTooltip.diet(option)
		var current := bool(option.get("current", false))
		var available := bool(option.get("available", false))
		# Un régime indisponible reste cliquable : la simulation motive le refus.
		button.disabled = current or changed
		if current:
			button.text += "  (actuel)"
		button.pressed.connect(func() -> void: request_diet(id))
		line.add_child(button)
		if not available and not current:
			var marker := Label.new()
			marker.text = "indisponible"
			marker.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
			marker.add_theme_color_override("font_color", ERROR_COLOR)
			line.add_child(marker)
		options_box.add_child(line)
		option_buttons[id] = button


## Ordre `set_diet` ; en cas de refus, message de la simulation affiché en rouge.
func request_diet(diet_id: String) -> Dictionary:
	if _sim == null or province_id == "":
		return {}
	last_result = _sim.call("submit_order", {"type": "set_diet", "province": province_id, "diet": diet_id})
	var ok := bool(last_result.get("ok", false))
	var keep_open := options_box.visible and not ok
	show_for(province_id, is_player_owner, _sim)
	options_box.visible = keep_open
	error_label.visible = not ok
	error_label.text = "Refusé : %s" % str(last_result.get("error", "?")) if not ok else ""
	if ok:
		diet_changed.emit(province_id, diet_id)
	return last_result


func _option(diet_id: String) -> Dictionary:
	for option in options:
		if str(option.get("id", "")) == diet_id:
			return option
	return {}


func _facade_sim() -> Object:
	var facade := get_node_or_null("/root/SimFacade") if is_inside_tree() else null
	if facade == null:
		var loop := Engine.get_main_loop() as SceneTree
		facade = loop.root.get_node_or_null("/root/SimFacade") if loop != null else null
	return facade.get("sim") if facade != null else null
