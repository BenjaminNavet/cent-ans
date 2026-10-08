class_name TechTreeView
extends Control

## Un arbre de technologies (une branche) : colonnes par tier, nœuds reliés par des lignes
## de prérequis dessinées sous les boutons. Couleur par état (`known`, `researching`,
## `available`, `locked`, fourni par `CampaignSim.get_tech_tree`). Clic sur un nœud
## disponible = `research_requested`. Aucune règle ici.

signal research_requested(technology_id: String)
## A6-L4 : Maj+clic sur une technologie disponible = la mettre en file derrière la recherche.
signal queue_requested(technology_id: String)

const NODE_SIZE := Vector2(212, 62)
const COLUMN_STEP := 252.0
const ROW_STEP := 78.0
const MARGIN := Vector2(12, 34)

const STATE_COLORS := {
	"known": Color(0.66, 0.80, 0.58),
	"researching": Color(0.95, 0.78, 0.36),
	"available": Color(0.97, 0.93, 0.80),
	"locked": Color(0.76, 0.73, 0.68),
}
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


## `nodes` : entrées de `get_tech_tree` d'une seule branche.
func show_tree(nodes: Array) -> void:
	_nodes = nodes
	for child in get_children():
		child.queue_free()
	buttons.clear()
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
	queue_redraw()


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
			var color := Color(0.30, 0.45, 0.22, 0.95) if known else Color(0.42, 0.29, 0.16, 0.55)
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
			draw_polyline(points, color, 2.0, true)


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
	var second_line: String
	if state == "researching" or (state == "available" and int(node.get("progress", 0)) > 0):
		second_line = "%d / %d pts — %s" % [int(node.get("progress", 0)), cost, STATE_LABELS[state]]
	else:
		second_line = "%d pts — %s" % [cost, STATE_LABELS.get(state, state)]
	var queue_position := int(node.get("queue_position", 0))
	if queue_position > 0 and state == "available":
		second_line = "%d pts — En file (n° %d)" % [cost, queue_position]
	button.text = "%s\n%s" % [str(node.get("name", id)), second_line]
	RichTooltip.set_tooltip(button, "technology", id, node)  # F2 / IB1 : infobulle en sections (tooltip_for : texte brut)
	var style := StyleBoxFlat.new()
	style.bg_color = STATE_COLORS.get(state, Color(0.8, 0.8, 0.8))
	style.set_border_width_all(2 if state != "researching" else 3)
	style.border_color = Color(0.42, 0.29, 0.16) if state != "locked" else Color(0.55, 0.52, 0.48)
	style.set_corner_radius_all(4)
	style.set_content_margin_all(4)
	var hover := style.duplicate() as StyleBoxFlat
	hover.bg_color = style.bg_color.lightened(0.12)
	for style_name in ["normal", "disabled", "focus"]:
		button.add_theme_stylebox_override(style_name, style)
	button.add_theme_stylebox_override("hover", hover)
	button.add_theme_stylebox_override("pressed", hover)
	var font_color := Color(0.22, 0.14, 0.07) if state != "locked" else Color(0.40, 0.37, 0.33)
	for color_name in ["font_color", "font_disabled_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
		button.add_theme_color_override(color_name, font_color)
	button.disabled = state != "available"
	button.pressed.connect(func() -> void:
		if Input.is_key_pressed(KEY_SHIFT):
			queue_requested.emit(id)
		else:
			research_requested.emit(id))
	return button


## Info-bulle : description, effets, déblocages, année historique, coût (surcoût anachronique).
static func tooltip_for(node: Dictionary) -> String:
	var lines := PackedStringArray()
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
