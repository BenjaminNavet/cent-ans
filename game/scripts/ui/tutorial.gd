class_name TutorialOverlay
extends Control

## F8 — affichage du tutoriel : parchemin d'étape (titre, consigne, conseil historique,
## objectif, boutons « Continuer » / « Passer l'étape » / « Passer le tutoriel ») et, par-dessus
## la carte, un halo pulsé et une flèche vers la cible de l'étape. La couche entière laisse
## passer la souris (seul le parchemin la capte). Ne connaît ni la simulation ni la carte :
## `TutorialController` fournit l'étape (`show_step`) et la cible à chaque image (`set_target`).

signal continue_pressed
signal skip_step_pressed
signal skip_all_pressed

const THEME_PATH := "res://scenes/ui/parchment_theme.tres"
const PANEL_WIDTH := 500.0
const BOTTOM_MARGIN := 18.0
## Q2 : hauteur laissée au journal replié (bas gauche) quand le parchemin se range à gauche.
const JOURNAL_CLEARANCE := 110.0
const HALO_COLOR := Color(0.85, 0.55, 0.10)
const ARROW_COLOR := Color(0.55, 0.12, 0.08)
const MUTED := "#6b5a40"

var panel: PanelContainer
var title_label: Label
var progress_label: Label
var text_label: RichTextLabel
var objective_label: RichTextLabel
var continue_button: Button
var skip_step_button: Button
var skip_all_button: Button

## Cible courante : `{rect: Rect2}` (contrôle) ou `{point: Vector2}` (carte), vide sinon.
var target: Dictionary = {}
var _time := 0.0


var avoid_rects: Array = []


func _ready() -> void:
	name = "Tutorial"
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	if ResourceLoader.exists(THEME_PATH):
		theme = load(THEME_PATH)
	panel = PanelContainer.new()
	panel.name = "StepPanel"
	panel.mouse_filter = Control.MOUSE_FILTER_STOP
	panel.custom_minimum_size = Vector2(PANEL_WIDTH, 0)
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.95, 0.89, 0.74, 0.97)
	style.border_color = Color(0.45, 0.28, 0.12)
	style.set_border_width_all(3)
	style.set_corner_radius_all(6)
	style.set_content_margin_all(14)
	style.shadow_color = Color(0, 0, 0, 0.35)
	style.shadow_size = 6
	panel.add_theme_stylebox_override("panel", style)
	add_child(panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 8)
	panel.add_child(box)
	var header := HBoxContainer.new()
	box.add_child(header)
	title_label = Label.new()
	title_label.add_theme_font_size_override("font_size", 21)
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title_label)
	progress_label = Label.new()
	progress_label.add_theme_font_size_override("font_size", 14)
	progress_label.add_theme_color_override("font_color", Color(0.42, 0.35, 0.25))
	header.add_child(progress_label)
	text_label = _rich(15)
	box.add_child(text_label)
	objective_label = _rich(15)
	box.add_child(objective_label)
	var buttons := HBoxContainer.new()
	buttons.add_theme_constant_override("separation", 10)
	box.add_child(buttons)
	skip_all_button = Button.new()
	skip_all_button.name = "SkipAllButton"
	skip_all_button.text = "Passer le tutoriel"
	skip_all_button.tooltip_text = "Ferme le tutoriel pour de bon (réactivable dans Réglages → Partie)."
	skip_all_button.pressed.connect(func() -> void: skip_all_pressed.emit())
	buttons.add_child(skip_all_button)
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	buttons.add_child(spacer)
	skip_step_button = Button.new()
	skip_step_button.name = "SkipStepButton"
	skip_step_button.text = "Passer l'étape"
	skip_step_button.pressed.connect(func() -> void: skip_step_pressed.emit())
	buttons.add_child(skip_step_button)
	continue_button = Button.new()
	continue_button.name = "ContinueButton"
	continue_button.text = "Continuer"
	continue_button.pressed.connect(func() -> void: continue_pressed.emit())
	buttons.add_child(continue_button)


func _rich(font_size: int) -> RichTextLabel:
	var label := RichTextLabel.new()
	label.bbcode_enabled = true
	label.fit_content = true
	label.scroll_active = false
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(PANEL_WIDTH - 28.0, 0)
	label.add_theme_color_override("default_color", Color(0.22, 0.14, 0.07))
	label.add_theme_font_size_override("normal_font_size", font_size)
	label.add_theme_font_size_override("bold_font_size", font_size + 1)
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return label


## `step` : entrée de `TutorialSteps.steps` ; `index` à partir de 0.
func show_step(step: Dictionary, index: int, total: int) -> void:
	title_label.text = str(step.get("title", ""))
	progress_label.text = "Étape %d / %d" % [index + 1, total]
	var text := str(step.get("text", ""))
	var advice := str(step.get("advice", ""))
	if advice != "":
		text += "\n[color=%s][i]%s[/i][/color]" % [MUTED, advice]
	text_label.text = text
	var manual := bool(step.get("manual", false))
	objective_label.text = "[b]Objectif :[/b] %s" % str(step.get("objective", ""))
	objective_label.visible = not manual
	continue_button.visible = manual
	continue_button.text = "Terminer" if index == total - 1 else "Continuer"
	skip_step_button.visible = not manual
	panel.reset_size()
	show()


## Objectif rempli : bref retour visuel avant l'étape suivante (le contrôleur enchaîne).
func mark_done() -> void:
	objective_label.text = "[color=#2a6a2a][b]✔ Objectif rempli.[/b][/color]"


func set_target(new_target: Dictionary) -> void:
	target = new_target


## Q2 : panneaux ouverts (coordonnées globales) que le parchemin ne doit pas couvrir ; il se
## range alors dans l'espace libre à leur gauche.
func set_avoid(rects: Array) -> void:
	avoid_rects = rects


func _process(delta: float) -> void:
	_time += delta
	if visible:
		# Calé en bas au centre, quelle que soit la hauteur du texte de l'étape ; Q2 : à gauche
		# d'un panneau ouvert qu'il couvrirait (liste des bâtiments, onglets…).
		var spot := Vector2(roundf((size.x - panel.size.x) * 0.5), size.y - panel.size.y - BOTTOM_MARGIN)
		var origin := get_global_rect().position
		var left_edge := size.x
		for rect in avoid_rects:
			var local := Rect2((rect as Rect2).position - origin, (rect as Rect2).size)
			if local.intersects(Rect2(spot, panel.size)):
				left_edge = minf(left_edge, local.position.x)
		if left_edge < size.x:
			spot.x = maxf(16.0, roundf((left_edge - panel.size.x) * 0.5))
			spot.y = maxf(16.0, spot.y - JOURNAL_CLEARANCE)
		panel.position = spot
		queue_redraw()


func _draw() -> void:
	if target.is_empty():
		return
	var pulse := 0.55 + 0.45 * sin(_time * 5.0)
	var halo := HALO_COLOR
	halo.a = 0.5 + 0.5 * pulse
	var aim: Vector2
	var radius := 0.0
	if target.has("rect"):
		var rect: Rect2 = target["rect"]
		rect = rect.grow(5.0 + 3.0 * pulse)
		draw_rect(rect, halo, false, 4.0)
		var glow := halo
		glow.a *= 0.25
		draw_rect(rect.grow(4.0), glow, false, 6.0)
		aim = rect.get_center()
		radius = minf(rect.size.x, rect.size.y) * 0.5
		aim = _rect_edge_toward(rect, _panel_anchor(aim))
	elif target.has("point"):
		aim = target["point"]
		radius = 30.0 + 6.0 * pulse
		draw_arc(aim, radius, 0.0, TAU, 48, halo, 4.0, true)
		var glow := halo
		glow.a *= 0.3
		draw_arc(aim, radius + 7.0, 0.0, TAU, 48, glow, 5.0, true)
		aim += (_panel_anchor(aim) - aim).normalized() * (radius + 4.0)
	else:
		return
	_draw_arrow(_panel_anchor(aim), aim)


## Point du parchemin d'où part la flèche (bord le plus proche de la cible).
func _panel_anchor(toward: Vector2) -> Vector2:
	var rect := panel.get_global_rect()
	rect.position -= get_global_rect().position
	return _rect_edge_toward(rect, toward)


static func _rect_edge_toward(rect: Rect2, toward: Vector2) -> Vector2:
	var center := rect.get_center()
	var direction := toward - center
	if direction.length() < 1.0:
		return center
	var half := rect.size * 0.5
	var scale_x := half.x / absf(direction.x) if absf(direction.x) > 0.001 else INF
	var scale_y := half.y / absf(direction.y) if absf(direction.y) > 0.001 else INF
	return center + direction * minf(minf(scale_x, scale_y), 1.0)


func _draw_arrow(from: Vector2, to: Vector2) -> void:
	var direction := to - from
	var length := direction.length()
	if length < 30.0:
		return
	direction /= length
	var shaft_end := to - direction * 16.0
	var shadow := Color(0, 0, 0, 0.25)
	draw_line(from + Vector2(2, 2), shaft_end + Vector2(2, 2), shadow, 6.0, true)
	draw_line(from, shaft_end, ARROW_COLOR, 5.0, true)
	var normal := Vector2(-direction.y, direction.x)
	var head := PackedVector2Array([to, shaft_end + normal * 11.0, shaft_end - normal * 11.0])
	draw_colored_polygon(head, ARROW_COLOR)
