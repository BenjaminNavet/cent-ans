class_name SkillTreeView
extends Control

## Arbre de compétences visuel de la fiche de personnage (lot C3) : une bande par domaine
## (Commandement, Gouvernance, Cour), les rangs en colonnes de gauche à droite, chaque
## compétence en médaillon relié à ses prérequis par un trait d'encre (doré une fois appris).
## États : appris (or), disponible (filet rubrique, cliquable), verrouillé (grisé).
## Clic sur un nœud disponible = `learn_requested` (ordre `learn_skill` soumis ailleurs).
## Aucune règle ici : l'état vient de `skills_learned` et de `CampaignSim.get_learnable`.

signal learn_requested(skill_id: String)

const BRANCH_ORDER := ["command", "governance", "court"]
const BRANCH_LABELS := {"command": "Commandement", "governance": "Gouvernance", "court": "Cour"}
const NODE_SIZE := Vector2(152, 56)
const COLUMN_STEP := 178.0
const ROW_STEP := 64.0
const BAND_HEADER := 30.0
const BAND_GAP := 14.0
const LEFT := 8.0

const STATE_LEARNED := "learned"
const STATE_AVAILABLE := "available"
const STATE_LOCKED := "locked"
const STATE_LABELS := {"learned": "Appris", "available": "Disponible", "locked": "Verrouillé"}

## id → SkillNode (médaillon), pour le dessin des liens et les tests.
var nodes: Dictionary = {}
var _positions: Dictionary = {}  # id → coin haut-gauche
var _entries: Dictionary = {}  # id → entrée de `get_skill_tree`
var _states: Dictionary = {}  # id → STATE_*
var _bands: Array = []  # [{branch, top, height, value}]


## `tree` : `get_skill_tree()` ; `learned` : `skills_learned` ; `learnable` : `get_learnable(id)` ;
## `branch_values` : `{command, governance, court}` du personnage (affichés dans les bandes).
func show_tree(tree: Array, learned: Array, learnable: Array, branch_values: Dictionary = {}) -> void:
	for child in get_children():
		remove_child(child)
		child.queue_free()
	nodes.clear()
	_positions.clear()
	_entries.clear()
	_states.clear()
	_bands.clear()
	for entry in tree:
		var id := str(entry.get("id", ""))
		_entries[id] = entry
		_states[id] = STATE_LEARNED if learned.has(id) else (STATE_AVAILABLE if learnable.has(id) else STATE_LOCKED)
	var top := 0.0
	var width := 0.0
	for branch in BRANCH_ORDER:
		var columns := _columns_of(branch)
		if columns.is_empty():
			continue
		var rows := 0
		for column in columns:
			rows = maxi(rows, (column as Array).size())
		var band_top := top
		for c in columns.size():
			var column: Array = columns[c]
			var offset := (float(rows - column.size()) * ROW_STEP) * 0.5
			for r in column.size():
				var id: String = column[r]
				_positions[id] = Vector2(LEFT + float(c) * COLUMN_STEP, band_top + BAND_HEADER + offset + float(r) * ROW_STEP)
		var height := BAND_HEADER + float(rows) * ROW_STEP
		_bands.append({"branch": branch, "top": band_top, "height": height, "value": int(branch_values.get(branch, -1))})
		width = maxf(width, LEFT + float(columns.size() - 1) * COLUMN_STEP + NODE_SIZE.x + LEFT)
		top += height + BAND_GAP
	for band in _bands:
		var header := IconChip.create("branch_" + str(band["branch"]), _band_title(band), RichTooltip.branch(str(band["branch"]), int(band["value"])), 20.0, 14)
		header.position = Vector2(LEFT, float(band["top"]) + 3.0)
		header.label.add_theme_color_override("font_color", HudStyle.INK)
		add_child(header)
	for id in _positions:
		var node := SkillNode.create(_entries[id], _states[id])
		node.position = _positions[id]
		node.clicked.connect(func() -> void:
			if _states.get(id, "") == STATE_AVAILABLE:
				learn_requested.emit(id))
		add_child(node)
		nodes[id] = node
	custom_minimum_size = Vector2(width, maxf(0.0, top - BAND_GAP))
	queue_redraw()


func state_of(skill_id: String) -> String:
	return str(_states.get(skill_id, ""))


## Ids des compétences disponibles (cliquables), dans l'ordre d'affichage.
func available_ids() -> Array:
	var result: Array = []
	for id in nodes:
		if _states[id] == STATE_AVAILABLE:
			result.append(id)
	return result


## Simule le clic sur le médaillon `skill_id` (tests) ; faux s'il n'est pas disponible.
func press(skill_id: String) -> bool:
	if not nodes.has(skill_id) or _states.get(skill_id, "") != STATE_AVAILABLE:
		return false
	(nodes[skill_id] as SkillNode).clicked.emit()
	return true


func _band_title(band: Dictionary) -> String:
	var label := str(BRANCH_LABELS.get(band["branch"], band["branch"]))
	return "%s — niveau %d" % [label, int(band["value"])] if int(band["value"]) >= 0 else label


## Colonnes d'un domaine : profondeur = max(rang − 1, profondeur des prérequis + 1), puis tri
## barycentrique (moyenne des lignes des prérequis) pour limiter les croisements.
func _columns_of(branch: String) -> Array:
	var ids: Array = []
	for id in _entries:
		if str((_entries[id] as Dictionary).get("branch", "")) == branch:
			ids.append(id)
	ids.sort()
	var depth: Dictionary = {}
	var changed := true
	for id in ids:
		depth[id] = int((_entries[id] as Dictionary).get("tier", 1)) - 1
	while changed:
		changed = false
		for id in ids:
			for prereq in (_entries[id] as Dictionary).get("prerequisites", []):
				if depth.has(str(prereq)) and int(depth[id]) <= int(depth[str(prereq)]):
					depth[id] = int(depth[str(prereq)]) + 1
					changed = true
	var columns: Array = []
	for id in ids:
		while columns.size() <= int(depth[id]):
			columns.append([])
		(columns[int(depth[id])] as Array).append(id)
	var row_of: Dictionary = {}
	for c in columns.size():
		var column: Array = columns[c]
		if c > 0:
			var weight := func(id: String) -> float:
				var total := 0.0
				var count := 0
				for prereq in (_entries[id] as Dictionary).get("prerequisites", []):
					if row_of.has(str(prereq)):
						total += float(row_of[str(prereq)])
						count += 1
				return total / float(count) if count > 0 else 99.0
			column.sort_custom(func(a: String, b: String) -> bool:
				var wa: float = weight.call(a)
				var wb: float = weight.call(b)
				return wa < wb if wa != wb else a < b)
		for r in column.size():
			row_of[column[r]] = r
	return columns


func _draw() -> void:
	# Bandes de domaine : fond de parchemin légèrement teinté, filet en tête.
	for band in _bands:
		var rect := Rect2(0, float(band["top"]), size.x, float(band["height"]))
		draw_rect(rect, Color(HudStyle.PARCHMENT_DARK, 0.18))
		draw_line(rect.position + Vector2(0, BAND_HEADER - 4.0), rect.position + Vector2(rect.size.x, BAND_HEADER - 4.0), Color(HudStyle.INK_SOFT, 0.5), 1.0)
	# Liens prérequis → compétence : courbe d'encre, dorée si les deux bouts sont appris.
	for id in _positions:
		var target: Vector2 = _positions[id] + Vector2(0, NODE_SIZE.y * 0.5)
		for prereq in (_entries[id] as Dictionary).get("prerequisites", []):
			var key := str(prereq)
			if not _positions.has(key):
				continue
			var source: Vector2 = _positions[key] + Vector2(NODE_SIZE.x, NODE_SIZE.y * 0.5)
			var both_learned: bool = _states[id] == STATE_LEARNED and _states[key] == STATE_LEARNED
			var color := HudStyle.GOLD if both_learned else (HudStyle.INK if _states[key] == STATE_LEARNED else HudStyle.INK_FADED)
			var curve := Curve2D.new()
			var bend := (target.x - source.x) * 0.5
			curve.add_point(source, Vector2.ZERO, Vector2(bend, 0))
			curve.add_point(target, Vector2(-bend, 0), Vector2.ZERO)
			draw_polyline(curve.tessellate(4, 3.0), color, 2.5 if both_learned else 1.6, true)


## Médaillon d'une compétence : disque (icône du domaine, chiffre du rang), nom et coût.
class SkillNode:
	extends Control

	signal clicked

	var entry: Dictionary = {}
	var state: String = ""
	var _hover := false
	var _icon: Texture2D

	static func create(data: Dictionary, node_state: String) -> SkillNode:
		var node := SkillNode.new()
		node.entry = data
		node.state = node_state
		node.name = "Skill_" + str(data.get("id", "?"))
		node.custom_minimum_size = SkillTreeView.NODE_SIZE
		node.size = SkillTreeView.NODE_SIZE
		node.mouse_filter = Control.MOUSE_FILTER_STOP
		if node_state == SkillTreeView.STATE_AVAILABLE:
			node.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
		# U10 (audit A3, P2) : une icône par compétence ; à défaut, celle de sa branche.
		node._icon = HudStyle.icon(str(data.get("id", "")), "skill")
		if node._icon == null:
			node._icon = HudStyle.icon("branch_" + str(data.get("branch", "")), "branch")
		var hint := ""
		if node_state == SkillTreeView.STATE_AVAILABLE:
			var cost := int(data.get("cost", 0))
			hint = "\n[color=#8b1a1a]Clic : apprendre (%d point%s)[/color]" % [cost, "s" if cost > 1 else ""]
		node.tooltip_text = RichTooltip.skill(data, str(SkillTreeView.STATE_LABELS.get(node_state, ""))) + hint
		node.mouse_entered.connect(func() -> void:
			node._hover = true
			node.queue_redraw())
		node.mouse_exited.connect(func() -> void:
			node._hover = false
			node.queue_redraw())
		return node

	func _make_custom_tooltip(for_text: String) -> Object:
		return TooltipHost.bubble(for_text, self)

	func _gui_input(event: InputEvent) -> void:
		if event is InputEventMouseButton:
			var button := event as InputEventMouseButton
			if button.pressed and button.button_index == MOUSE_BUTTON_LEFT:
				clicked.emit()
				accept_event()

	func _draw() -> void:
		var radius := size.y * 0.5 - 3.0
		var center := Vector2(radius + 3.0, size.y * 0.5)
		var learned := state == SkillTreeView.STATE_LEARNED
		var available := state == SkillTreeView.STATE_AVAILABLE
		# Cartouche du nom.
		var card := Rect2(Vector2(center.x, 6), Vector2(size.x - center.x - 2, size.y - 12))
		var fill := HudStyle.PARCHMENT_LIGHT
		if learned:
			fill = HudStyle.PARCHMENT.lerp(HudStyle.GOLD_PALE, 0.55)
		elif not available:
			fill = HudStyle.PARCHMENT.darkened(0.06)
		draw_rect(card, fill)
		var border := HudStyle.RUBRIC if (available and _hover) else (HudStyle.GOLD if learned else HudStyle.INK_SOFT)
		draw_rect(card, border, false, 2.0 if (available or learned) else 1.0)
		# Disque : or si appris, parchemin cerclé de rubrique si disponible, gris sinon.
		var disc := HudStyle.GOLD if learned else (HudStyle.PARCHMENT_LIGHT if available else HudStyle.PARCHMENT_DARK.lerp(Color(0.6, 0.58, 0.55), 0.6))
		draw_circle(center + Vector2(1, 2), radius, HudStyle.SHADOW)
		draw_circle(center, radius, disc)
		var ring := HudStyle.INK if learned else (HudStyle.RUBRIC if available else HudStyle.INK_FADED)
		draw_arc(center, radius, 0.0, TAU, 40, ring, 2.5 if available else 1.5, true)
		if available and _hover:
			draw_arc(center, radius + 3.0, 0.0, TAU, 40, Color(HudStyle.RUBRIC, 0.6), 1.5, true)
		var icon_tint := Color.WHITE if (learned or available) else Color(1, 1, 1, 0.45)
		HudStyle.draw_texture_fit(self, _icon, center, radius * 1.15, icon_tint)
		# Nom (deux lignes) et coût / état.
		var font := get_theme_default_font()
		var ink := HudStyle.INK if (learned or available) else HudStyle.INK_FADED
		var text_left := center.x + radius + 5.0
		var text_width := size.x - text_left - 4.0
		draw_multiline_string(font, Vector2(text_left, 20), str(entry.get("name", entry.get("id", "?"))), HORIZONTAL_ALIGNMENT_LEFT, text_width, 13, 2, ink)
		var status := "%s · %d pt" % [SkillTreeView.STATE_LABELS.get(state, ""), int(entry.get("cost", 0))]
		var status_color := HudStyle.GOOD if learned else (HudStyle.RUBRIC if available else HudStyle.INK_FADED)
		draw_string(font, Vector2(text_left, size.y - 8), status, HORIZONTAL_ALIGNMENT_LEFT, text_width, 11, status_color)
