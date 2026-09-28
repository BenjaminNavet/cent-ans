class_name BattleReplayBar
extends PanelContainer

## EP13 — barre de rejeu, en haut de l'écran de bataille : lecture/pause, vitesses ×1 à ×8, barre de
## temps (glisser puis lâcher pour sauter), temps écoulé / durée, « Quitter le rejeu », et l'avis
## du cœur quand la re-simulation ne suit plus l'enregistrement (règles changées depuis). Rendu et
## entrées seulement : la scène relaie au cœur (`BattleSim.replay_seek`, `tick`).

signal play_toggled
signal speed_chosen(speed: float)
signal seek_requested(seconds: float)
signal quit_pressed

const SPEEDS := [1.0, 2.0, 4.0, 8.0]

var play_button: Button
var speed_buttons: Array[Button] = []
var timeline: HSlider
var time_label: Label
var title_label: Label
var divergence_label: Label
var quit_button: Button
var duration: float = 0.0
var _dragging := false


func _ready() -> void:
	name = "ReplayBar"
	mouse_filter = Control.MOUSE_FILTER_STOP
	set_anchors_and_offsets_preset(Control.PRESET_CENTER_TOP)
	custom_minimum_size = Vector2(760, 0)
	offset_left = -380
	offset_right = 380
	offset_top = 8
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 4)
	add_child(box)
	var top := HBoxContainer.new()
	top.add_theme_constant_override("separation", 6)
	box.add_child(top)
	title_label = Label.new()
	title_label.text = "Rejeu"
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	title_label.clip_text = true
	title_label.add_theme_font_size_override("font_size", UiType.size(UiType.BODY))
	top.add_child(title_label)
	play_button = Button.new()
	play_button.name = "Play"
	play_button.custom_minimum_size = Vector2(86, 0)
	play_button.focus_mode = Control.FOCUS_NONE
	RichTooltip.attach_plain(play_button, "replay_play_pause")
	play_button.pressed.connect(func() -> void: play_toggled.emit())
	top.add_child(play_button)
	for speed in SPEEDS:
		var button := Button.new()
		button.name = "Speed_x%d" % int(speed)
		button.text = "×%d" % int(speed)
		button.toggle_mode = true
		button.focus_mode = Control.FOCUS_NONE
		RichTooltip.attach_plain(button, "replay_speed", {"title": "Vitesse ×%d" % int(speed), "hint": "+ / −"})
		button.pressed.connect(func() -> void: speed_chosen.emit(speed))
		top.add_child(button)
		speed_buttons.append(button)
	quit_button = Button.new()
	quit_button.name = "Quit"
	quit_button.text = "Quitter le rejeu"
	quit_button.focus_mode = Control.FOCUS_NONE
	quit_button.pressed.connect(func() -> void: quit_pressed.emit())
	top.add_child(quit_button)
	var bottom := HBoxContainer.new()
	bottom.add_theme_constant_override("separation", 8)
	box.add_child(bottom)
	timeline = HSlider.new()
	timeline.name = "Timeline"
	timeline.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	timeline.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	timeline.min_value = 0.0
	timeline.step = 0.1
	timeline.focus_mode = Control.FOCUS_NONE
	RichTooltip.attach_plain(timeline, "replay_timeline_scrub")
	timeline.drag_started.connect(func() -> void: _dragging = true)
	timeline.drag_ended.connect(_on_drag_ended)
	timeline.gui_input.connect(_on_timeline_input)
	bottom.add_child(timeline)
	time_label = Label.new()
	time_label.name = "Time"
	time_label.custom_minimum_size = Vector2(150, 0)
	time_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	bottom.add_child(time_label)
	divergence_label = Label.new()
	divergence_label.name = "Divergence"
	divergence_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	divergence_label.add_theme_color_override("font_color", Color(0.62, 0.12, 0.08))
	divergence_label.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	divergence_label.visible = false
	box.add_child(divergence_label)


## Titre et durée du rejeu.
func setup(title: String, p_duration: float) -> void:
	title_label.text = "Rejeu — %s" % title if title != "" else "Rejeu"
	duration = maxf(p_duration, 0.1)
	timeline.max_value = duration


## État de lecture (appelé à chaque image par la scène).
func show_state(elapsed: float, paused: bool, speed: float, divergence: Dictionary) -> void:
	play_button.text = "Lecture" if paused else "Pause"
	for i in speed_buttons.size():
		speed_buttons[i].set_pressed_no_signal(is_equal_approx(SPEEDS[i], speed))
	if not _dragging:
		timeline.set_value_no_signal(elapsed)
	time_label.text = "%s / %s" % [clock(timeline.value if _dragging else elapsed), clock(duration)]
	var message := str(divergence.get("message", ""))
	divergence_label.visible = message != ""
	divergence_label.text = message


## « 3:05 ».
static func clock(seconds: float) -> String:
	var total := int(floor(maxf(seconds, 0.0)))
	return "%d:%02d" % [total / 60, total % 60]


func is_dragging() -> bool:
	return _dragging


func _on_drag_ended(_changed: bool) -> void:
	_dragging = false
	seek_requested.emit(timeline.value)


## Un clic sur la barre (sans glisser) saute aussi.
func _on_timeline_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and not (event as InputEventMouseButton).pressed and not _dragging:
		if (event as InputEventMouseButton).button_index == MOUSE_BUTTON_LEFT:
			seek_requested.emit.call_deferred(timeline.value)
