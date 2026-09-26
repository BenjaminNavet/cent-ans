class_name FamilyTreeView
extends ScrollContainer

## Arbre familial graphique (lot C3, onglet « Arbre familial » du panneau Cour) : registre
## parchemin, médaillons de portrait reliés par des traits à l'encre. Génération courante
## centrée, ascendants au-dessus, descendants en dessous, conjoints à côté (double trait).
## Défunts grisés, héritier couronné, dirigeant cerclé d'or. Clic = fiche du personnage,
## clic droit = recentrer l'arbre. Molette + Ctrl ou boutons = zoom ; glisser = défilement.
## Aucune règle ici : les liens viennent de `CampaignSim.get_family_tree` (lecture seule).

signal character_selected(character_id: String)
signal recenter_requested(character_id: String)

const NODE_SIZE := Vector2(132, 156)
const PORTRAIT_RADIUS := 42.0
const SIBLING_GAP := 26.0
const SPOUSE_GAP := 34.0
const ROW_STEP := 206.0
const MARGIN := Vector2(36, 30)
const ZOOM_MIN := 0.45
const ZOOM_MAX := 1.6

## id → FamilyTreeNode (médaillon), pour le dessin des liens et les tests.
var medallions: Dictionary = {}
var zoom: float = 1.0
var root_id: String = ""
var ruler_id: String = ""
var heir_id: String = ""

var _canvas: FamilyTreeCanvas
var _nodes: Dictionary = {}  # id → entrée de `get_family_tree`
var _positions: Dictionary = {}  # id → coin haut-gauche (zoom 1)
var _couples: Array = []  # [[a, b]] conjoints dessinés côte à côte
var _drag_from: Vector2 = Vector2.INF
var _offset_x: float = 0.0


func _init() -> void:
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	size_flags_vertical = Control.SIZE_EXPAND_FILL
	_canvas = FamilyTreeCanvas.new()
	_canvas.view = self
	add_child(_canvas)
	resized.connect(_update_offset)


## `tree` : `CampaignSim.get_family_tree(id, up, down)` (`{root, ruler, heir, nodes}`).
func show_tree(tree: Dictionary) -> void:
	root_id = str(tree.get("root", ""))
	ruler_id = str(tree.get("ruler", ""))
	heir_id = str(tree.get("heir", ""))
	_nodes.clear()
	for entry in tree.get("nodes", []):
		_nodes[str(entry.get("id", ""))] = entry
	_layout()
	_rebuild()
	_update_offset.call_deferred()
	center_on.call_deferred(root_id)


## Dates de vie d'un nœud ou d'une fiche (`birth_year`, `death_year`, `alive`, `age`) :
## « ° 1312 · 25 ans » (vivant), « 1310–1346 » (défunt), « 1310–† » (année de décès inconnue).
static func life_dates(entry: Dictionary) -> String:
	var birth := int(entry.get("birth_year", 0))
	if bool(entry.get("alive", true)):
		return "° %d · %d ans" % [birth, int(entry.get("age", 0))]
	var death := int(entry.get("death_year", 0))
	if death > 0:
		return "%d–%d" % [birth, death]
	return "%d–†" % birth


func node_count() -> int:
	return medallions.size()


## Nombre de générations affichées (lignes distinctes).
func generation_count() -> int:
	var seen: Dictionary = {}
	for entry in _nodes.values():
		seen[int(entry.get("generation", 0))] = true
	return seen.size()


func set_zoom(value: float) -> void:
	var previous_center := (Vector2(scroll_horizontal, scroll_vertical) + size * 0.5) / zoom
	zoom = clampf(value, ZOOM_MIN, ZOOM_MAX)
	_rebuild()
	_update_offset.call_deferred()
	_scroll_to.call_deferred(previous_center * zoom - size * 0.5)


func _scroll_to(offset: Vector2) -> void:
	scroll_horizontal = int(offset.x)
	scroll_vertical = int(offset.y)


func center_on(character_id: String) -> void:
	if not _positions.has(character_id):
		return
	var center: Vector2 = _screen(character_id) + NODE_SIZE * 0.5 * zoom
	scroll_horizontal = int(center.x - size.x * 0.5)
	scroll_vertical = int(center.y - size.y * 0.5)


# --- Disposition -------------------------------------------------------------------------------


## Parent « de rattachement » d'un nœud de sang : père affiché, sinon mère affichée.
func _primary_parent(id: String) -> String:
	var entry: Dictionary = _nodes[id]
	for key in ["father", "mother"]:
		var parent := str(entry.get(key, ""))
		if parent != "" and _nodes.has(parent):
			return parent
	return ""


## Conjoint affiché à côté de `id` (sans parent affiché lui-même, sinon il a sa propre branche).
func _attached_spouse(id: String) -> String:
	var spouse := str((_nodes[id] as Dictionary).get("spouse", ""))
	if spouse == "" or not _nodes.has(spouse):
		return ""
	if _primary_parent(spouse) != "":
		return ""
	return spouse


func _layout() -> void:
	_positions.clear()
	_couples.clear()
	# Unités : un nœud « tête » (+ conjoint rattaché). Une femme rattachée à son mari n'est pas
	# une tête ; pour un couple de deux nœuds sans parents, l'homme (ou le plus âgé) est la tête.
	var attached: Dictionary = {}  # conjoint → tête
	var ids: Array = _nodes.keys()
	ids.sort_custom(_older_first)
	for id in ids:
		if attached.has(id):
			continue
		var spouse := _attached_spouse(id)
		if spouse == "" or attached.has(spouse):
			continue
		if _primary_parent(id) == "" and str((_nodes[id] as Dictionary).get("sex", "")) == "female" \
				and str((_nodes[spouse] as Dictionary).get("sex", "")) == "male":
			continue  # l'époux sera la tête
		attached[spouse] = id
	var children_of: Dictionary = {}  # tête → [têtes enfants]
	var heads: Array = []
	for id in ids:
		if attached.has(id):
			continue
		heads.append(id)
		children_of[id] = []
	var roots: Array = []
	for id in heads:
		var parent := _primary_parent(id)
		if parent != "" and attached.has(parent):
			parent = attached[parent]
		if parent != "" and children_of.has(parent):
			(children_of[parent] as Array).append(id)
		else:
			roots.append(id)
	var spouse_of: Dictionary = {}
	for spouse in attached:
		spouse_of[attached[spouse]] = spouse
		_couples.append([attached[spouse], spouse])
	# Largeurs de sous-arbres puis placement récursif, racines côte à côte.
	var widths: Dictionary = {}
	for id in roots:
		_subtree_width(id, children_of, spouse_of, widths)
	var min_generation := 0
	for entry in _nodes.values():
		min_generation = mini(min_generation, int(entry.get("generation", 0)))
	var x := MARGIN.x
	roots.sort_custom(func(a: String, b: String) -> bool:
		var ga := int((_nodes[a] as Dictionary).get("generation", 0))
		var gb := int((_nodes[b] as Dictionary).get("generation", 0))
		return ga < gb if ga != gb else _older_first(a, b))
	for id in roots:
		_place(id, x, children_of, spouse_of, widths, min_generation)
		x += float(widths[id]) + SIBLING_GAP * 2.0
	# Liens de couple non rattachés (deux branches mariées) : dessinés aussi.
	for id in ids:
		var spouse := str((_nodes[id] as Dictionary).get("spouse", ""))
		if spouse != "" and _nodes.has(spouse) and not attached.has(id) and not attached.has(spouse) and id < spouse:
			_couples.append([id, spouse])


func _older_first(a: String, b: String) -> bool:
	var ya := int((_nodes[a] as Dictionary).get("birth_year", 0))
	var yb := int((_nodes[b] as Dictionary).get("birth_year", 0))
	return ya < yb if ya != yb else a < b


func _unit_width(id: String, spouse_of: Dictionary) -> float:
	return NODE_SIZE.x * 2.0 + SPOUSE_GAP if spouse_of.has(id) else NODE_SIZE.x


func _subtree_width(id: String, children_of: Dictionary, spouse_of: Dictionary, widths: Dictionary) -> float:
	var kids: Array = children_of.get(id, [])
	kids.sort_custom(_older_first)
	var total := 0.0
	for kid in kids:
		total += _subtree_width(kid, children_of, spouse_of, widths)
	if not kids.is_empty():
		total += SIBLING_GAP * float(kids.size() - 1)
	var width := maxf(_unit_width(id, spouse_of), total)
	widths[id] = width
	return width


func _place(id: String, left: float, children_of: Dictionary, spouse_of: Dictionary, widths: Dictionary, min_generation: int) -> void:
	var width: float = widths[id]
	var unit := _unit_width(id, spouse_of)
	var y := MARGIN.y + float(int((_nodes[id] as Dictionary).get("generation", 0)) - min_generation) * ROW_STEP
	var unit_left := left + (width - unit) * 0.5
	_positions[id] = Vector2(unit_left, y)
	if spouse_of.has(id):
		_positions[spouse_of[id]] = Vector2(unit_left + NODE_SIZE.x + SPOUSE_GAP, y)
	var kids: Array = children_of.get(id, [])
	kids.sort_custom(_older_first)
	var total := 0.0
	for kid in kids:
		total += float(widths[kid])
	total += SIBLING_GAP * float(maxi(0, kids.size() - 1))
	var x := left + (width - total) * 0.5
	for kid in kids:
		_place(kid, x, children_of, spouse_of, widths, min_generation)
		x += float(widths[kid]) + SIBLING_GAP


func _rebuild() -> void:
	for child in _canvas.get_children():
		_canvas.remove_child(child)
		child.queue_free()
	medallions.clear()
	var extent := Vector2.ZERO
	for id in _positions:
		var medallion := FamilyTreeNode.create(_nodes[id], id == ruler_id, id == heir_id, id == root_id, zoom, _portrait_context(id))
		medallion.position = _screen(id)
		medallion.pressed.connect(func() -> void: character_selected.emit(id))
		medallion.recenter.connect(func() -> void: recenter_requested.emit(id))
		_canvas.add_child(medallion)
		medallions[id] = medallion
		extent = extent.max((_positions[id] as Vector2) + NODE_SIZE)
	_offset_x = 0.0
	for id in medallions:
		(medallions[id] as Control).position = _screen(id)
	_canvas.custom_minimum_size = (extent + MARGIN) * zoom
	_canvas.queue_redraw()


## DA2 : contexte de rang du portrait vivant (souverain de l'arbre, maison régnante).
func _portrait_context(id: String) -> Dictionary:
	var entry: Dictionary = _nodes[id]
	var context := LivingPortrait.context_for(entry)
	var root_faction := str((_nodes.get(root_id, {}) as Dictionary).get("faction", ""))
	if ruler_id != "" and str(entry.get("faction", "")) == root_faction:
		context["ruler"] = ruler_id
		context["heir"] = heir_id
		if _nodes.has(ruler_id):
			context["ruler_house"] = str((_nodes[ruler_id] as Dictionary).get("house", ""))
	return context


## Décalage horizontal qui centre un arbre plus étroit que la vue.
func _update_offset() -> void:
	var content := _canvas.custom_minimum_size.x
	var offset := maxf(0.0, (size.x - content) * 0.5)
	if absf(offset - _offset_x) < 0.5:
		return
	_offset_x = offset
	for id in medallions:
		(medallions[id] as Control).position = _screen(id)
	_canvas.queue_redraw()


func _screen(id: String) -> Vector2:
	return (_positions[id] as Vector2) * zoom + Vector2(_offset_x, 0)


# --- Traits à l'encre (appelé par le canevas) ------------------------------------------------


func draw_links(canvas: Control) -> void:
	var ink := Color(HudStyle.INK, 0.85)
	var width := maxf(1.2, 2.0 * zoom)
	var node_size := NODE_SIZE * zoom
	var portrait_y := (PORTRAIT_RADIUS + 10.0) * zoom
	# Conjoints : double trait horizontal à hauteur des portraits.
	for couple in _couples:
		var a: Vector2 = _screen(couple[0])
		var b: Vector2 = _screen(couple[1])
		var left := a if a.x < b.x else b
		var right := b if a.x < b.x else a
		var from := left + Vector2(node_size.x * 0.5 + (PORTRAIT_RADIUS + 4.0) * zoom, portrait_y)
		var to := right + Vector2(node_size.x * 0.5 - (PORTRAIT_RADIUS + 4.0) * zoom, portrait_y)
		canvas.draw_line(from + Vector2(0, -2.5 * zoom), to + Vector2(0, -2.5 * zoom), ink, width * 0.8, true)
		canvas.draw_line(from + Vector2(0, 2.5 * zoom), to + Vector2(0, 2.5 * zoom), ink, width * 0.8, true)
	# Fratrie sans parents affichés (ex. Philippe VI et Charles d'Alençon) : arc pointillé.
	if _positions.has(root_id) and _primary_parent(root_id) == "":
		var root_top := _screen(root_id) + Vector2(node_size.x * 0.5, 2.0 * zoom)
		for id in _nodes:
			var entry: Dictionary = _nodes[id]
			if id == root_id or not bool(entry.get("blood", true)) or _primary_parent(id) != "":
				continue
			if int(entry.get("generation", 0)) != int((_nodes[root_id] as Dictionary).get("generation", 0)):
				continue
			var top := _screen(id) + Vector2(node_size.x * 0.5, 2.0 * zoom)
			var lift := Vector2(0, -16.0 * zoom)
			canvas.draw_dashed_line(root_top, root_top + lift, ink, width * 0.7, 5.0 * zoom)
			canvas.draw_dashed_line(root_top + lift, top + lift, ink, width * 0.7, 5.0 * zoom)
			canvas.draw_dashed_line(top + lift, top, ink, width * 0.7, 5.0 * zoom)
			var font := canvas.get_theme_default_font()
			canvas.draw_string(font, (root_top + top) * 0.5 + lift + Vector2(-40.0 * zoom, -4.0 * zoom), "fratrie", HORIZONTAL_ALIGNMENT_CENTER, 80.0 * zoom, int(11.0 * zoom), HudStyle.INK_SOFT)
	# Parents → enfants : descente depuis le couple (ou le parent seul), barre, puis chaque enfant.
	var groups: Dictionary = {}  # clé parents → [enfants]
	for id in _nodes:
		if not _positions.has(id):
			continue
		var entry: Dictionary = _nodes[id]
		var father := str(entry.get("father", ""))
		var mother := str(entry.get("mother", ""))
		var parents: Array = []
		for parent in [father, mother]:
			if parent != "" and _positions.has(parent):
				parents.append(parent)
		if parents.is_empty():
			continue
		var key := "|".join(parents)
		if not groups.has(key):
			groups[key] = []
		(groups[key] as Array).append(id)
	for key in groups:
		var parents: PackedStringArray = str(key).split("|")
		var anchor := Vector2.ZERO
		for parent in parents:
			anchor += _screen(parent) + Vector2(node_size.x * 0.5, 0)
		anchor /= float(parents.size())
		var bottom := 0.0
		for parent in parents:
			bottom = maxf(bottom, (_positions[parent] as Vector2).y * zoom + node_size.y)
		if parents.size() == 2:
			anchor.y = (_positions[parents[0]] as Vector2).y * zoom + portrait_y
		else:
			anchor.y = bottom - 4.0 * zoom
		var kids: Array = groups[key]
		var child_top := INF
		var xs: Array = [anchor.x]
		for kid in kids:
			var top: Vector2 = _screen(kid) + Vector2(node_size.x * 0.5, 0)
			child_top = minf(child_top, top.y)
			xs.append(top.x)
		var bar_y := child_top - 18.0 * zoom
		canvas.draw_line(anchor, Vector2(anchor.x, bar_y), ink, width, true)
		canvas.draw_line(Vector2(xs.min(), bar_y), Vector2(xs.max(), bar_y), ink, width, true)
		for kid in kids:
			var top: Vector2 = _screen(kid) + Vector2(node_size.x * 0.5, 0)
			canvas.draw_line(Vector2(top.x, bar_y), top + Vector2(0, 4.0 * zoom), ink, width, true)
			# Petite pointe de plume au bout du trait.
			canvas.draw_circle(top + Vector2(0, 4.0 * zoom), width * 1.2, ink)


# --- Entrées : zoom (Ctrl + molette) et défilement à la main ------------------------------


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMagnifyGesture:
		set_zoom(zoom * (event as InputEventMagnifyGesture).factor)
		accept_event()
	elif event is InputEventMouseButton:
		var button := event as InputEventMouseButton
		if button.ctrl_pressed and button.pressed and button.button_index in [MOUSE_BUTTON_WHEEL_UP, MOUSE_BUTTON_WHEEL_DOWN]:
			set_zoom(zoom * (1.1 if button.button_index == MOUSE_BUTTON_WHEEL_UP else 1.0 / 1.1))
			accept_event()
		elif button.button_index == MOUSE_BUTTON_MIDDLE or (button.button_index == MOUSE_BUTTON_LEFT and not button.pressed):
			_drag_from = button.position if button.pressed else Vector2.INF
		elif button.button_index == MOUSE_BUTTON_LEFT and button.pressed:
			_drag_from = button.position
	elif event is InputEventMouseMotion and _drag_from != Vector2.INF:
		var motion := event as InputEventMouseMotion
		if motion.button_mask & (MOUSE_BUTTON_MASK_LEFT | MOUSE_BUTTON_MASK_MIDDLE):
			scroll_horizontal -= int(motion.relative.x)
			scroll_vertical -= int(motion.relative.y)
			accept_event()
		else:
			_drag_from = Vector2.INF


## Canevas défilant : porte les médaillons et trace les liens sous eux.
class FamilyTreeCanvas:
	extends Control

	var view: FamilyTreeView

	func _init() -> void:
		mouse_filter = Control.MOUSE_FILTER_PASS

	func _draw() -> void:
		if view != null:
			view.draw_links(self)


## Médaillon d'un personnage : portrait en disque (ou écu de sa faction), cadre d'encre ou
## d'or, nom, dates ; grisé s'il est mort ; couronne pour l'héritier.
class FamilyTreeNode:
	extends Control

	signal pressed
	signal recenter

	const GREY_SHADER := "shader_type canvas_item;\nuniform float amount = 0.0;\nvoid fragment() {\n\tvec4 c = texture(TEXTURE, UV) * COLOR;\n\tfloat g = dot(c.rgb, vec3(0.299, 0.587, 0.114));\n\tCOLOR = vec4(mix(c.rgb, vec3(g) * 0.95 + 0.05, amount), c.a * mix(1.0, 0.92, amount));\n}"
	static var _grey_material: ShaderMaterial

	var entry: Dictionary = {}
	var is_ruler := false
	var is_heir := false
	var is_root := false
	var zoom := 1.0
	var _texture: Texture2D
	var _mirror := false  # DA2 : visage type retourné (anti-clones)
	var _is_portrait := false
	var _hover := false

	static func create(data: Dictionary, ruler: bool, heir: bool, root: bool, zoom_value: float, context: Dictionary = {}) -> FamilyTreeNode:
		var node := FamilyTreeNode.new()
		node.entry = data
		node.is_ruler = ruler
		node.is_heir = heir
		node.is_root = root
		node.zoom = zoom_value
		node.name = "Node_" + str(data.get("id", "?"))
		node.custom_minimum_size = FamilyTreeView.NODE_SIZE * zoom_value
		node.size = node.custom_minimum_size
		node.mouse_filter = Control.MOUSE_FILTER_STOP
		node.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
		var id := str(data.get("id", ""))
		# DA2 : portrait vivant (tranche d'âge et rang courants, archétype pour les nés en jeu).
		node._texture = LivingPortrait.texture_for(data, context) if id != "" else null
		node._is_portrait = node._texture != null
		if node._is_portrait:
			var ctx := context if not context.is_empty() else LivingPortrait.context_for(data)
			node._mirror = LivingPortrait.mirrored(data, LivingPortrait.resolve(data, ctx))
		if node._texture == null:
			node._texture = PortraitLoader.house_heraldry_texture(str(data.get("house", "")), str(data.get("faction", "")))  # DA1
		# Médaillon dessiné par un enfant : seul le portrait est grisé pour un défunt.
		var disc := Control.new()
		disc.name = "Disc"
		disc.mouse_filter = Control.MOUSE_FILTER_IGNORE
		# Taille fixée par les ancres (plein cadre, marges nulles) : pas d'affectation de
		# `size`, qui déclenchait l'avertissement « non-equal opposite anchors ».
		disc.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		disc.draw.connect(func() -> void: node._draw_disc(disc))
		if not bool(data.get("alive", true)):
			disc.material = _grey()
		node.add_child(disc)
		node.tooltip_text = node._tooltip()
		node.mouse_entered.connect(func() -> void:
			node._hover = true
			node.queue_redraw())
		node.mouse_exited.connect(func() -> void:
			node._hover = false
			node.queue_redraw())
		return node

	static func _grey() -> ShaderMaterial:
		if _grey_material == null:
			var shader := Shader.new()
			shader.code = GREY_SHADER
			_grey_material = ShaderMaterial.new()
			_grey_material.shader = shader
			_grey_material.set_shader_parameter("amount", 1.0)
		return _grey_material

	## « ° 1312 · 25 ans » pour un vivant, « 1310–1346 » pour un défunt (C7 : année de décès
	## tenue par la simulation ; « 1310–† » si elle manque, sauvegarde antérieure).
	func dates_text() -> String:
		return FamilyTreeView.life_dates(entry)

	func _tooltip() -> String:
		var lines: Array = ["[b]%s[/b]" % str(entry.get("name", "?"))]
		var title := str(entry.get("title", ""))
		if title != "":
			lines.append("[i]%s[/i]" % title)
		lines.append("Maison %s — %s" % [str(entry.get("house", "")), dates_text()])
		if is_ruler:
			lines.append("[color=#8b6a1a]Chef de la maison régnante[/color]")
		var female := str(entry.get("sex", "")) == "female"
		if is_heir:
			lines.append("[color=#8b6a1a]♛ %s[/color]" % ("Héritière désignée" if female else "Héritier désigné"))
		if not bool(entry.get("alive", true)):
			lines.append("[color=#6b5a40]%s[/color]" % ("Défunte" if female else "Défunt"))
		lines.append("[color=#6b5a40]Clic : fiche — clic droit : recentrer l'arbre[/color]")
		return "\n".join(lines)

	func _make_custom_tooltip(for_text: String) -> Object:
		return RichTooltip.make_panel(for_text)

	func _gui_input(event: InputEvent) -> void:
		if event is InputEventMouseButton and (event as InputEventMouseButton).pressed:
			var button := event as InputEventMouseButton
			if button.button_index == MOUSE_BUTTON_LEFT and not button.ctrl_pressed:
				pressed.emit()
				accept_event()
			elif button.button_index == MOUSE_BUTTON_RIGHT:
				recenter.emit()
				accept_event()

	func _draw() -> void:
		var z := zoom
		var width := size.x
		var radius := FamilyTreeView.PORTRAIT_RADIUS * z
		var center := Vector2(width * 0.5, radius + 10.0 * z)
		var alive := bool(entry.get("alive", true))
		# Cartouche de parchemin sous le nom.
		var card := Rect2(Vector2(2, center.y + radius * 0.55), Vector2(width - 4, size.y - center.y - radius * 0.55 - 2))
		var card_fill := HudStyle.PARCHMENT_LIGHT if not is_root else HudStyle.PARCHMENT.lerp(HudStyle.GOLD_PALE, 0.35)
		if is_heir:  # U10 (audit A3, P5) : l'héritier se voit de loin
			card_fill = HudStyle.PARCHMENT_LIGHT.lerp(HudStyle.GOLD_PALE, 0.6)
		draw_rect(card, card_fill)
		var card_border := HudStyle.RUBRIC if _hover else (HudStyle.GOLD if is_heir else HudStyle.INK_SOFT)
		draw_rect(card, card_border, false, maxf(1.0, (2.5 if (_hover or is_heir) else 1.0) * z))
		# Nom (deux lignes au plus) et dates.
		var font := get_theme_default_font()
		var name_size := int(round(13.0 * z))
		var text := str(entry.get("name", "?"))
		var name_y := center.y + radius + 18.0 * z
		var ink := HudStyle.INK if alive else HudStyle.INK_SOFT
		draw_multiline_string(font, Vector2(4, name_y), text, HORIZONTAL_ALIGNMENT_CENTER, width - 8, name_size, 2, ink)
		var date_size := int(round(11.0 * z))
		draw_string(font, Vector2(4, size.y - 8.0 * z), dates_text(), HORIZONTAL_ALIGNMENT_CENTER, width - 8, date_size, HudStyle.RUBRIC if alive else HudStyle.INK_SOFT)
		if is_heir:
			# Cartouche « HÉRITIER » sous le cartouche du nom (le médaillon couvre le haut).
			var tag := "HÉRITIÈRE" if str(entry.get("sex", "")) == "female" else "HÉRITIER"
			var tag_size := int(round(10.0 * z))
			var tag_width := font.get_string_size(tag, HORIZONTAL_ALIGNMENT_LEFT, -1, tag_size).x + 10.0 * z
			var tag_rect := Rect2(Vector2((width - tag_width) * 0.5, card.end.y + 1.0 * z), Vector2(tag_width, 13.0 * z))
			draw_rect(tag_rect, HudStyle.RUBRIC)
			draw_rect(tag_rect, HudStyle.GOLD, false, maxf(1.0, z))
			draw_string(font, Vector2(tag_rect.position.x + 5.0 * z, tag_rect.end.y - 3.0 * z), tag, HORIZONTAL_ALIGNMENT_LEFT, -1, tag_size, HudStyle.PARCHMENT_LIGHT)

	## Médaillon : fond, portrait (ou écu), filet d'encre, anneau d'or, couronnes.
	func _draw_disc(canvas: Control) -> void:
		var z := zoom
		var radius := FamilyTreeView.PORTRAIT_RADIUS * z
		var center := Vector2(size.x * 0.5, radius + 10.0 * z)
		canvas.draw_circle(center + Vector2(1.5, 2.5) * z, radius + 3.0 * z, HudStyle.SHADOW)
		canvas.draw_circle(center, radius + 3.0 * z, HudStyle.PARCHMENT_DARK)
		if _is_portrait:
			HudStyle.draw_texture_disc(canvas, _texture, center, radius, _mirror)
		else:
			canvas.draw_circle(center, radius, HudStyle.PARCHMENT)
			HudStyle.draw_texture_fit(canvas, _texture, center, radius * 1.3)
		var ring := HudStyle.GOLD if (is_ruler or is_heir) else HudStyle.INK
		canvas.draw_arc(center, radius + 1.5 * z, 0.0, TAU, 64, ring, maxf(1.5, (4.0 if is_ruler else 2.5) * z), true)
		if is_root:
			canvas.draw_arc(center, radius + 6.0 * z, 0.0, TAU, 64, HudStyle.RUBRIC, maxf(1.0, 1.5 * z), true)
		if is_heir:
			canvas.draw_arc(center, radius + 6.0 * z, 0.0, TAU, 64, Color(HudStyle.GOLD, 0.7), maxf(1.5, 3.0 * z), true)
			_draw_crown(canvas, center + Vector2(0, -radius - 2.0 * z), 15.0 * z)
		if is_ruler:
			_draw_crown(canvas, center + Vector2(0, -radius - 2.0 * z), 18.0 * z)

	## Couronne à l'encre et à l'or (cinq fleurons), posée au sommet du médaillon.
	func _draw_crown(canvas: Control, base_center: Vector2, crown_width: float) -> void:
		var half := crown_width * 0.5
		var h := crown_width * 0.62
		var base := base_center + Vector2(0, h * 0.35)
		var points := PackedVector2Array([
			base + Vector2(-half, 0), base + Vector2(-half, -h * 0.7), base + Vector2(-half * 0.5, -h * 0.35),
			base + Vector2(0, -h), base + Vector2(half * 0.5, -h * 0.35), base + Vector2(half, -h * 0.7),
			base + Vector2(half, 0)])
		canvas.draw_colored_polygon(points, HudStyle.GOLD)
		var outline := points.duplicate()
		outline.append(points[0])
		canvas.draw_polyline(outline, HudStyle.INK, maxf(1.0, crown_width * 0.08), true)
		for tip in [points[1], points[3], points[5]]:
			canvas.draw_circle(tip, crown_width * 0.08, HudStyle.WAX)
