class_name EndTurnCluster
extends Control

## Cloche de fin de saison (HUD de campagne, bas à droite ; lot F10a) : gros bouton rond
## (cloche + saison/année) qui termine le tour — raccourci `campaign_end_turn` (Entrée) —,
## surmonté d'une colonne de pastilles d'alerte regroupées par type. Lot U5 (audit A3, C10) :
## chaque pastille porte un libellé court et un compteur lisibles sans survol.
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
## Libellés courts des pastilles (lot U5), écrits sur la pastille.
const SHORT_LABELS := {
	"chronicle_decision": "Décision",
	"enemy_army": "Armée ennemie",
	"siege": "Siège",
	"debt": "Dette",
	"idle_character": "Sans charge",
	"construction_done": "Chantier fini",
	"research_done": "Recherche finie",
	"research_idle": "Aucune recherche",
	"ransom": "Rançon",
	"table": "Vivres",
	"medicine": "Médecine",
	"herbarium": "Herbier",
	"coinage": "Monnaie",
	"chivalry": "Chevalerie",
	"crusade": "Croisade",  # JR3
	"agent": "Agents",
	"diplomacy_offer": "Proposition",
}
const KIND_LABELS := {
	"chronicle_decision": "Décision de chronique",
	"enemy_army": "Armée ennemie",
	"siege": "Siège",
	"debt": "Dette",
	"idle_character": "Personnage sans affectation",
	"construction_done": "Construction achevée",
	"research_done": "Recherche achevée",
	"table": "Table et vivres",
	"medicine": "Médecine",
	"herbarium": "Herbier",
	"research_idle": "Aucune recherche en cours",
	"ransom": "Captifs et rançons",
	"crusade": "Croisade et ferveur",  # JR3
	"diplomacy_offer": "Proposition diplomatique",
}
## Icône d'un type sans icône propre (`hud_<alias>`).
const ICON_ALIASES := {"diplomacy_offer": "diplomacy", "research_idle": "research", "ransom": "treasury", "coinage": "treasury", "other": "chronicle"}
## Types dessinés sur cire rouge (danger) ; les autres sur parchemin.
const DANGER_KINDS := ["chronicle_decision", "enemy_army", "siege", "debt"]
const BUTTON_RADIUS := 62.0
const BADGE_RADIUS := 17.0
## Colonne des pastilles au-dessus de la cloche (lot U5).
const PILL_WIDTH := 190.0
const PILL_HEIGHT := 30.0
const PILL_GAP := 4.0
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
var _down := false
## DA5 : états du médaillon enluminé de la cloche (`IconLibrary.medallion_states`), vide si absent
## (repli : cloche dessinée au trait).
var _medallion: Dictionary = {}


func _ready() -> void:
	_update_minimum_size()
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
		_down = false
		queue_redraw())
	_button.button_down.connect(func() -> void:
		_down = true
		queue_redraw())
	_button.button_up.connect(func() -> void:
		_down = false
		queue_redraw())
	add_child(_button)
	var library := get_node_or_null("/root/IconLibrary")
	if library != null and library.has_method("medallion_states"):
		_medallion = library.call("medallion_states", "end_turn")
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


## Abscisse locale du bord gauche de la cloche (pour borner le bandeau d'ost à sa gauche ; les
## pastilles sont au-dessus de la cloche, hors de la bande du bandeau).
func fan_left_edge() -> float:
	return _button_center().x - BUTTON_RADIUS - 8.0


## Hauteur de la cloche seule (les panneaux ancrés à droite s'arrêtent au-dessus).
func bell_height() -> float:
	return BUTTON_RADIUS * 2.0 + 12.0


## Ordonnée locale du haut de la colonne des pastilles (ou de la cloche s'il n'y en a pas).
func stack_top() -> float:
	return size.y - bell_height() - _groups.size() * (PILL_HEIGHT + PILL_GAP)


func _update_minimum_size() -> void:
	var count := _groups.size()
	custom_minimum_size = Vector2(maxf(PILL_WIDTH + 4.0, bell_height()), bell_height() + count * (PILL_HEIGHT + PILL_GAP))


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
	# Au-delà de `MAX_BADGES` types : la dernière pastille regroupe le reste (« Autres avis »).
	if order.size() > MAX_BADGES:
		var rest: Array = []
		for kind in order.slice(MAX_BADGES - 1):
			rest.append_array(by_kind[kind])
		_groups[MAX_BADGES - 1] = {"kind": "other", "alerts": rest}
	for badge in _badges:
		badge.queue_free()
	_badges.clear()
	for i in _groups.size():
		var badge := AlertBadge.new()
		badge.cluster = self
		badge.group_index = i
		badge.kind = str(_groups[i]["kind"])
		badge.count = (_groups[i]["alerts"] as Array).size()
		badge.label = short_label(badge.kind)
		badge.glyph = str(((_groups[i]["alerts"] as Array)[0] as Dictionary).get("glyph", ""))
		badge.tooltip_text = _group_tooltip(_groups[i])
		badge.size = Vector2(PILL_WIDTH, PILL_HEIGHT)
		add_child(badge)
		_badges.append(badge)
	_update_minimum_size()
	_place_badges()
	_update_tooltip()
	queue_redraw()


## Pastilles empilées au-dessus de la cloche, la plus urgente juste au-dessus d'elle.
func _place_badges() -> void:
	var bottom := size.y - bell_height()
	for i in _badges.size():
		_badges[i].position = Vector2(size.x - PILL_WIDTH - 2.0, bottom - (i + 1) * (PILL_HEIGHT + PILL_GAP) + PILL_GAP)


## Libellé court d'un type d'alerte (pastille).
static func short_label(kind: String) -> String:
	if kind == "other":
		return "Autres avis"
	return str(SHORT_LABELS.get(kind, KIND_LABELS.get(kind, "Avis")))


func _group_tooltip(group: Dictionary) -> String:
	var kind := str(group["kind"])
	var list: Array = group["alerts"]
	var lines := PackedStringArray([str(KIND_LABELS.get(kind, short_label(kind)))])
	for alert in list.slice(0, 6):
		lines.append("• " + str((alert as Dictionary).get("text", "")))
	if list.size() > 6:
		var more := list.size() - 6
		lines.append("… et %d autre%s" % [more, "s" if more > 1 else ""])
	lines.append("Clic : aller à la suivante")
	return "\n".join(lines)


func _update_tooltip() -> void:
	if _button == null:
		return
	var blocking := blocking_alert()
	if not blocking.is_empty():
		RichTooltip.attach_plain(_button, "end_turn_blocked", {"body": str(blocking.get("text", ""))})
	else:
		RichTooltip.attach_plain(_button, "end_turn_finish")


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
	# Filet d'or reliant la colonne des pastilles à la cloche.
	if not _badges.is_empty():
		draw_line(Vector2(center.x, stack_top() + 4.0), Vector2(center.x, center.y - BUTTON_RADIUS), HudStyle.GOLD, 1.5, true)
	if not _medallion.is_empty():
		_draw_medallion(center, blocked)
		return
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


## DA5 : médaillon enluminé de la cloche (planche validée), état dérivé de la même image
## (éclairci au survol, assombri enfoncé, désaturé désactivé), et banderole de parchemin
## portant la saison et l'année (ou « Décision en attente », filet rubrique autour du disque).
func _draw_medallion(center: Vector2, blocked: bool) -> void:
	var state := "normal"
	if not _enabled:
		state = "disabled"
	elif _down:
		state = "pressed"
	elif _hover:
		state = "hover"
	var texture: Texture2D = _medallion.get(state, _medallion["normal"])
	# Les fleurons de l'image touchent le bord du disque ; l'anneau d'or vaut ~0.86 du rayon.
	var half := BUTTON_RADIUS
	draw_circle(center + Vector2(2, 3), BUTTON_RADIUS * 0.86, HudStyle.SHADOW)
	draw_texture_rect(texture, Rect2(center - Vector2(half, half), Vector2(half, half) * 2.0), false)
	if blocked:
		draw_arc(center, BUTTON_RADIUS * 0.86 + 2.0, 0.0, TAU, 64, HudStyle.RUBRIC, 3.5, true)
	var font := get_theme_default_font()
	var parts := date_label.split(" ", false)
	var line1 := "Décision" if blocked else (parts[0] if parts.size() > 0 else "Fin de tour")
	var line2 := "en attente" if blocked else (" ".join(parts.slice(1)) if parts.size() > 1 else "")
	var ink := HudStyle.INK_FADED if not _enabled else (HudStyle.RUBRIC if blocked else HudStyle.INK)
	var band := Rect2(center + Vector2(-50, BUTTON_RADIUS * 0.50), Vector2(100, 34 if line2 != "" else 20))
	draw_rect(Rect2(band.position + Vector2(1.5, 2.0), band.size), HudStyle.SHADOW)
	draw_rect(band, HudStyle.PARCHMENT_LIGHT if _hover and _enabled else HudStyle.PARCHMENT)
	draw_rect(band, HudStyle.RUBRIC if blocked else HudStyle.INK_SOFT, false, 1.5)
	draw_line(band.position + Vector2(3, band.size.y - 3), band.end - Vector2(3, 3), HudStyle.GOLD, 1.0)
	_draw_centered(font, line1, band.position + Vector2(band.size.x * 0.5, 15), 14, ink)
	if line2 != "":
		_draw_centered(font, line2, band.position + Vector2(band.size.x * 0.5, 30), 12, HudStyle.RUBRIC if blocked else HudStyle.INK_SOFT)


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
	## Libellé court écrit sur la pastille (lot U5).
	var label: String = ""
	## Glyphe fourni par l'alerte (`glyph`), à défaut d'icône.
	var glyph: String = ""
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
			box.add_child(HudStyle.label(lines[i], UiType.size(UiType.CAPTION), color))
		return panel

	func _draw() -> void:
		var r := EndTurnCluster.BADGE_RADIUS * 0.8
		var danger := EndTurnCluster.DANGER_KINDS.has(kind)
		var rect := Rect2(Vector2.ZERO, size)
		# Bandeau parchemin (cire pour un danger), filet d'encre ; disque à gauche pour l'icône.
		var face := HudStyle.PARCHMENT_LIGHT if _hover else HudStyle.PARCHMENT
		if danger:
			face = HudStyle.WAX_LIGHT if _hover else HudStyle.WAX
		draw_rect(Rect2(rect.position + Vector2(1, 2), rect.size), HudStyle.SHADOW)
		draw_rect(rect, face)
		draw_rect(rect.grow(-0.5), HudStyle.WAX_DARK if danger else HudStyle.INK_SOFT, false, 1.0)
		draw_rect(Rect2(0, 0, 3, size.y), HudStyle.GOLD if danger else HudStyle.RUBRIC)
		var center := Vector2(r + 8.0, size.y * 0.5)
		draw_circle(center, r, HudStyle.PARCHMENT_LIGHT)
		draw_arc(center, r, 0.0, TAU, 24, HudStyle.GOLD, 1.0, true)
		var texture := HudStyle.icon("hud_" + str(EndTurnCluster.ICON_ALIASES.get(kind, kind)), "hud")
		var font := get_theme_default_font()
		if texture != null:
			HudStyle.draw_texture_fit(self, texture, center, r * 1.5)
		elif glyph != "":
			var glyph_width := font.get_string_size(glyph, HORIZONTAL_ALIGNMENT_LEFT, -1, 15).x
			draw_string(font, center + Vector2(-glyph_width * 0.5, 5.0), glyph, HORIZONTAL_ALIGNMENT_LEFT, -1, 15, HudStyle.INK)
		else:
			var glyph_id := "siege_alert" if kind == "siege" else kind
			HudStyle.draw_glyph(self, glyph_id, center, r * 1.4, HudStyle.INK, HudStyle.PARCHMENT_LIGHT)
		var ink := HudStyle.PARCHMENT_LIGHT if danger else HudStyle.INK
		var count_text := str(count)
		var count_width := font.get_string_size(count_text, HORIZONTAL_ALIGNMENT_LEFT, -1, 14).x
		var text_left := center.x + r + 7.0
		var text_room := size.x - text_left - count_width - 18.0
		draw_string(font, Vector2(text_left, size.y * 0.5 + 5.0), label, HORIZONTAL_ALIGNMENT_LEFT, text_room, 15, ink)
		# Compteur dans un cartouche à droite.
		var box := Rect2(size.x - count_width - 13.0, 5.0, count_width + 8.0, size.y - 10.0)
		draw_rect(box, HudStyle.PARCHMENT_LIGHT if danger else HudStyle.INK)
		draw_string(font, Vector2(box.position.x + 4.0, size.y * 0.5 + 5.0), count_text, HORIZONTAL_ALIGNMENT_LEFT, -1, 14, HudStyle.WAX_DARK if danger else HudStyle.PARCHMENT_LIGHT)
