class_name EdictSection
extends VBoxContainer

## Lot C4 — section « Édit régional » du panneau de province (onglet Ville), construite en code :
## édit actif (icône, nom, description), bandeau si un changement est en attente (délai avant
## effet), sélecteur des édits (`get_edict_options`) avec une infobulle riche par édit, ordre
## `set_edict` et message de refus. Lecture seule hors des provinces du joueur — et hors des
## provinces qui ne sont pas entièrement tenues par leur contrôleur (un édit régional suppose
## l'autorité complète). Aucune règle ici : disponibilité, effets et refus viennent de
## `CampaignSim` (`sim_campaign::edicts`).

signal edict_changed(province_id: String, edict_id: String)

const ROW_ICON := 20.0
const ERROR_COLOR := Color(0.55, 0.20, 0.15)
const MUTED_COLOR := Color(0.42, 0.33, 0.20)

var province_id: String = ""
var is_player_owner: bool = false
## Édits proposés (dernier `get_edict_options`), par id.
var options: Array = []
var option_buttons: Dictionary = {}
var last_result: Dictionary = {}

var header_label: Label
var current_chip: IconChip
var pending_label: Label
var description_label: RichTextLabel
var choose_button: Button
var options_box: VBoxContainer
var error_label: Label
var _sim: Object = null


func _init() -> void:
	name = "EdictSection"
	add_theme_constant_override("separation", 4)
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header_label = Label.new()
	header_label.text = "Édit régional"
	header_label.add_theme_font_size_override("font_size", 16)
	add_child(header_label)

	var current_row := HBoxContainer.new()
	current_row.add_theme_constant_override("separation", 8)
	add_child(current_row)
	current_chip = IconChip.create("cat_building", "—", "", ROW_ICON, 14, "building")
	current_chip.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	current_row.add_child(current_chip)

	pending_label = Label.new()
	pending_label.add_theme_font_size_override("font_size", 12)
	pending_label.add_theme_color_override("font_color", MUTED_COLOR)
	pending_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	pending_label.hide()
	add_child(pending_label)

	description_label = _rich_text(12)
	add_child(description_label)

	choose_button = RichButton.new()
	choose_button.text = "Changer d'édit"
	choose_button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	choose_button.pressed.connect(func() -> void: options_box.visible = not options_box.visible)
	add_child(choose_button)
	options_box = VBoxContainer.new()
	options_box.add_theme_constant_override("separation", 2)
	options_box.hide()
	add_child(options_box)

	error_label = Label.new()
	error_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	error_label.add_theme_font_size_override("font_size", 12)
	error_label.add_theme_color_override("font_color", ERROR_COLOR)
	error_label.hide()
	add_child(error_label)


func _ready() -> void:
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles != null:
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
## simulation n'expose pas l'API des édits (mock) ou si la province est inconnue.
func show_for(province: String, player_owned: bool, sim: Object = null) -> void:
	province_id = province
	is_player_owner = player_owned
	_sim = sim if sim != null else _facade_sim()
	error_label.hide()
	if _sim == null or not _sim.has_method("get_province_edict") or province == "":
		hide()
		return
	var edict: Dictionary = _sim.call("get_province_edict", province)
	if edict.is_empty():
		hide()
		return
	show()
	options = _sim.call("get_edict_options", province) if _sim.has_method("get_edict_options") else []
	var edict_id := str(edict.get("edict", ""))
	var current := _option(edict_id)
	current_chip.label.text = str(edict.get("name", edict_id))
	var library := RichTooltip.icons()
	if library != null:
		current_chip.icon_rect.texture = library.call("get_icon", edict_id, "building")
	current_chip.tooltip_text = _tooltip(current) if not current.is_empty() else ""
	var pending := bool(edict.get("pending", false))
	pending_label.visible = pending and player_owned
	if pending:
		pending_label.text = "En vigueur dans %d tour(s) ; l'ancien édit s'applique en attendant." % int(edict.get("delay_turns", 0))
	description_label.text = CodexText.format("[i]%s[/i]" % str(current.get("description", ""))) if not current.is_empty() else ""
	description_label.visible = description_label.text != ""
	choose_button.visible = player_owned
	choose_button.tooltip_text = "Choisir l'édit de la province (délai avant effet selon l'édit)."
	if not player_owned:
		options_box.hide()
	_fill_options()


func _fill_options() -> void:
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
		button.text = str(option.get("name", id))
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
		button.clip_text = true
		button.custom_minimum_size = Vector2(0, 26)
		var library := RichTooltip.icons()  # autoload par /root (le smoke est compilé avant)
		if library != null:
			library.call("decorate_button", button, id, int(ROW_ICON), "building")
		button.tooltip_text = _tooltip(option)
		var current := bool(option.get("current", false))
		var available := bool(option.get("available", false))
		# Un édit indisponible reste cliquable : la simulation motive le refus.
		button.disabled = current
		if current:
			button.text += "  (actuel)" if bool(option.get("active", false)) else "  (en attente)"
		button.pressed.connect(func() -> void: request_edict(id))
		line.add_child(button)
		if not available and not current:
			var marker := Label.new()
			marker.text = str(option.get("reason", "indisponible"))
			marker.add_theme_font_size_override("font_size", 12)
			marker.add_theme_color_override("font_color", ERROR_COLOR)
			line.add_child(marker)
		options_box.add_child(line)
		option_buttons[id] = button


## Infobulle riche d'un édit : effets au format de `RichTooltip._effects_block` (mêmes rangées
## que les bâtiments/régimes), plus le délai s'il n'est pas nul.
func _tooltip(option: Dictionary) -> String:
	if option.is_empty():
		return ""
	var lines: PackedStringArray = []
	lines.append("[b]%s[/b]" % str(option.get("name", "")))
	var delay := int(option.get("delay_turns", 0))
	if delay > 0:
		lines.append("Délai avant effet : %d tour(s)" % delay)
	var effects_block: String = RichTooltip._effects_block(option.get("effects", []))
	if effects_block != "":
		lines.append(effects_block)
	return "\n".join(lines)


## Ordre `set_edict` ; en cas de refus, message de la simulation affiché en rouge.
func request_edict(edict_id: String) -> Dictionary:
	if _sim == null or province_id == "":
		return {}
	last_result = _sim.call("submit_order", {"type": "set_edict", "province": province_id, "edict": edict_id})
	var ok := bool(last_result.get("ok", false))
	var keep_open := options_box.visible and not ok
	show_for(province_id, is_player_owner, _sim)
	options_box.visible = keep_open
	error_label.visible = not ok
	error_label.text = "Refusé : %s" % str(last_result.get("error", "?")) if not ok else ""
	if ok:
		edict_changed.emit(province_id, edict_id)
	return last_result


func _option(edict_id: String) -> Dictionary:
	for option in options:
		if str(option.get("id", "")) == edict_id:
			return option
	return {}


func _facade_sim() -> Object:
	var facade := get_node_or_null("/root/SimFacade") if is_inside_tree() else null
	if facade == null:
		var loop := Engine.get_main_loop() as SceneTree
		facade = loop.root.get_node_or_null("/root/SimFacade") if loop != null else null
	return facade.get("sim") if facade != null else null
