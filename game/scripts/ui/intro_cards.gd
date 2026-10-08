class_name IntroCards
extends Control

## Lot MM1 — introduction facultative et passable : cartons illustrés sur les origines de la
## guerre (1328-1337), miniature encadrée d'or qui s'approche lentement, date, titre et texte en
## fondu, musique de cour. Clic, Espace ou Entrée : carton suivant ; Échap ou « Passer » : fin.
## Textes : `data/ui/front_end.json` (`intro`). Signal `finished` à la fin (la vue se libère).

signal finished

const FADE := 0.8
const ART_SIZE := Vector2(880, 495)

var index: int = -1
var cards: Array = []
var _card_seconds := 8.0
var _elapsed := 0.0
var _art_frame: PanelContainer
var _art: TextureRect
var _date: Label
var _title: Label
var _text: Label
var _counter: Label
var _content: Control
var _closing := false
var _previous_music := ""


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	var intro := FrontEndData.intro()
	cards = intro.get("cards", [])
	_card_seconds = float(intro.get("card_seconds", 8.0))
	_build()
	var audio := get_node_or_null("/root/AudioDirector")
	if audio != null and intro.has("music"):
		_previous_music = str(audio.get("current_context"))
		audio.call("play_music", str(intro["music"]))
	modulate.a = 0.0
	var tween := create_tween()
	tween.tween_property(self, "modulate:a", 1.0, FADE)
	next()


func _build() -> void:
	var black := ColorRect.new()
	black.color = Color(0.02, 0.015, 0.01, 1.0)
	black.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	black.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(black)

	_content = VBoxContainer.new()
	_content.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	(_content as VBoxContainer).alignment = BoxContainer.ALIGNMENT_CENTER
	(_content as VBoxContainer).add_theme_constant_override("separation", 14)
	_content.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_content)

	_art_frame = PanelContainer.new()
	var style := StyleBoxFlat.new()
	style.bg_color = FrontEndStyle.GOLD_DARK
	style.border_color = FrontEndStyle.GOLD
	style.set_border_width_all(3)
	style.set_content_margin_all(6)
	style.shadow_color = Color(0, 0, 0, 0.6)
	style.shadow_size = 18
	_art_frame.add_theme_stylebox_override("panel", style)
	_art_frame.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	_art_frame.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_content.add_child(_art_frame)
	var clip := Control.new()
	clip.custom_minimum_size = ART_SIZE
	clip.clip_contents = true
	clip.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_art_frame.add_child(clip)
	_art = TextureRect.new()
	_art.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_art.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	_art.size = ART_SIZE
	_art.pivot_offset = ART_SIZE * 0.5
	_art.mouse_filter = Control.MOUSE_FILTER_IGNORE
	clip.add_child(_art)

	_date = FrontEndStyle.label("", 22, FrontEndStyle.GOLD, FrontEndStyle.title_italic(), 4)
	_date.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_content.add_child(_date)
	_title = FrontEndStyle.label("", 40, Color(0.97, 0.92, 0.80), FrontEndStyle.title_font(), 6)
	_title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_content.add_child(_title)
	_text = FrontEndStyle.label("", 21, Color(0.90, 0.85, 0.74), FrontEndStyle.body_font())
	_text.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_text.custom_minimum_size = Vector2(900, 0)
	_text.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	_content.add_child(_text)

	var footer := UiBuild.hbox(16)
	footer.set_anchors_and_offsets_preset(Control.PRESET_BOTTOM_WIDE)
	footer.offset_top = -64
	footer.offset_bottom = -20
	footer.offset_left = 40
	footer.offset_right = -40
	add_child(footer)
	_counter = FrontEndStyle.label("", 16, Color(0.7, 0.64, 0.52), FrontEndStyle.body_italic())
	_counter.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	footer.add_child(_counter)
	var next_button := UiBuild.button("Suivant")
	FrontEndStyle.style_action_button(next_button, false, 18)
	next_button.pressed.connect(next)
	footer.add_child(next_button)
	var skip := UiBuild.button("Passer l'introduction")
	FrontEndStyle.style_action_button(skip, false, 18)
	skip.pressed.connect(close)
	footer.add_child(skip)


func next() -> void:
	if _closing:
		return
	index += 1
	if index >= cards.size():
		close()
		return
	_elapsed = 0.0
	var card: Dictionary = cards[index]
	var tween := create_tween()
	if index > 0:
		tween.tween_property(_content, "modulate:a", 0.0, FADE * 0.5)
	tween.tween_callback(func() -> void:
		_art.texture = PortraitLoader.load_texture(str(card.get("illustration", "")))
		_art.scale = Vector2.ONE
		_date.text = str(card.get("date", ""))
		_title.text = str(card.get("title", ""))
		_text.text = str(card.get("text", ""))
		_counter.text = "%d / %d — clic ou Espace : suite ; Échap : passer" % [index + 1, cards.size()])
	tween.tween_property(_content, "modulate:a", 1.0, FADE)


func _process(delta: float) -> void:
	if _closing or index < 0:
		return
	_elapsed += delta
	# Lent rapprochement de la miniature (effet « Ken Burns »).
	_art.scale = Vector2.ONE * (1.0 + 0.06 * clampf(_elapsed / _card_seconds, 0.0, 1.0))
	if _elapsed >= _card_seconds:
		next()


func close() -> void:
	if _closing:
		return
	_closing = true
	var audio := get_node_or_null("/root/AudioDirector")
	if audio != null and _previous_music != "":
		audio.call("play_music", _previous_music)
	var tween := create_tween()
	tween.tween_property(self, "modulate:a", 0.0, FADE)
	tween.tween_callback(func() -> void:
		finished.emit()
		queue_free())


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
		accept_event()
		next()


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("ui_cancel"):
		get_viewport().set_input_as_handled()
		close()
	elif event.is_action_pressed("ui_accept") or (event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_SPACE):
		get_viewport().set_input_as_handled()
		next()
