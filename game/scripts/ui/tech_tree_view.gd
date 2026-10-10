class_name TechTreeView
extends Control

## Un arbre de technologies (une branche) : colonnes par tier, nœuds reliés par des lignes
## de prérequis dessinées sous les boutons. Couleur par état (`known`, `researching`,
## `available`, `locked`, fourni par `CampaignSim.get_tech_tree`). Clic sur un nœud
## disponible = `research_requested`. Aucune règle ici.

signal research_requested(technology_id: String)
## A6-L4 : Maj+clic sur une technologie disponible = la mettre en file derrière la recherche.
signal queue_requested(technology_id: String)

const NODE_SIZE := Vector2(212, 92)
const COLUMN_STEP := 252.0
const ROW_STEP := 106.0
## Hauteur réservée en pied de nœud aux pastilles de déblocage et à l'effet clé (UX5-T1).
const FOOT_HEIGHT := 38.0
const DIM_ALPHA := 0.38
const MAX_PILLS := 2
const PILL_NAME_CHARS := 13
const MARGIN := Vector2(12, 34)

const STATE_LABELS := {
	"known": "Acquise",
	"researching": "En cours",
	"available": "Disponible",
	"queued": "En file",
	"locked": "Verrouillée",
}
const EFFECT_LABELS := {
	"tax_income": "Impôts", "trade_income": "Commerce", "health": "Santé", "growth": "Croissance",
	"unrest": "Mécontentement", "wealth": "Richesse", "production": "Production",
	"army_armor": "Armure", "army_ranged": "Tir", "army_melee": "Mêlée", "army_morale": "Moral",
	"army_upkeep": "Entretien des armées", "army_experience": "Expérience des troupes",
	"recruit_cost": "Coût de recrutement", "movement": "Mouvement",
	"siege_resistance": "Résistance aux sièges", "fortification_level": "Fortifications",
	"research_points": "Points de recherche", "prestige": "Prestige", "piety": "Piété",
	"plague_resistance": "Résistance à la peste", "wound_recovery": "Soin des blessés",
	"diet_health": "Santé tirée des régimes",
}
const CATEGORY_LABELS := {"infantry": "infanterie", "ranged": "tireurs", "cavalry": "cavalerie", "siege": "siège"}

## id → bouton, pour tracer les lignes et les tests.
var buttons: Dictionary = {}
var _nodes: Array = []
var _by_id: Dictionary = {}
var _children_of: Dictionary = {}  # id → ids des savoirs qui le requièrent (UX5-T2)
var _hover_id: String = ""
var _path: Dictionary = {}  # id → true : chemin surligné du savoir visé (ancêtres non acquis + descendants)
## UX5-T3 : {query, reach, unlock, fade_known}. Filtrer estompe, ne retire rien de la mise en page.
var _filter: Dictionary = {}
## Glyphes d'état (UX5-T7) : doublent la couleur (l'état n'est jamais porté par la couleur seule).
var state_glyphs: Dictionary = {}


## `nodes` : entrées de `get_tech_tree` d'une seule branche.
func show_tree(nodes: Array) -> void:
	_nodes = nodes
	for child in get_children():
		child.queue_free()
	buttons.clear()
	_hover_id = ""
	_path.clear()
	_by_id.clear()
	_children_of.clear()
	if state_glyphs.is_empty():
		state_glyphs = {"known": _glyph("✓", "+"), "researching": _glyph("✒", "*"), "queued": "",
			"available": "", "locked": _glyph("⛓", "×")}
	for node in nodes:
		_by_id[str(node["id"])] = node
	for node in nodes:
		for prereq in node.get("prerequisites", []):
			if not _children_of.has(str(prereq)):
				_children_of[str(prereq)] = []
			(_children_of[str(prereq)] as Array).append(str(node["id"]))
	var by_tier: Dictionary = {}
	for node in nodes:
		var tier := int(node.get("tier", 1))
		if not by_tier.has(tier):
			by_tier[tier] = []
		(by_tier[tier] as Array).append(node)
	var tiers: Array = by_tier.keys()
	tiers.sort()
	var rows_of: Dictionary = {}  # id → rang dans sa colonne (pour le tri barycentrique)
	var max_rows := 0
	for column in tiers.size():
		var tier: int = tiers[column]
		var entries: Array = by_tier[tier]
		# Trie par position moyenne des prérequis pour limiter les croisements.
		entries.sort_custom(func(a: Dictionary, b: Dictionary) -> bool:
			var ka := _barycenter(a, rows_of)
			var kb := _barycenter(b, rows_of)
			return ka < kb if not is_equal_approx(ka, kb) else str(a["id"]) < str(b["id"]))
		var header := UiBuild.label("Rang %d" % tier)
		UiType.apply(header, UiType.CAPTION)  # PO phase 2 (P2b) : plus de taille ad hoc
		header.position = Vector2(MARGIN.x + column * COLUMN_STEP, 6)
		header.size = Vector2(NODE_SIZE.x, 22)
		header.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		add_child(header)
		for row in entries.size():
			var node: Dictionary = entries[row]
			rows_of[str(node["id"])] = row
			var button := _make_button(node)
			button.position = MARGIN + Vector2(column * COLUMN_STEP, row * ROW_STEP)
			add_child(button)
			buttons[str(node["id"])] = button
		max_rows = maxi(max_rows, entries.size())
	custom_minimum_size = Vector2(MARGIN.x * 2 + (tiers.size() - 1) * COLUMN_STEP + NODE_SIZE.x, MARGIN.y + max_rows * ROW_STEP)
	_refresh_dim()
	queue_redraw()


## Premier glyphe présent dans la police par défaut, sinon `fallback`.
static func _glyph(candidate: String, fallback: String) -> String:
	var font := ThemeDB.fallback_font
	return candidate if font != null and font.has_char(candidate.unicode_at(0)) else fallback


## Savoirs non acquis requis (transitivement) par `id`, dans la branche affichée.
func _unmet_ancestors(id: String, into: Dictionary) -> void:
	for prereq in (_by_id.get(id, {}) as Dictionary).get("prerequisites", []):
		var key := str(prereq)
		if into.has(key) or not _by_id.has(key):
			continue
		if str((_by_id[key] as Dictionary).get("state", "")) != "known":
			into[key] = true
			_unmet_ancestors(key, into)


func _descendants(id: String, into: Dictionary) -> void:
	for child in _children_of.get(id, []):
		if not into.has(child):
			into[child] = true
			_descendants(child, into)


## UX5-T2 : surligne le chemin du savoir `id` (vide = aucun) et estompe le reste.
func set_focus(id: String) -> void:
	if id == _hover_id:
		return
	_hover_id = id
	_path.clear()
	if id != "" and _by_id.has(id):
		_path[id] = true
		_unmet_ancestors(id, _path)
		_descendants(id, _path)
	_refresh_dim()
	queue_redraw()


## Savoirs requis non acquis, par nom (« Verrouillée : requiert … », UX5-T2).
func missing_requirements(id: String) -> PackedStringArray:
	var missing := PackedStringArray()
	for prereq in (_by_id.get(id, {}) as Dictionary).get("prerequisites", []):
		var key := str(prereq)
		if _by_id.has(key):
			if str((_by_id[key] as Dictionary).get("state", "")) != "known":
				missing.append(str((_by_id[key] as Dictionary).get("name", key)))
		else:
			missing.append(GameCatalog.display_name(key))
	return missing


## UX5-T3 : `filter` = {query: String, reach: bool, unlock: bool, fade_known: bool}.
func set_filter(filter: Dictionary) -> void:
	_filter = filter
	_refresh_dim()


func filter_active() -> bool:
	return str(_filter.get("query", "")).strip_edges() != "" or bool(_filter.get("reach", false)) \
		or bool(_filter.get("unlock", false)) or bool(_filter.get("fade_known", false))


func matches_filter(node: Dictionary) -> bool:
	var state := str(node.get("state", "locked"))
	if bool(_filter.get("fade_known", false)) and state == "known":
		return false
	if bool(_filter.get("reach", false)) and state != "available":
		return false
	var unlocks: Dictionary = node.get("unlock_names", {})
	if bool(_filter.get("unlock", false)) and (unlocks.get("units", []) as Array).is_empty() and (unlocks.get("buildings", []) as Array).is_empty():
		return false
	var query := str(_filter.get("query", "")).strip_edges().to_lower()
	if query != "":
		var haystack := str(node.get("name", "")).to_lower()
		for key in ["units", "buildings"]:
			for unlocked_name in unlocks.get(key, []):
				haystack += " " + str(unlocked_name).to_lower()
		if haystack.find(query) < 0:
			return false
	return true


## Vrai si le nœud `id` est estompé (filtre ou survol d'un autre chemin).
func is_dimmed(id: String) -> bool:
	return buttons.has(id) and (buttons[id] as Control).modulate.a < 0.99


func _refresh_dim() -> void:
	var filtering := filter_active()
	for id in buttons:
		var button: Control = buttons[id]
		var dim := false
		if filtering and not matches_filter(_by_id[id]):
			dim = true
		if _hover_id != "" and not _path.has(id):
			dim = true
		button.modulate.a = DIM_ALPHA if dim else 1.0


func _barycenter(node: Dictionary, rows_of: Dictionary) -> float:
	var total := 0.0
	var count := 0
	for prereq in node.get("prerequisites", []):
		if rows_of.has(str(prereq)):
			total += float(rows_of[str(prereq)])
			count += 1
	return total / count if count > 0 else 100.0


func _draw() -> void:
	for node in _nodes:
		var id: String = str(node["id"])
		if not buttons.has(id):
			continue
		var target: Control = buttons[id]
		var to := target.position + Vector2(0, NODE_SIZE.y * 0.5)
		for prereq in node.get("prerequisites", []):
			if not buttons.has(str(prereq)):
				continue
			var source: Control = buttons[str(prereq)]
			var from := source.position + Vector2(NODE_SIZE.x, NODE_SIZE.y * 0.5)
			var known: bool = str(source.get_meta("state", "")) == "known"
			var color := Color(HudStyle.WAX_GREEN, 0.95) if known else Color(HudStyle.INK_SOFT, 0.55)
			var lit := _hover_id != "" and _path.has(id) and _path.has(str(prereq))
			if _hover_id != "" and not lit:
				color.a *= 0.35
			if lit:
				color = HudStyle.GOLD
			var gap := (COLUMN_STEP - NODE_SIZE.x) * 0.5
			var points := PackedVector2Array([from])
			if to.x - from.x <= COLUMN_STEP:
				# Colonnes voisines : coude au milieu de l'intervalle.
				points.append_array([Vector2(from.x + gap, from.y), Vector2(from.x + gap, to.y)])
			else:
				# Saut de rang : passe dans l'interligne au-dessus de la cible pour ne pas
				# traverser les nœuds des colonnes intermédiaires.
				var lane_y := target.position.y - (ROW_STEP - NODE_SIZE.y) * 0.5
				points.append_array([Vector2(from.x + gap, from.y), Vector2(from.x + gap, lane_y),
					Vector2(to.x - gap, lane_y), Vector2(to.x - gap, to.y)])
			points.append(to)
			draw_polyline(points, color, 3.5 if lit else 2.0, true)


func _make_button(node: Dictionary) -> Button:
	var id: String = str(node["id"])
	var state: String = str(node.get("state", "locked"))
	var button := RichButton.new()
	button.set_meta("state", state)
	IconLibrary.decorate_button(button, id, 30, "technology")  # F2
	button.size = NODE_SIZE
	button.custom_minimum_size = NODE_SIZE
	button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	UiType.apply(button, UiType.CAPTION)  # PO phase 2 (P2b) : 12 px sous le minimum, Caption (14 px)
	var cost := int(node.get("effective_cost", node.get("cost", 0)))
	var queue_position := int(node.get("queue_position", 0))
	var shown_state := "queued" if queue_position > 0 and state == "available" else state
	var second_line: String
	if state == "researching" or (state == "available" and int(node.get("progress", 0)) > 0):
		second_line = "%d / %d pts — %s" % [int(node.get("progress", 0)), cost, STATE_LABELS[state]]
	else:
		second_line = "%d pts — %s" % [cost, STATE_LABELS.get(state, state)]
	if shown_state == "queued":
		second_line = "%d pts — En file (n° %d)" % [cost, queue_position]
	var glyph := str(state_glyphs.get(shown_state, ""))
	if shown_state == "queued":
		glyph = "n°%d" % queue_position
	button.set_meta("shown_state", shown_state)
	button.set_meta("glyph", glyph)
	button.text = "%s%s\n%s" % [glyph + " " if glyph != "" else "", str(node.get("name", id)), second_line]
	var tip := node
	var missing := missing_requirements(id) if state == "locked" else PackedStringArray()
	if not missing.is_empty():
		tip = node.duplicate()
		tip["lock_note"] = "Verrouillée : requiert " + ", ".join(missing)
	TooltipHost.set_tooltip(button, "technology", id, tip)  # Infobulle en sections (tooltip_for : texte brut)
	button.mouse_entered.connect(set_focus.bind(id))
	button.mouse_exited.connect(func() -> void:
		if _hover_id == id:
			set_focus(""))
	button.focus_entered.connect(set_focus.bind(id))
	button.focus_exited.connect(func() -> void:
		if _hover_id == id:
			set_focus(""))
	_add_foot(button, node)
	var fill := state_fill(shown_state)
	var style := StyleBoxFlat.new()
	style.bg_color = fill
	style.set_border_width_all(2 if state != "researching" else 3)
	style.border_color = HudStyle.GOLD if state == "researching" else (HudStyle.INK_SOFT if state != "locked" else HudStyle.INK_FADED)
	style.set_corner_radius_all(4)
	style.set_content_margin_all(4)
	style.content_margin_bottom = FOOT_HEIGHT + 2.0
	var hover := style.duplicate() as StyleBoxFlat
	hover.bg_color = style.bg_color.lightened(0.12)
	for style_name in ["normal", "disabled", "focus"]:
		button.add_theme_stylebox_override(style_name, style)
	button.add_theme_stylebox_override("hover", hover)
	button.add_theme_stylebox_override("pressed", hover)
	var font_color := HudStyle.INK if state != "locked" else HudStyle.INK_FADED
	for color_name in ["font_color", "font_disabled_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
		button.add_theme_color_override(color_name, font_color)
	button.disabled = state != "available"
	button.pressed.connect(func() -> void:
		if Input.is_key_pressed(KEY_SHIFT):
			queue_requested.emit(id)
		else:
			research_requested.emit(id))
	return button


## Fond d'un nœud selon son état (jetons `HudStyle`, UX5-T7).
static func state_fill(state: String) -> Color:
	match state:
		"known":
			return HudStyle.PARCHMENT_LIGHT.lerp(HudStyle.WAX_GREEN, 0.38)
		"researching":
			return HudStyle.GOLD_PALE
		"queued":
			return HudStyle.PARCHMENT_LIGHT.lerp(HudStyle.GOLD_PALE, 0.55)
		"available":
			return HudStyle.PARCHMENT_LIGHT
		_:
			return HudStyle.PARCHMENT_DARK.lerp(HudStyle.INK_FADED, 0.25)


## Pastilles de déblocage (UX5-T1) : « ⚔ unité » / « ⛫ bâtiment », nom court.
static func unlock_pills(node: Dictionary) -> Array:
	var pills: Array = []
	var unlocks: Dictionary = node.get("unlock_names", {})
	for unit_name in unlocks.get("units", []):
		pills.append({"kind": "unit", "glyph": "⚔", "name": str(unit_name)})
	for building_name in unlocks.get("buildings", []):
		pills.append({"kind": "building", "glyph": "⛫", "name": str(building_name)})
	return pills


static func _short(text: String) -> String:
	return text if text.length() <= PILL_NAME_CHARS else text.substr(0, PILL_NAME_CHARS - 1) + "…"


## Pied du nœud : ligne de pastilles de déblocage et effet clé.
func _add_foot(button: Button, node: Dictionary) -> void:
	var foot := VBoxContainer.new()
	foot.name = "Foot"
	foot.mouse_filter = Control.MOUSE_FILTER_IGNORE
	foot.add_theme_constant_override("separation", 0)
	foot.position = Vector2(6, NODE_SIZE.y - FOOT_HEIGHT - 2)
	foot.size = Vector2(NODE_SIZE.x - 12, FOOT_HEIGHT)
	var pills := unlock_pills(node)
	var parts := PackedStringArray()
	for index in mini(pills.size(), MAX_PILLS):
		parts.append("%s %s" % [pills[index]["glyph"], _short(str(pills[index]["name"]))])
	if pills.size() > MAX_PILLS:
		parts.append("+%d" % (pills.size() - MAX_PILLS))
	var pill_line := _foot_label("  ".join(parts), HudStyle.INK_SOFT)
	pill_line.name = "Pills"
	foot.add_child(pill_line)
	var effects: Array = node.get("effects", [])
	var effect_text := effect_label(effects[0]) if not effects.is_empty() and effects[0] is Dictionary else ""
	var effect_line := _foot_label(effect_text, HudStyle.WAX_GREEN if str(node.get("state", "")) != "locked" else HudStyle.INK_FADED)
	effect_line.name = "Effect"
	foot.add_child(effect_line)
	button.add_child(foot)


func _foot_label(text: String, color: Color) -> Label:
	var label := Label.new()
	label.text = text
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	label.clip_text = true
	label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	label.add_theme_font_size_override("font_size", 14)
	label.add_theme_color_override("font_color", color)
	label.custom_minimum_size = Vector2(0, 18)
	return label


## Info-bulle : description, effets, déblocages, année historique, coût (surcoût anachronique).
static func tooltip_for(node: Dictionary) -> String:
	var lines := PackedStringArray()
	if str(node.get("lock_note", "")) != "":
		lines.append(str(node["lock_note"]))
	lines.append(str(node.get("name", "")))
	var description: String = str(node.get("description", ""))
	if description != "":
		lines.append(description)
	var effects := PackedStringArray()
	for effect in node.get("effects", []):
		effects.append(effect_label(effect))
	if not effects.is_empty():
		lines.append("Effets : " + ", ".join(effects))
	var unlock_names: Dictionary = node.get("unlock_names", {})
	var unlocked := PackedStringArray()
	for unit_name in unlock_names.get("units", []):
		unlocked.append("unité %s" % unit_name)
	for building_name in unlock_names.get("buildings", []):
		unlocked.append("bâtiment %s" % building_name)
	if not unlocked.is_empty():
		lines.append("Débloque : " + ", ".join(unlocked))
	var year := int(node.get("historical_year", 0))
	if year > 0:
		lines.append("Année historique : %s%d" % ["vers " if bool(node.get("historical_uncertain", false)) else "", year])
	var cost := int(node.get("cost", 0))
	var effective := int(node.get("effective_cost", cost))
	if effective > cost:
		lines.append("Coût : %d points (%d + %s %% : trop en avance sur son temps)" % [effective, cost, RuleValues.text("anachronism_surcharge_percent")])
	else:
		lines.append("Coût : %d points" % cost)
	return "\n".join(lines)


static func effect_label(effect: Dictionary) -> String:
	var kind: String = str(effect.get("kind", ""))
	var label: String = str(EFFECT_LABELS.get(kind, kind))
	var category: String = str(effect.get("unit_category", ""))
	if category != "":
		label += " (%s)" % CATEGORY_LABELS.get(category, category)
	var value := float(effect.get("value", 0))
	var sign := "+" if value >= 0 else ""
	var suffix := " %" if str(effect.get("mode", "add")) == "percent" else ""
	return "%s %s%s%s" % [label, sign, str(int(value)) if is_equal_approx(value, roundf(value)) else str(value), suffix]
