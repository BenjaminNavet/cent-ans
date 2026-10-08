class_name ProvinceChoiceSection
extends ProvinceSection

## Base des sections « choisir une option pour la province » du panneau de province (La Table,
## Édit régional) : titre, option actuelle (icône, nom), description aux mots du Codex, bouton
## « Changer… » qui déplie la liste des options, refus en rouge. Lecture seule hors des provinces
## du joueur. Patron de méthode : la fille donne les noms d'appels de la simulation à
## `super(...)`, surcharge `_option_text` / `_option_tooltip` / `_option_locked` / `_option_note`,
## `_refresh_extra` et `_on_option_chosen`, et passe son ordre par `_submit_choice`.

const ROW_ICON := 20.0

## Options proposées (dernier appel d'options de la simulation).
var options: Array = []
var option_buttons: Dictionary = {}

var header_label: Label
var current_chip: IconChip
var description_label: RichTextLabel
var choose_button: Button
var options_box: VBoxContainer

var _state_call: String
var _options_call: String
var _state_key: String
var _icon_kind: String


func _init(section_name: String, title: String, change_text: String, state_call: String, options_call: String,
		state_key: String, icon_category: String, icon_kind: String) -> void:
	super(section_name, 4)
	_state_call = state_call
	_options_call = options_call
	_state_key = state_key
	_icon_kind = icon_kind
	header_label = UiBuild.label(title, UiType.size(UiType.BODY), null, false, 0.0, self)
	var current_row := UiBuild.hbox(8, self)
	current_chip = IconChip.create(icon_category, "—", "", ROW_ICON, 14, icon_kind)
	current_chip.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	current_row.add_child(current_chip)
	description_label = _rich_text(12, 160.0, true)  # zone `SIDE_PANEL` étroite (264 px utiles à 1280×720)
	add_child(description_label)
	choose_button = RichButton.new()
	choose_button.text = change_text
	choose_button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	choose_button.pressed.connect(func() -> void: options_box.visible = not options_box.visible)
	add_child(choose_button)
	options_box = UiBuild.vbox(2, self)
	options_box.hide()
	_add_error_label()


## Section masquée si la simulation n'expose pas l'API (mock) ou si la province est inconnue.
func _render(_data: Dictionary) -> void:
	error_label.hide()
	if _sim == null or not _sim.has_method(_state_call) or province_id == "":
		hide()
		return
	var state: Dictionary = _sim.call(_state_call, province_id)
	if state.is_empty():
		hide()
		return
	show()
	options = _sim.call(_options_call, province_id) if _sim.has_method(_options_call) else []
	var option_id := str(state.get(_state_key, ""))
	var current := find_option(options, option_id)
	current_chip.label.text = str(state.get("name", option_id))
	var library := RichTooltip.icons()
	if library != null:
		current_chip.icon_rect.texture = library.call("get_icon", option_id, _icon_kind)
	current_chip.tooltip_text = _option_tooltip(current) if not current.is_empty() else ""
	description_label.text = CodexText.format("[i]%s[/i]" % str(current.get("description", ""))) if not current.is_empty() else ""
	description_label.visible = description_label.text != ""
	choose_button.visible = is_player_owner
	_refresh_extra(state, current)
	if not is_player_owner:
		options_box.hide()
	_fill_options()


## Ordre `order` ; en cas de refus, message de la simulation en rouge, liste dépliée conservée.
func _submit_choice(order: Dictionary) -> Dictionary:
	if _sim == null or province_id == "":
		return {}
	_submit(order, func() -> void:
		var keep_open := options_box.visible and not bool(last_result.get("ok", false))
		show_for(province_id, is_player_owner, _sim)
		options_box.visible = keep_open)
	return last_result


func _fill_options() -> void:
	UiBuild.clear_children(options_box)
	option_buttons.clear()
	if not is_player_owner:
		return
	var library := RichTooltip.icons()  # autoload par /root (le smoke est compilé avant)
	for option in options:
		var id := str(option.get("id", ""))
		var line := UiBuild.hbox(6, options_box)
		var button := RichButton.new()
		button.text = _option_text(option)
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
		button.clip_text = true
		button.custom_minimum_size = Vector2(0, 26)
		if library != null:
			library.call("decorate_button", button, id, int(ROW_ICON), _icon_kind)
		button.tooltip_text = _option_tooltip(option)
		# Une option indisponible reste cliquable : la simulation motive le refus.
		button.disabled = _option_locked(option)
		button.pressed.connect(func() -> void: _on_option_chosen(id))
		line.add_child(button)
		var note := _option_note(option)
		if note != "":
			UiBuild.label(note, UiType.size(UiType.CAPTION), ERROR_COLOR, false, 0.0, line)
		option_buttons[id] = button


# --- Points d'extension ----------------------------------------------------------------------


## Texte du bouton d'une option (suffixe « (actuel) » compris).
func _option_text(option: Dictionary) -> String:
	return str(option.get("name", option.get("id", "")))


func _option_tooltip(_option: Dictionary) -> String:
	return ""


## Bouton désactivé (option déjà en place ou changement interdit).
func _option_locked(option: Dictionary) -> bool:
	return bool(option.get("current", false))


## Note rouge à droite d'une option (`""` : aucune).
func _option_note(option: Dictionary) -> String:
	return "" if bool(option.get("available", false)) or bool(option.get("current", false)) else "indisponible"


## Complète la section une fois l'option actuelle affichée (bandeaux, coût, bouton « Changer »).
func _refresh_extra(_state: Dictionary, _current: Dictionary) -> void:
	pass


## Appelé quand le joueur choisit l'option `id` : la fille émet son ordre.
func _on_option_chosen(_id: String) -> void:
	pass
