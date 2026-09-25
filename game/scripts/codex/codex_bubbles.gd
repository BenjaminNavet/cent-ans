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
##  - touche T (`codex_pin_tooltip`) sur une infobulle riche (F2) → bulle épinglée équivalente.
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
	if not bubbles.is_empty() and str(bubbles[-1].get_meta("codex_id", "")) == id:
		return bubbles[-1]
	var bubble := _make_bubble(id, _entry_bbcode(id), pinned)
	_push(bubble, at)
	codex.call("discover", id)
	bubble_opened.emit(id)
	return bubble


## Bulle au texte libre (infobulle épinglée), toujours épinglée.
func open_text(bbcode: String, at: Vector2 = Vector2(-1, -1)) -> PanelContainer:
	var bubble := _make_bubble("", bbcode, true)
	_push(bubble, at)
	return bubble


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
	open_text(RichTooltip.last_bbcode, at)
	return true


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


func set_pinned(bubble: PanelContainer, pinned: bool) -> void:
	bubble.set_meta("pinned", pinned)
	# Épinglée : page à bande d'or (lot UI1) plutôt que simple note marginale.
	var style: StyleBox = HudStyle.panel_box(10) if pinned else RichTooltip.panel_style()
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
	if event.is_action_pressed("codex_pin_tooltip") and pin_native_tooltip():
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
			var parent := _index_of_source(_hover_source)
			var opened := open(_hover_id, -Vector2.ONE, parent, true)
			if opened != null:
				set_pinned(opened, true)
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
	if parent + 1 < bubbles.size() and str(bubbles[parent + 1].get_meta("codex_id", "")) == id:
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
	var summary := CodexText.format(str((codex.call("entry", id) as Dictionary).get("summary", "")))
	return "%s\n%s" % [head, summary]


func _footer_text(id: String, pinned: bool) -> String:
	var parts := PackedStringArray()
	if id != "":
		parts.append("Clic : lire la fiche")
	parts.append("Clic droit : détacher" if pinned else "Clic droit : épingler")
	return " · ".join(parts)


func _make_bubble(id: String, bbcode: String, pinned: bool) -> PanelContainer:
	var bubble := PanelContainer.new()
	bubble.name = "Bubble"
	bubble.theme = load(RichTooltip.THEME_PATH)
	bubble.mouse_filter = Control.MOUSE_FILTER_STOP
	bubble.set_meta("codex_id", id)
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
	set_pinned(bubble, pinned)
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


func _remove(bubble: PanelContainer) -> void:
	bubbles.erase(bubble)
	if _hover_source != null and bubble.is_ancestor_of(_hover_source):
		_hover_id = ""
		_hover_source = null
	if _pending_source != null and bubble.is_ancestor_of(_pending_source):
		_pending_id = ""
		_pending_source = null
	bubble.queue_free()


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
