extends CanvasLayer

## Autoload `CodexBubbles` (H2) : bulles imbriquées au-dessus de toute
## l'interface. Une UI branche ses textes par `attach(label)` (un `RichTextLabel` dont le BBCode
## vient de `CodexText.format`) :
##  - survol d'un mot-lien ≥ 0,35 s → bulle fille près de la souris (titre, catégorie, résumé) ;
##  - la souris peut entrer dans la bulle ; ses liens ouvrent la bulle suivante (pile de 6 au
##    plus, la plus ancienne se ferme au-delà) ;
##  - quitter toutes les bulles → fermeture après 0,4 s de grâce ; Échap ferme tout ;
##  - clic gauche sur un lien ou sur une bulle → fiche complète dans la fenêtre Codex ;
##  - clic droit sur un lien → bulle épinglée ; clic droit sur une bulle → épingle / détache ;
##  - touche T (`codex_pin_tooltip`, B1) : verrouille, par priorité, la bulle non épinglée la plus
##    récente (ou celle en attente de survol), l'infobulle riche visible (F2), ou l'infobulle simple
##    du contrôle survolé, convertie en bulle épinglée avec auto-liens. L'événement n'est consommé
##    que si quelque chose a été verrouillé (T ouvre aussi l'arbre des techniques).
## Chaque bulle retient sa bulle parente (méta `parent`) : épingler une bulle épingle ses
## ancêtres, détacher une bulle détache ses descendantes, de sorte que la chaîne reste entière.
## Chaque bulle ouverte marque sa fiche découverte. La fenêtre Codex (touche K, `codex_open`)
## vit sur un calque juste en dessous. Présentation pure : aucune règle de jeu.

signal bubble_opened(id: String)
signal entry_requested(id: String)

const LAYER := 110
const WIDTH := 340.0
const HOVER_DELAY := 0.35
const CLOSE_GRACE := 0.4
const MAX_BUBBLES := 6
const MOUSE_OFFSET := Vector2(14, 18)
const MARGIN := 8.0
const INK := Color(0.22, 0.14, 0.07)
const MUTED := "#6b5a40"
const PINNED_BORDER := Color(0.55, 0.12, 0.10)

## Pile des bulles ouvertes (de la plus ancienne à la plus récente).
var bubbles: Array[PanelContainer] = []
var _pending_id := ""
var _pending_source: Control = null
var _pending_time := 0.0
var _hover_id := ""
var _hover_source: Control = null
var _outside_time := 0.0
var _window_layer: CanvasLayer
var _window: Control
## Contrôle survolé et durée du survol (T n'épingle une infobulle simple qu'une fois affichée).
var _hovered_control: Control = null
var _hovered_time := 0.0


func _ready() -> void:
	layer = LAYER
	process_mode = Node.PROCESS_MODE_ALWAYS
	_window_layer = CanvasLayer.new()
	_window_layer.layer = LAYER - 1
	_window_layer.name = "WindowLayer"
	add_child(_window_layer)


## Branche un `RichTextLabel` (BBCode issu de `CodexText.format`) sur les bulles.
func attach(label: RichTextLabel) -> void:
	if label == null or label.has_meta("codex_attached"):
		return
	label.set_meta("codex_attached", true)
	label.meta_underlined = false  # le soulignement marque seulement les fiches non lues
	label.meta_hover_started.connect(_on_meta_hover_started.bind(label))
	label.meta_hover_ended.connect(_on_meta_hover_ended.bind(label))
	label.meta_clicked.connect(_on_meta_clicked)


## Ouvre la bulle de la fiche `id` à `at` (position de la souris si négative), au-dessus de la
## bulle d'index `parent_index` (-1 : nouvelle pile ; les bulles non épinglées au-dessus se
## ferment). Renvoie la bulle, ou null si la fiche n'existe pas.
func open(id: String, at: Vector2 = Vector2(-1, -1), parent_index: int = -1, pinned: bool = false) -> PanelContainer:
	var codex := CodexText.store()
	if codex == null or not bool(codex.call("has_entry", id)):
		return null
	_trim_above(parent_index)
	var parent: PanelContainer = bubbles[parent_index] if parent_index >= 0 and parent_index < bubbles.size() else null
	var existing := _child_of(parent, id)
	if existing != null:
		if pinned:
			set_pinned(existing, true)
		return existing
	var bubble := _make_bubble(id, _entry_bbcode(id), false, parent)
	_push(bubble, at)
	if pinned:
		set_pinned(bubble, true)
	codex.call("discover", id)
	bubble_opened.emit(id)
	return bubble


## Bulle au texte libre (infobulle épinglée), toujours épinglée ; `entry_id` : fiche ouverte par
## un clic sur la bulle (titre lié d'une infobulle riche), vide sinon.
func open_text(bbcode: String, at: Vector2 = Vector2(-1, -1), entry_id: String = "") -> PanelContainer:
	var bubble := _make_bubble(entry_id, bbcode, true)
	bubble.set_meta("free_text", true)
	_push(bubble, at)
	return bubble


## Touche T : verrouille la bulle ou l'infobulle visible la plus récente. Renvoie true si
## quelque chose a été verrouillé (l'appelant ne consomme l'événement que dans ce cas).
func pin_current() -> bool:
	if pin_top_bubble() or pin_native_tooltip():
		return true
	if _hovered_control == null or _hovered_time < _tooltip_delay():
		return false
	return pin_hovered_tooltip()


## Épingle la bulle en attente de survol (ouverte aussitôt) ou la bulle non épinglée la plus récente.
func pin_top_bubble() -> bool:
	if _pending_id != "":
		var id := _pending_id
		var source := _pending_source
		_pending_id = ""
		_pending_source = null
		if open(id, -Vector2.ONE, _index_of_source(source), true) != null:
			return true
	for index in range(bubbles.size() - 1, -1, -1):
		if not bubbles[index].get_meta("pinned", false):
			set_pinned(bubbles[index], true)
			return true
	return false


## Épingle l'infobulle riche affichée (F2) : bulle interactive au même endroit.
func pin_native_tooltip() -> bool:
	var panel := RichTooltip.visible_panel()
	if panel == null:
		return false
	var window := panel.get_window()
	var at := Vector2(window.position)
	if not window.is_embedded():
		at -= Vector2(get_tree().root.position)
	window.hide()
	open_text(RichTooltip.last_bbcode, at, RichTooltip.title_entry(RichTooltip.last_bbcode))
	return true


## Épingle l'infobulle simple du contrôle survolé (texte converti avec auto-liens).
func pin_hovered_tooltip() -> bool:
	var control := get_viewport().gui_get_hovered_control()
	if control == null:
		return false
	return pin_control_tooltip(control, control.get_local_mouse_position())


## Bulle épinglée tirée de l'infobulle de `control` au point `local_pos` (remonte aux parents
## comme Godot tant que le contrôle laisse passer la souris). False si aucune infobulle.
func pin_control_tooltip(control: Control, local_pos: Vector2 = Vector2.ZERO) -> bool:
	if control == null or _index_of_source(control) >= 0 or control is PanelContainer and bubbles.has(control as PanelContainer):
		return false
	var text := ""
	var current := control
	var pos := local_pos
	while current != null:
		text = current.get_tooltip(pos)
		if text != "" or current.mouse_filter == Control.MOUSE_FILTER_STOP or current.top_level:
			break
		pos = current.get_transform() * pos
		current = current.get_parent() as Control
	if text.strip_edges() == "":
		return false
	var at := _hide_native_tooltips()
	var formatted := CodexText.format(text, true)
	open_text(formatted, at, RichTooltip.title_entry(formatted))
	return true


## Bulle parente de `bubble` (null pour une racine).
func parent_of(bubble: PanelContainer) -> PanelContainer:
	var parent: Variant = bubble.get_meta("parent") if bubble.has_meta("parent") else null
	if parent is PanelContainer and is_instance_valid(parent) and bubbles.has(parent):
		return parent
	return null


func close_all() -> void:
	for bubble in bubbles:
		bubble.queue_free()
	bubbles.clear()
	_pending_id = ""
	_outside_time = 0.0


func close_unpinned() -> void:
	for bubble in bubbles.duplicate():
		if not bubble.get_meta("pinned", false):
			_remove(bubble)


func bubble_count() -> int:
	return bubbles.size()


## Id de la fiche de la bulle au sommet de la pile (vide si aucune ou texte libre).
func top_id() -> String:
	return str(bubbles[-1].get_meta("codex_id", "")) if not bubbles.is_empty() else ""


## Épingle (et ses ancêtres, pour garder la chaîne) ou détache (et ses descendantes) `bubble`.
func set_pinned(bubble: PanelContainer, pinned: bool) -> void:
	if pinned:
		var ancestor := parent_of(bubble)
		if ancestor != null and not ancestor.get_meta("pinned", false):
			set_pinned(ancestor, true)
	else:
		for child in bubbles.duplicate():
			if parent_of(child) == bubble and child.get_meta("pinned", false):
				set_pinned(child, false)
	_apply_pinned(bubble, pinned)


func _apply_pinned(bubble: PanelContainer, pinned: bool) -> void:
	bubble.set_meta("pinned", pinned)
	var style := RichTooltip.panel_style()
	if pinned:
		style.border_color = PINNED_BORDER
		style.set_border_width_all(3)
	bubble.add_theme_stylebox_override("panel", style)
	var footer := bubble.find_child("Footer", true, false) as Label
	if footer != null:
		footer.text = _footer_text(str(bubble.get_meta("codex_id", "")), pinned)


# --- Fenêtre Codex ---------------------------------------------------------------------------


## Fenêtre Codex (créée à la demande).
func window() -> Control:
	# U11 : sur la carte, la fenêtre commune « Codex » (onglet Histoire) remplace la fenêtre seule.
	var hub := get_tree().get_first_node_in_group("codex_hub") if is_inside_tree() else null
	if hub != null and hub.get("codex_window") != null:
		return hub.get("codex_window")
	if _window == null:
		_window = CodexWindow.new()
		_window.name = "CodexWindow"
		_window.hide()
		_window_layer.add_child(_window)
	return _window


func is_window_open() -> bool:
	var current := window()
	return current != null and current.visible


## Ouvre la fenêtre Codex sur `id` (liste seule si vide) ; les bulles non épinglées se ferment.
func open_entry(id: String = "") -> void:
	close_unpinned()
	window().call("open", id)
	if id != "":
		entry_requested.emit(id)


func toggle_window() -> void:
	if is_window_open():
		window().hide()
	else:
		open_entry()


# --- Entrées ---------------------------------------------------------------------------------


func _input(event: InputEvent) -> void:
	if event.is_action_pressed("codex_pin_tooltip") and pin_current():
		get_viewport().set_input_as_handled()
	elif event.is_action_pressed("ui_cancel"):
		if not bubbles.is_empty():
			close_all()
			get_viewport().set_input_as_handled()
		elif is_window_open():
			window().hide()
			get_viewport().set_input_as_handled()
	elif event is InputEventMouseButton and event.pressed:
		_on_mouse_pressed(event as InputEventMouseButton)


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("codex_open"):
		toggle_window()
		get_viewport().set_input_as_handled()


func _on_mouse_pressed(event: InputEventMouseButton) -> void:
	var under := _bubble_under_mouse()
	if event.button_index == MOUSE_BUTTON_RIGHT:
		if _hover_id != "":
			open(_hover_id, -Vector2.ONE, _index_of_source(_hover_source), true)
			get_viewport().set_input_as_handled()
		elif under != null:
			set_pinned(under, not under.get_meta("pinned", false))
			get_viewport().set_input_as_handled()
	elif event.button_index == MOUSE_BUTTON_LEFT and under != null and _hover_id == "":
		var id := str(under.get_meta("codex_id", ""))
		if id != "":
			open_entry(id)
			get_viewport().set_input_as_handled()


func _on_meta_hover_started(meta: Variant, label: RichTextLabel) -> void:
	var id := CodexText.meta_id(meta)
	if id == "":
		return
	_hover_id = id
	_hover_source = label
	var parent := _index_of_source(label)
	if _child_of(bubbles[parent] if parent >= 0 else null, id) != null:
		_pending_id = ""
		return
	_pending_id = id
	_pending_source = label
	_pending_time = 0.0


func _on_meta_hover_ended(_meta: Variant, label: RichTextLabel) -> void:
	if label == _hover_source:
		_hover_id = ""
		_hover_source = null
	if label == _pending_source:
		_pending_id = ""
		_pending_source = null


func _on_meta_clicked(meta: Variant) -> void:
	var id := CodexText.meta_id(meta)
	if id != "":
		open_entry(id)


func _process(delta: float) -> void:
	var hovered := get_viewport().gui_get_hovered_control()
	if hovered != _hovered_control:
		_hovered_control = hovered
		_hovered_time = 0.0
	else:
		_hovered_time += delta
	if _pending_id != "":
		_pending_time += delta
		if _pending_time >= HOVER_DELAY:
			var id := _pending_id
			_pending_id = ""
			open(id, -Vector2.ONE, _index_of_source(_pending_source))
	var has_unpinned := false
	for bubble in bubbles:
		if not bubble.get_meta("pinned", false):
			has_unpinned = true
			break
	if not has_unpinned:
		_outside_time = 0.0
		return
	if _hover_id != "" or _bubble_under_mouse() != null:
		_outside_time = 0.0
	else:
		_outside_time += delta
		if _outside_time >= CLOSE_GRACE:
			_outside_time = 0.0
			close_unpinned()


# --- Construction et placement ---------------------------------------------------------------


func _entry_bbcode(id: String) -> String:
	var codex := CodexText.store()
	var head := "[b]%s[/b]" % str(codex.call("title", id))
	var meta := PackedStringArray([str(codex.call("category_label", id))])
	var era := str(codex.call("era_label", id))
	if era != "":
		meta.append(era)
	head += "  [color=%s][i]%s[/i][/color]" % [MUTED, " · ".join(meta)]
	var entry: Dictionary = codex.call("entry", id)
	var text := "%s\n%s" % [head, CodexText.format(str(entry.get("summary", "")))]
	var gameplay := first_sentence(str(entry.get("gameplay", "")))
	if gameplay != "":
		text += "\n[i]En jeu :[/i] %s" % CodexText.format(gameplay)
	return text


## Première phrase d'un texte (jusqu'au premier « . », « ! », « ? » ou « … » suivi d'une espace
## ou en fin de texte), hors liens `[[…]]`.
static func first_sentence(text: String) -> String:
	text = text.strip_edges()
	var depth := 0
	for index in text.length():
		var character := text[index]
		if character == "[":
			depth += 1
		elif character == "]":
			depth = maxi(0, depth - 1)
		elif depth == 0 and ".!?…".contains(character) and (index + 1 == text.length() or text[index + 1] == " "):
			return text.substr(0, index + 1)
	return text


func _footer_text(id: String, pinned: bool) -> String:
	var parts := PackedStringArray()
	if id != "":
		parts.append("Clic : lire la fiche")
	parts.append("Clic droit : détacher" if pinned else "T : maintenir ouverte")
	return " · ".join(parts)


func _make_bubble(id: String, bbcode: String, pinned: bool, parent: PanelContainer = null) -> PanelContainer:
	var bubble := PanelContainer.new()
	bubble.name = "Bubble"
	bubble.theme = load(RichTooltip.THEME_PATH)
	bubble.mouse_filter = Control.MOUSE_FILTER_STOP
	bubble.set_meta("codex_id", id)
	if parent != null:
		bubble.set_meta("parent", parent)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 4)
	box.mouse_filter = Control.MOUSE_FILTER_PASS
	bubble.add_child(box)
	var label := RichTextLabel.new()
	label.name = "Text"
	label.bbcode_enabled = true
	label.fit_content = true
	label.scroll_active = false
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size = Vector2(WIDTH, 0)
	label.mouse_filter = Control.MOUSE_FILTER_PASS
	label.add_theme_color_override("default_color", INK)
	label.add_theme_font_size_override("normal_font_size", 14)
	label.add_theme_font_size_override("bold_font_size", 15)
	label.text = bbcode
	box.add_child(label)
	var footer := Label.new()
	footer.name = "Footer"
	footer.add_theme_font_size_override("font_size", 11)
	footer.add_theme_color_override("font_color", Color(MUTED))
	footer.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	box.add_child(footer)
	attach(label)
	_apply_pinned(bubble, pinned)
	return bubble


func _push(bubble: PanelContainer, at: Vector2) -> void:
	if at.x < 0 or at.y < 0:
		at = get_viewport().get_mouse_position() + MOUSE_OFFSET
	bubble.set_meta("anchor", at)
	bubble.position = at
	add_child(bubble)
	bubbles.append(bubble)
	while bubbles.size() > MAX_BUBBLES:
		_remove(bubbles[0])
	_outside_time = 0.0
	bubble.resized.connect(_clamp.bind(bubble))
	_clamp(bubble)


## Garde la bulle à l'écran : à gauche de l'ancre si elle déborde à droite, remontée sinon.
func _clamp(bubble: PanelContainer) -> void:
	if not is_instance_valid(bubble):
		return
	var area := get_viewport().get_visible_rect().size
	var at: Vector2 = bubble.get_meta("anchor", bubble.position)
	var pos := at
	if pos.x + bubble.size.x > area.x - MARGIN:
		pos.x = at.x - bubble.size.x - MOUSE_OFFSET.x * 2.0
	pos.x = clampf(pos.x, MARGIN, maxf(MARGIN, area.x - bubble.size.x - MARGIN))
	pos.y = clampf(pos.y, MARGIN, maxf(MARGIN, area.y - bubble.size.y - MARGIN))
	bubble.position = pos.floor()


func _trim_above(parent_index: int) -> void:
	for index in range(bubbles.size() - 1, parent_index, -1):
		if not bubbles[index].get_meta("pinned", false):
			_remove(bubbles[index])


## Retire `bubble` ; ses filles restantes (épinglées) sont rattachées à sa propre parente.
func _remove(bubble: PanelContainer) -> void:
	var grandparent := parent_of(bubble)
	bubbles.erase(bubble)
	for child in bubbles:
		if child.has_meta("parent") and child.get_meta("parent") == bubble:
			if grandparent != null:
				child.set_meta("parent", grandparent)
			else:
				child.remove_meta("parent")
	if _hover_source != null and bubble.is_ancestor_of(_hover_source):
		_hover_id = ""
		_hover_source = null
	if _pending_source != null and bubble.is_ancestor_of(_pending_source):
		_pending_id = ""
		_pending_source = null
	bubble.queue_free()


## Bulle fille de `parent` (null : racine) sur la fiche `id`, null si aucune.
func _child_of(parent: PanelContainer, id: String) -> PanelContainer:
	for bubble in bubbles:
		if parent_of(bubble) == parent and not bubble.get_meta("free_text", false) and str(bubble.get_meta("codex_id", "")) == id:
			return bubble
	return null


## Position de l'infobulle native visible (masquée au passage), sinon (-1, -1) : la souris.
func _hide_native_tooltips() -> Vector2:
	var at := -Vector2.ONE
	for node in get_tree().root.find_children("*", "Window", true, false):
		var popup := node as Window
		if popup != null and popup.visible and popup.get_class() == "TooltipPanel":
			at = Vector2(popup.position) if popup.is_embedded() else Vector2(popup.position - get_tree().root.position)
			popup.hide()
	return at


func _tooltip_delay() -> float:
	return float(ProjectSettings.get_setting("gui/timers/tooltip_delay_sec", 0.5))


## Index de la bulle contenant `source`, -1 si `source` est hors des bulles.
func _index_of_source(source: Control) -> int:
	if source == null:
		return -1
	for index in bubbles.size():
		if bubbles[index].is_ancestor_of(source):
			return index
	return -1


func _bubble_under_mouse() -> PanelContainer:
	var mouse := get_viewport().get_mouse_position()
	for index in range(bubbles.size() - 1, -1, -1):
		if bubbles[index].get_global_rect().has_point(mouse):
			return bubbles[index]
	return null
