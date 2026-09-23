class_name EndTurnCluster
extends Control

## Cloche de fin de saison (HUD de campagne, bas à droite ; lot F10a) : gros bouton rond
## (cloche + saison/année) qui termine le tour — raccourci `campaign_end_turn` (Entrée) —,
## entouré d'un éventail de pastilles d'alerte regroupées par type.
##
## Aucune règle de jeu : les alertes sont fournies par l'appelant (`set_alerts`, lot F3) sous
## la forme `{kind, text, province_id?, army_id?, character_id?, blocking?}`. Types connus :
## `enemy_army`, `siege`, `construction_done`, `research_done`, `debt`, `chronicle_decision`,
## `idle_character` (les autres s'affichent avec une pastille neutre).
##
## Une alerte « bloquante » (`blocking: true`, ou tout `chronicle_decision`) change le bouton :
## liseré rubrique, mention « Décision », et un clic émet `alert_activated(alerte bloquante)`
## au lieu de `end_turn_requested`.

## Fin de saison demandée (clic ou raccourci, aucune alerte bloquante).
signal end_turn_requested
## Clic sur une pastille (alertes du même type parcourues à tour de rôle), ou sur le bouton
## quand une décision bloque la fin de saison.
signal alert_activated(alert: Dictionary)

## Ordre d'affichage des types dans l'éventail (du bas-gauche vers le haut).
const KIND_ORDER := [
	"chronicle_decision", "enemy_army", "siege", "debt", "idle_character", "construction_done", "research_done"]
const KIND_LABELS := {
	"chronicle_decision": "Décision de chronique",
	"enemy_army": "Armée ennemie",
	"siege": "Siège",
	"debt": "Dette",
	"idle_character": "Personnage sans affectation",
	"construction_done": "Construction achevée",
	"research_done": "Recherche achevée",
}
## Types dessinés sur cire rouge (danger) ; les autres sur parchemin.
const DANGER_KINDS := ["chronicle_decision", "enemy_army", "siege", "debt"]
const BUTTON_RADIUS := 62.0
const BADGE_RADIUS := 17.0
const FAN_RADIUS := 120.0
const FAN_START := PI * 0.92
const FAN_END := PI * 1.58
const MAX_BADGES := 7

## Relie le bouton à l'action `campaign_end_turn` (désactiver si un autre bouton la porte).
@export var use_shortcut: bool = true

var alerts: Array = []
var date_label: String = ""
var turn: int = -1

var _groups: Array = []  # [{kind, alerts[]}] dans l'ordre d'affichage
var _cursor: Dictionary = {}  # kind → prochain index à activer
var _badges: Array[AlertBadge] = []
var _button: Button
var _hover := false
var _enabled := true


func _ready() -> void:
	custom_minimum_size = Vector2(FAN_RADIUS + BUTTON_RADIUS + BADGE_RADIUS + 16.0, FAN_RADIUS + BUTTON_RADIUS + BADGE_RADIUS + 16.0)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	_button = Button.new()
	_button.flat = true
	for style in ["normal", "hover", "pressed", "disabled", "focus", "hover_pressed"]:
		_button.add_theme_stylebox_override(style, StyleBoxEmpty.new())
	_button.focus_mode = Control.FOCUS_NONE
	_button.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	_button.pressed.connect(_on_button_pressed)
	_button.mouse_entered.connect(func() -> void:
		_hover = true
		queue_redraw())
	_button.mouse_exited.connect(func() -> void:
		_hover = false
		queue_redraw())
	add_child(_button)
	if use_shortcut and InputMap.has_action("campaign_end_turn"):
		var action := InputEventAction.new()
		action.action = "campaign_end_turn"
		var shortcut := Shortcut.new()
		shortcut.events = [action]
		_button.shortcut = shortcut
		_button.shortcut_in_tooltip = false
	resized.connect(_layout)
	_layout()
	_rebuild()


## Saison et année affichées dans la cloche (« Printemps 1337 ») ; `turn_number` facultatif.
func set_date(label: String, turn_number: int = -1) -> void:
	date_label = label
	turn = turn_number
	_update_tooltip()
	queue_redraw()


## Remplace les alertes. Chaque alerte : `{kind, text, province_id?, army_id?,
## character_id?, blocking?}`.
func set_alerts(new_alerts: Array) -> void:
	alerts = new_alerts.duplicate(true)
	_cursor.clear()
	_rebuild()


## Active ou désactive la cloche (pendant une animation de fin de tour, une boîte modale…).
func set_end_turn_enabled(enabled: bool) -> void:
	_enabled = enabled
	if _button != null:
		_button.disabled = not enabled
	queue_redraw()


## Première alerte bloquante, ou `{}`.
func blocking_alert() -> Dictionary:
	for alert in alerts:
		if is_blocking(alert):
			return alert
	return {}


## Vrai si l'alerte empêche de finir la saison.
static func is_blocking(alert: Dictionary) -> bool:
	return bool(alert.get("blocking", str(alert.get("kind", "")) == "chronicle_decision"))


## Groupes affichés : `[{kind, alerts[]}]`.
func get_alert_groups() -> Array:
	return _groups


## Active la prochaine alerte du groupe `group_index` (clic sur une pastille).
func activate_group(group_index: int) -> void:
	if group_index < 0 or group_index >= _groups.size():
		return
	var group: Dictionary = _groups[group_index]
	var list: Array = group["alerts"]
	var kind := str(group["kind"])
	var cursor := int(_cursor.get(kind, 0)) % list.size()
	_cursor[kind] = cursor + 1
	alert_activated.emit(list[cursor])


## Abscisse locale du bord gauche de l'éventail (pour borner le bandeau d'ost à sa gauche).
func fan_left_edge() -> float:
	return _button_center().x - FAN_RADIUS - BADGE_RADIUS - 4.0


func _button_center() -> Vector2:
	return size - Vector2(BUTTON_RADIUS + 6.0, BUTTON_RADIUS + 6.0)


func _layout() -> void:
	if _button == null:
		return
	var center := _button_center()
	_button.position = center - Vector2(BUTTON_RADIUS, BUTTON_RADIUS)
	_button.size = Vector2(BUTTON_RADIUS, BUTTON_RADIUS) * 2.0
	_place_badges()
	queue_redraw()


func _rebuild() -> void:
	if _button == null:
		return
	var by_kind := {}
	var order: Array = []
	for alert in alerts:
		var kind := str((alert as Dictionary).get("kind", "other"))
		if not by_kind.has(kind):
			by_kind[kind] = []
			order.append(kind)
		(by_kind[kind] as Array).append(alert)
	order.sort_custom(func(a: String, b: String) -> bool:
		var ia := KIND_ORDER.find(a)
		var ib := KIND_ORDER.find(b)
		ia = ia if ia >= 0 else 99
		ib = ib if ib >= 0 else 99
		return ia < ib if ia != ib else a < b)
	_groups.clear()
	for kind in order.slice(0, MAX_BADGES):
		_groups.append({"kind": kind, "alerts": by_kind[kind]})
	for badge in _badges:
		badge.queue_free()
	_badges.clear()
	for i in _groups.size():
		var badge := AlertBadge.new()
		badge.cluster = self
		badge.group_index = i
		badge.kind = str(_groups[i]["kind"])
		badge.count = (_groups[i]["alerts"] as Array).size()
		badge.tooltip_text = _group_tooltip(_groups[i])
		badge.size = Vector2(BADGE_RADIUS, BADGE_RADIUS) * 2.0 + Vector2(6, 6)
		add_child(badge)
		_badges.append(badge)
	_place_badges()
	_update_tooltip()
	queue_redraw()


func _place_badges() -> void:
	var center := _button_center()
	var count := _badges.size()
	for i in count:
		var t := 0.5 if count == 1 else float(i) / float(MAX_BADGES - 1)
		var angle := lerpf(FAN_START, FAN_END, t)
		var at := center + Vector2(cos(angle), sin(angle)) * FAN_RADIUS
		_badges[i].position = at - _badges[i].size * 0.5


func _group_tooltip(group: Dictionary) -> String:
	var kind := str(group["kind"])
	var list: Array = group["alerts"]
	var lines := PackedStringArray([str(KIND_LABELS.get(kind, "Avis"))])
	for alert in list.slice(0, 6):
		lines.append("• " + str((alert as Dictionary).get("text", "")))
	if list.size() > 6:
		lines.append("… et %d autre(s)" % (list.size() - 6))
	lines.append("Clic : aller à la suivante")
	return "\n".join(lines)


func _update_tooltip() -> void:
	if _button == null:
		return
	var blocking := blocking_alert()
	if not blocking.is_empty():
		_button.tooltip_text = "Une décision attend avant la fin de la saison :\n%s" % str(blocking.get("text", ""))
	else:
		_button.tooltip_text = "Finir la saison (Entrée)"


func _on_button_pressed() -> void:
	if not _enabled:
		return
	var blocking := blocking_alert()
	if not blocking.is_empty():
		alert_activated.emit(blocking)
		return
	end_turn_requested.emit()


func _draw() -> void:
	var center := _button_center()
	var blocked := not blocking_alert().is_empty()
	# Filet d'or de l'éventail, derrière les pastilles.
	if not _badges.is_empty():
		draw_arc(center, FAN_RADIUS, FAN_START - 0.08, FAN_END + 0.08, 32, HudStyle.GOLD, 1.5, true)
	# Disque : ombre, parchemin, double filet (encre + or ; rubrique si bloqué).
	draw_circle(center + Vector2(2, 3), BUTTON_RADIUS, HudStyle.SHADOW)
	var face := HudStyle.PARCHMENT
	if not _enabled:
		face = HudStyle.PARCHMENT_DARK
	elif _hover:
		face = HudStyle.PARCHMENT_LIGHT
	draw_circle(center, BUTTON_RADIUS, face)
	var ring := HudStyle.RUBRIC if blocked else HudStyle.INK_SOFT
	draw_arc(center, BUTTON_RADIUS - 1.5, 0.0, TAU, 64, ring, 3.0, true)
	draw_arc(center, BUTTON_RADIUS - 7.0, 0.0, TAU, 64, HudStyle.GOLD, 1.5, true)
	# Cloche.
	var ink := HudStyle.INK_FADED if not _enabled else (HudStyle.RUBRIC if blocked else HudStyle.INK)
	_draw_bell(center + Vector2(0, -20), 38.0, ink)
	# Saison / année (ou « Décision »).
	var font := get_theme_default_font()
	var parts := date_label.split(" ", false)
	var line1 := "Décision" if blocked else (parts[0] if parts.size() > 0 else "Fin de tour")
	var line2 := "en attente" if blocked else (" ".join(parts.slice(1)) if parts.size() > 1 else "")
	_draw_centered(font, line1, center + Vector2(0, 17), 15, ink)
	if line2 != "":
		_draw_centered(font, line2, center + Vector2(0, 34), 13, HudStyle.RUBRIC if blocked else HudStyle.INK_SOFT)


func _draw_centered(font: Font, text: String, baseline_center: Vector2, font_size: int, color: Color) -> void:
	var width := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size).x
	draw_string(font, baseline_center - Vector2(width * 0.5, 0), text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, color)


## Cloche au trait : robe évasée, anse, battant.
func _draw_bell(center: Vector2, height: float, color: Color) -> void:
	var h := height * 0.5
	var points := PackedVector2Array()
	# Profil droit de la robe (de l'épaule à la lèvre), puis symétrique.
	var profile := [Vector2(0.0, -0.78), Vector2(0.30, -0.72), Vector2(0.40, -0.45), Vector2(0.44, 0.05),
		Vector2(0.52, 0.42), Vector2(0.70, 0.62), Vector2(0.72, 0.70)]
	for p in profile:
		points.append(center + Vector2(p.x, p.y) * height * 0.62)
	for i in range(profile.size() - 1, -1, -1):
		var p: Vector2 = profile[i]
		points.append(center + Vector2(-p.x, p.y) * height * 0.62)
	draw_colored_polygon(points, color)
	draw_arc(center + Vector2(0, -h * 0.98), h * 0.2, PI, TAU, 12, color, 2.0, true)
	draw_circle(center + Vector2(0, h * 0.62), h * 0.14, color)
	draw_line(center + Vector2(-h * 0.5, h * 0.18), center + Vector2(h * 0.5, h * 0.18), HudStyle.GOLD_PALE, 1.5, true)


## Pastille d'alerte (un type, avec compteur).
class AlertBadge:
	extends Control

	var cluster: EndTurnCluster
	var group_index: int = 0
	var kind: String = ""
	var count: int = 1
	var _hover := false

	func _ready() -> void:
		mouse_filter = Control.MOUSE_FILTER_STOP
		mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
		mouse_entered.connect(func() -> void:
			_hover = true
			queue_redraw())
		mouse_exited.connect(func() -> void:
			_hover = false
			queue_redraw())

	func _has_point(point: Vector2) -> bool:
		return point.distance_to(size * 0.5) <= EndTurnCluster.BADGE_RADIUS + 2.0

	func _gui_input(event: InputEvent) -> void:
		var click := event as InputEventMouseButton
		if click != null and click.pressed and click.button_index == MOUSE_BUTTON_LEFT:
			cluster.activate_group(group_index)
			accept_event()

	func _make_custom_tooltip(for_text: String) -> Object:
		var panel := PanelContainer.new()
		panel.add_theme_stylebox_override("panel", HudStyle.panel_box(8))
		var box := VBoxContainer.new()
		panel.add_child(box)
		var lines := for_text.split("\n")
		for i in lines.size():
			var color := HudStyle.RUBRIC if i == 0 else (HudStyle.INK_FADED if i == lines.size() - 1 else HudStyle.INK)
			box.add_child(HudStyle.label(lines[i], HudStyle.FONT_BODY, color))
		return panel

	func _draw() -> void:
		var center := size * 0.5
		var r := EndTurnCluster.BADGE_RADIUS
		var danger := EndTurnCluster.DANGER_KINDS.has(kind)
		draw_circle(center + Vector2(1, 2), r, HudStyle.SHADOW)
		if danger:
			var wax := HudStyle.WAX_LIGHT if _hover else HudStyle.WAX
			draw_colored_polygon(HudStyle.wax_points(center, r, hash(kind) % 13, 28), wax)
			draw_arc(center, r * 0.78, 0.0, TAU, 28, wax.lightened(0.2), 1.0, true)
		else:
			draw_circle(center, r, HudStyle.PARCHMENT_LIGHT if _hover else HudStyle.PARCHMENT)
			draw_arc(center, r - 1.0, 0.0, TAU, 28, HudStyle.INK_SOFT, 1.5, true)
			draw_arc(center, r - 4.0, 0.0, TAU, 28, HudStyle.GOLD, 1.0, true)
		var glyph_color := HudStyle.PARCHMENT_LIGHT if danger else HudStyle.INK
		var texture := HudStyle.icon("hud_" + kind, "hud")
		if texture != null:
			HudStyle.draw_texture_fit(self, texture, center, r * 1.25)
		else:
			var glyph := "siege_alert" if kind == "siege" else kind
			HudStyle.draw_glyph(self, glyph, center, r * 1.15, glyph_color, HudStyle.WAX if danger else HudStyle.PARCHMENT)
		if count > 1:
			var badge := center + Vector2(r * 0.72, -r * 0.72)
			draw_circle(badge, 8.0, HudStyle.INK)
			var font := get_theme_default_font()
			var text := str(count) if count < 10 else "9+"
			var width := font.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, 11).x
			draw_string(font, badge + Vector2(-width * 0.5, 4.0), text, HORIZONTAL_ALIGNMENT_LEFT, -1, 11, HudStyle.PARCHMENT_LIGHT)
