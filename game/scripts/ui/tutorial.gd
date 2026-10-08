class_name TutorialOverlay
extends Control

## F8 — affichage du tutoriel : parchemin d'étape (titre, consigne, conseil historique,
## objectif, boutons « Continuer » / « Passer l'étape » / « Plus tard » / « Passer le
## tutoriel », sommaire des étapes) et, par-dessus la carte, un cadre doré pulsé autour de la
## cible de l'étape. La couche entière laisse passer la souris (seul le parchemin la capte).
## Ne connaît ni la simulation ni la carte : `TutorialController` fournit l'étape
## (`show_step`), la liste des étapes (`set_steps`) et la cible à chaque image (`set_target`).
##
## Lot UX2 (audit A3, U1 et U2) : le parchemin évite la cible (placement automatique du côté
## libre de l'écran, `place_panel`) ; la surbrillance dorée remplace la flèche rouge ; elle ne
## pulse pas quand le réglage `access/reduce_motion` est coché.

signal continue_pressed
signal skip_step_pressed
signal skip_all_pressed
## UX2 : « Plus tard » (le guide se range, reprise à la même étape).
signal later_pressed
## UX2 : clic sur une étape du sommaire (`index` à partir de 0).
signal step_chosen(index: int)

const THEME_PATH := "res://scenes/ui/parchment_theme.tres"
const PANEL_WIDTH := 500.0
const BOTTOM_MARGIN := 18.0
## Marges du placement : bord de l'écran, écart à la cible, bande de la barre du haut.
const SCREEN_MARGIN := 16.0
const TARGET_GAP := 22.0
const TOP_RESERVED := 60.0
## Rayon de la surbrillance d'un point de la carte (armée, ville).
const POINT_RADIUS := 34.0
const TOC_MIN_HEIGHT := 90.0
const HALO_COLOR := Color(0.93, 0.73, 0.26)
const HALO_SHADOW := Color(0.18, 0.10, 0.03, 0.55)
const MUTED := "#6b5a40"

var panel: PanelContainer
var title_label: Label
var progress_label: Label
var text_label: RichTextLabel
var objective_label: RichTextLabel
var continue_button: Button
var skip_step_button: Button
var skip_all_button: Button
var later_button: Button
var toc_button: Button
var toc_box: VBoxContainer
## VN lot 3 : le sommaire défile quand le parchemin ne tient pas dans la hauteur de l'écran.
var toc_scroll: ScrollContainer

## Cible courante : `{rect: Rect2}` (contrôle) ou `{point: Vector2}` (carte), vide sinon.
var target: Dictionary = {}
var _time := 0.0
## Q2 : panneaux ouverts (coordonnées globales) que le parchemin ne doit pas couvrir.
var avoid_rects: Array = []
## UX2 : titres des étapes (sommaire) et étape affichée.
var step_titles: PackedStringArray = PackedStringArray()
var current_index := -1
## VN : les fenêtres modales (`UiZones.Zone.MODAL` : rencontre, Codex…) sont centrées et grandes ;
## le parchemin les masquerait ou les cacherait. Il s'efface donc tant qu'une est ouverte, sauf
## pour les étapes dont l'objectif se joue dans une modale (`modal_ok` : technologies, cour).
var modal_ok := false
var _hidden_by_modal := false
## Vrai tant qu'aucun placement n'a été calculé pour l'étape (le parchemin ne saute pas ensuite
## tant que sa place reste libre).
var _needs_placement := true


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
	panel.add_theme_stylebox_override("panel", HudStyle.panel_box(16))
	add_child(panel)
	var box := UiBuild.vbox(8, panel)
	var header := UiBuild.hbox(8, box)
	title_label = Label.new()
	title_label.add_theme_font_size_override("font_size", UiType.size(UiType.HEADING))
	title_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	title_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	header.add_child(title_label)
	progress_label = Label.new()
	progress_label.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	progress_label.add_theme_color_override("font_color", Color(0.42, 0.35, 0.25))
	progress_label.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	header.add_child(progress_label)
	toc_button = UiBuild.button("☰ Étapes")
	toc_button.name = "TocButton"
	toc_button.toggle_mode = true
	toc_button.focus_mode = Control.FOCUS_NONE
	toc_button.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	toc_button.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	TooltipHost.attach_plain(toc_button, "tutorial_toc")
	toc_button.toggled.connect(func(on: bool) -> void: set_toc_open(on))
	header.add_child(toc_button)
	toc_box = UiBuild.vbox(0)
	toc_box.name = "Toc"
	toc_scroll = ScrollContainer.new()
	toc_scroll.name = "TocScroll"
	toc_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	toc_scroll.hide()
	toc_scroll.add_child(toc_box)
	toc_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_child(toc_scroll)
	text_label = _rich(15)
	text_label.mouse_filter = Control.MOUSE_FILTER_PASS  # BP1 : survol des liens du Codex
	box.add_child(text_label)
	# BP1 : mots du Codex cliquables (bulles imbriquées) dans le texte de l'étape.
	var bubbles := get_node_or_null("/root/CodexBubbles")
	if bubbles != null:
		bubbles.call("attach", text_label)
	objective_label = _rich(15)
	box.add_child(objective_label)
	var buttons := UiBuild.hbox(8, box)
	skip_all_button = UiBuild.button("Passer le tutoriel")
	skip_all_button.name = "SkipAllButton"
	TooltipHost.attach_plain(skip_all_button, "tutorial_skip_all")
	skip_all_button.pressed.connect(func() -> void: skip_all_pressed.emit())
	buttons.add_child(skip_all_button)
	later_button = UiBuild.button("Plus tard")
	later_button.name = "LaterButton"
	TooltipHost.attach_plain(later_button, "tutorial_later")
	later_button.pressed.connect(func() -> void: later_pressed.emit())
	buttons.add_child(later_button)
	var spacer := Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	buttons.add_child(spacer)
	skip_step_button = UiBuild.button("Passer l'étape", func() -> void: skip_step_pressed.emit())
	skip_step_button.name = "SkipStepButton"
	buttons.add_child(skip_step_button)
	continue_button = UiBuild.button("Continuer", func() -> void: continue_pressed.emit())
	continue_button.name = "ContinueButton"
	buttons.add_child(continue_button)
	for button: Button in [skip_all_button, later_button, skip_step_button]:
		button.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))


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


## UX2 : titres des étapes du sommaire (dans l'ordre).
func set_steps(titles: PackedStringArray) -> void:
	step_titles = titles
	_rebuild_toc()


## `step` : entrée de `TutorialSteps.steps` ; `index` à partir de 0.
func show_step(step: Dictionary, index: int, total: int) -> void:
	current_index = index
	modal_ok = bool(step.get("modal_ok", false))
	title_label.text = str(step.get("title", ""))
	progress_label.text = "Étape %d / %d" % [index + 1, total]
	var text := CodexText.format(str(step.get("text", "")), true)
	var advice := str(step.get("advice", ""))
	if advice != "":
		text += "\n[color=%s][i]%s[/i][/color]" % [MUTED, CodexText.format(advice, true)]
	text_label.text = text
	var manual := bool(step.get("manual", false))
	objective_label.text = "[b]Objectif :[/b] %s" % str(step.get("objective", ""))
	objective_label.visible = not manual
	continue_button.visible = manual
	continue_button.text = "Terminer" if index == total - 1 else "Continuer"
	skip_step_button.visible = not manual
	set_toc_open(false)
	_rebuild_toc()
	panel.reset_size()
	_needs_placement = true
	show()


## Objectif rempli : bref retour visuel avant l'étape suivante (le contrôleur enchaîne).
func mark_done() -> void:
	objective_label.text = "[color=#2a6a2a][b]%s Objectif rempli.[/b][/color]" % InkGlyph.bbcode("glyph_check", "✔")


func set_target(new_target: Dictionary) -> void:
	target = new_target


## Q2 : panneaux ouverts (coordonnées globales) que le parchemin ne doit pas couvrir.
func set_avoid(rects: Array) -> void:
	avoid_rects = rects


# --- Sommaire (UX2) -------------------------------------------------------------------


func set_toc_open(open: bool) -> void:
	if toc_button.button_pressed != open:
		toc_button.set_pressed_no_signal(open)
	toc_scroll.visible = open
	_fit_toc_height()
	panel.reset_size()
	_needs_placement = true


func toc_open() -> bool:
	return toc_scroll.visible


## VN lot 3 : hauteur du sommaire bornée à la place restante sous la barre du haut (le reste du
## parchemin garde sa taille) ; hors de cette borne, le sommaire défile.
func _fit_toc_height() -> void:
	if not toc_scroll.visible:
		toc_scroll.custom_minimum_size.y = 0.0
		return
	var natural := toc_box.get_combined_minimum_size().y
	var others := panel.get_combined_minimum_size().y - toc_scroll.custom_minimum_size.y
	var room := size.y - TOP_RESERVED - BOTTOM_MARGIN - others
	var wanted := clampf(room, TOC_MIN_HEIGHT, maxf(natural, TOC_MIN_HEIGHT)) if size.y > 0.0 else natural
	wanted = minf(wanted, natural)
	if not is_equal_approx(wanted, toc_scroll.custom_minimum_size.y):
		toc_scroll.custom_minimum_size.y = wanted
		panel.reset_size()
		_needs_placement = true


func _rebuild_toc() -> void:
	for child in toc_box.get_children():
		toc_box.remove_child(child)
		child.queue_free()
	var plain := StyleBoxEmpty.new()
	plain.content_margin_left = 6
	plain.content_margin_top = 2
	plain.content_margin_bottom = 2
	var hover := StyleBoxFlat.new()
	hover.bg_color = Color(0.85, 0.74, 0.52, 0.6)
	hover.set_corner_radius_all(3)
	hover.content_margin_left = 6
	hover.content_margin_top = 2
	hover.content_margin_bottom = 2
	for index in step_titles.size():
		var row := Button.new()
		row.flat = true
		row.focus_mode = Control.FOCUS_NONE
		row.alignment = HORIZONTAL_ALIGNMENT_LEFT
		row.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
		for state in ["normal", "pressed", "focus", "disabled"]:
			row.add_theme_stylebox_override(state, plain)
		row.add_theme_stylebox_override("hover", hover)
		row.add_theme_stylebox_override("hover_pressed", hover)
		var mark := "✔" if index < current_index else ("▸" if index == current_index else "  ")
		row.text = "%s %d. %s" % [mark, index + 1, step_titles[index]]
		if index == current_index:
			row.add_theme_color_override("font_color", Color(0.55, 0.12, 0.08))
		elif index < current_index:
			row.add_theme_color_override("font_color", Color(0.42, 0.35, 0.25))
		TooltipHost.attach_plain(row, "tutorial_go_to_step")
		row.pressed.connect(func() -> void: step_chosen.emit(index))
		toc_box.add_child(row)


# --- Placement du parchemin (UX2) ------------------------------------------------------


## Rectangle écran (coordonnées locales) de la cible courante, ou rectangle vide.
func target_rect() -> Rect2:
	var origin := get_global_rect().position
	if target.has("rect"):
		var rect: Rect2 = target["rect"]
		return Rect2(rect.position - origin, rect.size)
	if target.has("point"):
		var point: Vector2 = target["point"]
		return Rect2(point - origin - Vector2.ONE * POINT_RADIUS, Vector2.ONE * POINT_RADIUS * 2.0)
	return Rect2()


## Coin haut gauche du parchemin (`panel_size`) dans `view` : en bas au centre par défaut ;
## s'il couvre la cible (`target`, agrandie de `TARGET_GAP`) ou un panneau de `avoid`, du côté
## où l'écran est le plus libre autour de la cible (dessous, dessus, droite, gauche), puis
## dans un coin. `current` (si fourni) est gardé tant qu'il reste libre : le parchemin ne saute
## pas à chaque mouvement de caméra. Faute de place libre : la position qui couvre le moins.
static func place_panel(view: Rect2, panel_size: Vector2, target: Rect2, avoid: Array = [], current: Variant = null) -> Vector2:
	var area := Rect2(view.position + Vector2(SCREEN_MARGIN, TOP_RESERVED),
		view.size - Vector2(SCREEN_MARGIN * 2.0, TOP_RESERVED + BOTTOM_MARGIN))
	var candidates: Array[Vector2] = []
	if current is Vector2:
		candidates.append(current)
	var bottom_y := area.end.y - panel_size.y
	var center_x := area.position.x + (area.size.x - panel_size.x) * 0.5
	candidates.append(Vector2(center_x, bottom_y))
	var has_target := target.size.x > 0.0 and target.size.y > 0.0
	var grown := target.grow(TARGET_GAP) if has_target else Rect2()
	if has_target:
		var middle := grown.get_center()
		var sides := [
			[area.end.y - grown.end.y - panel_size.y, Vector2(middle.x - panel_size.x * 0.5, grown.end.y)],
			[grown.position.y - area.position.y - panel_size.y, Vector2(middle.x - panel_size.x * 0.5, grown.position.y - panel_size.y)],
			[area.end.x - grown.end.x - panel_size.x, Vector2(grown.end.x, middle.y - panel_size.y * 0.5)],
			[grown.position.x - area.position.x - panel_size.x, Vector2(grown.position.x - panel_size.x, middle.y - panel_size.y * 0.5)],
		]
		sides.sort_custom(func(a: Array, b: Array) -> bool: return float(a[0]) > float(b[0]))
		for side in sides:
			candidates.append(side[1])
	candidates.append(Vector2(area.position.x, bottom_y))
	candidates.append(Vector2(area.end.x - panel_size.x, bottom_y))
	candidates.append(Vector2(area.position.x, area.position.y))
	candidates.append(Vector2(area.end.x - panel_size.x, area.position.y))
	candidates.append(Vector2(center_x, area.position.y))
	var best := Vector2.ZERO
	var best_cost := INF
	for index in candidates.size():
		var spot := _clamp_into(candidates[index], panel_size, area)
		var cost := _overlap_cost(Rect2(spot, panel_size), grown, avoid)
		if index == 0 and current is Vector2 and not spot.is_equal_approx(current):
			continue  # position courante sortie de l'écran : recalcul
		if cost <= 0.0:
			return spot
		if cost < best_cost:
			best_cost = cost
			best = spot
	return best


static func _clamp_into(spot: Vector2, panel_size: Vector2, area: Rect2) -> Vector2:
	return Vector2(
		roundf(clampf(spot.x, area.position.x, maxf(area.position.x, area.end.x - panel_size.x))),
		roundf(clampf(spot.y, area.position.y, maxf(area.position.y, area.end.y - panel_size.y))))


## Surface couverte : la cible compte triple (la cacher est pire que couvrir un panneau).
static func _overlap_cost(rect: Rect2, target: Rect2, avoid: Array) -> float:
	var cost := 0.0
	if target.has_area() and rect.intersects(target):
		cost += rect.intersection(target).get_area() * 3.0
	for other in avoid:
		var other_rect := other as Rect2
		if rect.intersects(other_rect):
			cost += rect.intersection(other_rect).get_area()
	return cost


func _process(delta: float) -> void:
	_time += delta
	if not visible:
		return
	var modal_open := false
	if not modal_ok:
		var layout := UiZones.layout()
		modal_open = layout != null and layout.modal_open()
	if modal_open != _hidden_by_modal:
		_hidden_by_modal = modal_open
		panel.visible = not modal_open
		queue_redraw()
	if _hidden_by_modal:
		return
	_fit_toc_height()
	var origin := get_global_rect().position
	var avoid: Array = []
	for rect in avoid_rects:
		avoid.append(Rect2((rect as Rect2).position - origin, (rect as Rect2).size))
	var current: Variant = null if _needs_placement else panel.position
	panel.position = place_panel(Rect2(Vector2.ZERO, size), panel.size, target_rect(), avoid, current)
	_needs_placement = false
	queue_redraw()


# --- Surbrillance (UX2 : cadre doré au lieu de la flèche) ------------------------------


## Intensité du pulsé (0..1) ; fixe quand les animations sont réduites.
func pulse() -> float:
	if Accessibility.reduce_motion():
		return 1.0
	return 0.5 + 0.5 * sin(_time * 4.0)


func _draw() -> void:
	if _hidden_by_modal:
		return
	var rect := target_rect()
	if rect.size.x <= 0.0:
		return
	var strength := pulse()
	var gold := HALO_COLOR
	gold.a = 0.65 + 0.35 * strength
	var glow := HALO_COLOR
	glow.a = 0.12 + 0.22 * strength
	if target.has("point"):
		var center := rect.get_center()
		var radius := POINT_RADIUS + 4.0 * strength
		draw_arc(center, radius + 2.0, 0.0, TAU, 56, HALO_SHADOW, 6.0, true)
		draw_arc(center, radius, 0.0, TAU, 56, gold, 3.5, true)
		draw_arc(center, radius + 7.0, 0.0, TAU, 56, glow, 8.0, true)
		return
	var frame := rect.grow(4.0 + 2.0 * strength)
	draw_rect(frame.grow(2.0), HALO_SHADOW, false, 5.0)
	draw_rect(frame, gold, false, 3.0)
	draw_rect(frame.grow(6.0), glow, false, 8.0)
	# Équerres aux coins : le cadre se lit même sur une carte chargée.
	var arm := minf(18.0, minf(frame.size.x, frame.size.y) * 0.5)
	var outer := frame.grow(3.0)
	for corner: Vector2 in [outer.position, Vector2(outer.end.x, outer.position.y), outer.end, Vector2(outer.position.x, outer.end.y)]:
		var sx := 1.0 if corner.x <= outer.get_center().x else -1.0
		var sy := 1.0 if corner.y <= outer.get_center().y else -1.0
		draw_line(corner, corner + Vector2(arm * sx, 0.0), gold, 4.0, true)
		draw_line(corner, corner + Vector2(0.0, arm * sy), gold, 4.0, true)
