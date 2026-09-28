class_name ChronicleWindow
extends PanelContainer

## Fenêtre « Chronique » (M10) : un événement historique ou aléatoire qui attend la décision du
## joueur — titre, texte d'époque, un bouton par choix (effets en info-bulle et en petit sous le
## bouton). Affiche la première décision de la file ; « Plus tard » la referme sans répondre (la
## décision reste ouverte jusqu'à son expiration). Aucune règle ici : les décisions viennent de
## `CampaignSim.get_pending_decisions`, le choix est renvoyé par le signal `option_chosen`.
## Miniature facultative en bandeau : `res://assets/events/<événement>.jpg` (OpenRouter).

signal option_chosen(decision_id: int, option_index: int)
signal closed

const INK := Color(0.22, 0.14, 0.07)
const FADED_INK := Color(0.40, 0.30, 0.18)
const RUBRIC := Color(0.55, 0.12, 0.10)
const EVENT_ART_DIR := "res://assets/events/"
const ART_SIZE := Vector2(580, 240)

var _kind_label: Label
var _art: TextureRect
var _title_label: Label
var _meta_label: Label
var _text_label: RichTextLabel
var _options_box: VBoxContainer
var _queue_label: Label
var _decision_id: int = -1


func _ready() -> void:
	PanelStack.set_tier(self, PanelStack.Tier.MODAL, true)  # Q4 : décision au-dessus des panneaux
	if theme == null:
		theme = load("res://scenes/ui/parchment_theme.tres")
	# Positionnée à la main (centre de l'écran) : la hauteur dépend du texte et des choix.
	set_anchors_preset(Control.PRESET_TOP_LEFT)
	custom_minimum_size = Vector2(620, 0)
	resized.connect(_center_on_screen)
	var root := VBoxContainer.new()
	root.add_theme_constant_override("separation", 8)
	add_child(root)

	var header := HBoxContainer.new()
	_kind_label = Label.new()
	_kind_label.add_theme_font_size_override("font_size", 14)
	_kind_label.add_theme_color_override("font_color", RUBRIC)
	_kind_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(_kind_label)
	var close := Button.new()
	close.text = "×"
	close.tooltip_text = "Fermer (la décision reste en attente)"
	close.pressed.connect(_close)
	header.add_child(close)
	root.add_child(header)

	_art = TextureRect.new()
	_art.custom_minimum_size = ART_SIZE
	_art.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_art.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	_art.clip_contents = true
	_art.hide()
	root.add_child(_art)

	_title_label = Label.new()
	_title_label.add_theme_font_size_override("font_size", 26)
	_title_label.add_theme_color_override("font_color", INK)
	_title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_title_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	# Wrap width fixed: at width 0 the title wraps one letter per line on the first layout and
	# the window grew taller than the screen (Q3).
	_title_label.custom_minimum_size = Vector2(580, 0)
	root.add_child(_title_label)

	_meta_label = Label.new()
	_meta_label.add_theme_font_size_override("font_size", 14)
	_meta_label.add_theme_color_override("font_color", FADED_INK)
	_meta_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root.add_child(_meta_label)
	root.add_child(HSeparator.new())

	_text_label = RichTextLabel.new()
	_text_label.bbcode_enabled = true
	_text_label.fit_content = true
	_text_label.scroll_active = false
	_text_label.custom_minimum_size = Vector2(580, 0)
	_text_label.add_theme_font_size_override("normal_font_size", 17)
	_text_label.add_theme_font_size_override("italics_font_size", 17)
	_text_label.add_theme_color_override("default_color", INK)
	root.add_child(_text_label)
	# H2 : mots du Codex cliquables (bulles imbriquées).
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles != null:
		bubbles.call("attach", _text_label)
	root.add_child(HSeparator.new())

	_options_box = VBoxContainer.new()
	_options_box.add_theme_constant_override("separation", 6)
	root.add_child(_options_box)

	var footer := HBoxContainer.new()
	_queue_label = Label.new()
	_queue_label.add_theme_font_size_override("font_size", 13)
	_queue_label.add_theme_color_override("font_color", FADED_INK)
	_queue_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	footer.add_child(_queue_label)
	var later := Button.new()
	later.text = "Plus tard"
	later.tooltip_text = "Sans réponse, le premier choix s'appliquera d'office à l'expiration."
	later.pressed.connect(_close)
	footer.add_child(later)
	root.add_child(footer)


## Affiche `decision` (un élément de `get_pending_decisions`) ; `queue_size` décisions en attente.
func show_decision(decision: Dictionary, queue_size: int) -> void:
	_decision_id = int(decision.get("id", -1))
	_kind_label.text = "✠ Chronique du temps" if decision.get("historical", false) else "✠ Nouvelles du royaume"
	_title_label.text = str(decision.get("title", ""))
	_art.texture = PortraitLoader.load_texture(EVENT_ART_DIR + str(decision.get("event", "")) + ".jpg")
	if _art.texture == null:  # AR1 : décision sans miniature propre, vignette de son genre
		_art.texture = ArtPlates.texture(ArtPlates.vignette_for_kind(str(decision.get("kind", ""))))
	_art.visible = _art.texture != null
	var meta := PackedStringArray()
	var province_name := str(decision.get("province_name", ""))
	if province_name != "":
		meta.append(province_name)
	var expires := int(decision.get("expires_in", 0))
	meta.append("à décider ce tour-ci" if expires <= 1 else "à décider sous %d tours" % expires)
	_meta_label.text = " — ".join(meta)
	_text_label.text = "[i]%s[/i]" % CodexText.format(str(decision.get("text", "")), true)
	for child in _options_box.get_children():
		_options_box.remove_child(child)  # out of the layout now, not at the end of the frame
		child.queue_free()
	var options: Array = decision.get("options", [])
	for option in options:
		_options_box.add_child(_option_row(option))
	_queue_label.text = "Décision 1 sur %d" % queue_size if queue_size > 1 else ""
	show()
	reset_size()
	_center_on_screen()
	call_deferred("_fit")


func current_decision() -> int:
	return _decision_id if visible else -1


func _option_row(option: Dictionary) -> Control:
	var row := VBoxContainer.new()
	row.add_theme_constant_override("separation", 1)
	var button := Button.new()
	button.text = str(option.get("text", ""))
	button.add_theme_font_size_override("font_size", 17)
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	var effects := str(option.get("effects_text", ""))
	button.tooltip_text = effects if effects != "" else "Sans effet notable."
	var index := int(option.get("index", 0))
	button.pressed.connect(func() -> void: option_chosen.emit(_decision_id, index))
	row.add_child(button)
	if effects != "":
		var summary := Label.new()
		summary.text = "   " + effects.replace("\n", " · ")
		summary.add_theme_font_size_override("font_size", 13)
		summary.add_theme_color_override("font_color", FADED_INK)
		summary.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		summary.custom_minimum_size = Vector2(580, 0)
		row.add_child(summary)
	return row


## Ramène la fenêtre à la taille de son contenu une fois le texte mis en page.
func _fit() -> void:
	reset_size()
	_center_on_screen()


func _center_on_screen() -> void:
	if not is_inside_tree():
		return
	# PO1 : dans une zone de `UiLayout` — enveloppe défilante du panneau latéral (le conteneur
	# place la fenêtre) ou zone modale (centrée dans la zone, pas dans l'écran).
	var parent := get_parent()
	if parent is Container:
		return
	var area := (parent as Control).size if parent is Control else get_viewport_rect().size
	position = ((area - size) / 2.0).floor()


func _close() -> void:
	hide()
	closed.emit()
