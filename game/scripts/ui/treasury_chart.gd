class_name TreasuryChart
extends Control

## Courbe du trésor sur les dernières saisons (audit A3 E2, lot U3) : une seule série, à l'encre
## sur le parchemin, points aux saisons résolues, ligne du zéro pointillée si le trésor passe
## sous zéro, valeur actuelle écrite au bout de la courbe, plus bas et plus haut à gauche.
## Données : `get_faction_economy().budget_history` (`{turn, treasury, net, change}`, au plus
## 12 saisons) plus le trésor actuel. Infobulle : saison par saison, variation et solde.

const INK := Color(0.30, 0.20, 0.10)
const LINE := Color(0.45, 0.28, 0.10)
const FILL := Color(0.72, 0.55, 0.20, 0.18)
const GRID := Color(0.40, 0.28, 0.14, 0.25)
const NEGATIVE := Color(0.55, 0.12, 0.10)
const SEASONS := ["Printemps", "Été", "Automne", "Hiver"]
const START_YEAR := 1337

## Points `[{turn, treasury, change}]`, du plus ancien au plus récent (dernier = maintenant).
var points: Array = []


func _init() -> void:
	custom_minimum_size = Vector2(0, 96)
	mouse_filter = Control.MOUSE_FILTER_PASS


## `history` : `budget_history` ; `treasury` : trésor actuel (point « maintenant » si différent
## du dernier point, par exemple après un recrutement payé ce tour-ci).
func set_history(history: Array, treasury: int, current_turn: int) -> void:
	points.clear()
	for record in history:
		points.append({"turn": int(record.get("turn", 0)) + 1, "treasury": int(record.get("treasury", 0)), "change": int(record.get("change", 0)), "net": int(record.get("net", 0))})
	if points.is_empty() or int(points.back()["treasury"]) != treasury:
		var change := treasury - int(points.back()["treasury"]) if not points.is_empty() else 0
		points.append({"turn": current_turn, "treasury": treasury, "change": change, "now": true})
	tooltip_text = _tooltip()
	queue_redraw()


func _make_custom_tooltip(for_text: String) -> Object:
	return TooltipHost.bubble(for_text, self)


static func season_label(turn: int) -> String:
	return "%s %d" % [SEASONS[posmod(turn, 4)], START_YEAR + int(turn / 4.0)]


func _tooltip() -> String:
	var lines := PackedStringArray(["[b]Trésor, saison par saison[/b]"])
	for point in points:
		var text := "%s : %s" % [season_label(int(point["turn"])), Money.amount(int(point["treasury"]))]
		if points.find(point) > 0:
			text += " (%s)" % Money.signed(int(point["change"]))
		lines.append(text)
	if points.size() < 2:
		lines.append("La courbe se remplit au fil des saisons (douze au plus).")
	return "\n".join(lines)


func _draw() -> void:
	var font := get_theme_default_font()
	var font_size := 12
	var left := 64.0
	var right := 12.0
	var top := 8.0
	var bottom := 18.0
	var plot := Rect2(left, top, maxf(10.0, size.x - left - right), maxf(10.0, size.y - top - bottom))
	draw_rect(plot, GRID, false, 1.0)
	if points.is_empty():
		return
	var low := 0
	var high := 0
	var first := true
	for point in points:
		var value := int(point["treasury"])
		low = value if first else mini(low, value)
		high = value if first else maxi(high, value)
		first = false
	# Marges : l'échelle part de zéro quand le trésor est positif (on juge la pente honnêtement).
	low = mini(low, 0)
	if high == low:
		high = low + 1
	var span := float(high - low)
	var count := points.size()
	var step := plot.size.x / float(maxi(count - 1, 1))
	var coords := PackedVector2Array()
	for index in count:
		var value := int(points[index]["treasury"])
		var x := plot.position.x + (step * index if count > 1 else plot.size.x * 0.5)
		var y := plot.end.y - (float(value - low) / span) * plot.size.y
		coords.append(Vector2(x, y))
	var zero_y := plot.end.y - (float(0 - low) / span) * plot.size.y
	# Aire sous la courbe, jusqu'au zéro.
	if coords.size() >= 2:
		var area := PackedVector2Array(coords)
		area.append(Vector2(coords[coords.size() - 1].x, zero_y))
		area.append(Vector2(coords[0].x, zero_y))
		draw_colored_polygon(area, FILL)
		draw_polyline(coords, LINE, 2.0, true)
	if low < 0:
		_dashed(Vector2(plot.position.x, zero_y), Vector2(plot.end.x, zero_y), NEGATIVE)
	for index in coords.size():
		var value := int(points[index]["treasury"])
		var is_last := index == coords.size() - 1
		draw_circle(coords[index], 3.5 if is_last else 2.5, NEGATIVE if value < 0 else LINE)
	# Axe : plus haut et plus bas (ou zéro), à gauche.
	draw_string(font, Vector2(4, plot.position.y + font_size * 0.8), _short(high), HORIZONTAL_ALIGNMENT_LEFT, left - 8, font_size, INK)
	draw_string(font, Vector2(4, plot.end.y), _short(low), HORIZONTAL_ALIGNMENT_LEFT, left - 8, font_size, INK)
	# Saisons : première et dernière sous l'axe.
	draw_string(font, Vector2(plot.position.x, size.y - 3), season_label(int(points[0]["turn"])), HORIZONTAL_ALIGNMENT_LEFT, plot.size.x * 0.5, font_size - 1, INK)
	if count > 1:
		draw_string(font, Vector2(plot.end.x - plot.size.x * 0.5, size.y - 3), "maintenant", HORIZONTAL_ALIGNMENT_RIGHT, plot.size.x * 0.5, font_size - 1, INK)


func _dashed(from: Vector2, to: Vector2, color: Color) -> void:
	var length := from.distance_to(to)
	var direction := (to - from).normalized()
	var at := 0.0
	while at < length:
		draw_line(from + direction * at, from + direction * minf(at + 5.0, length), color, 1.0)
		at += 9.0


## « 60 000 ₶ », « 1,2 M ₶ » au-delà du million (axe compact).
static func _short(value: int) -> String:
	if absi(value) >= 1000000:
		return "%s M %s" % [String.num(value / 1000000.0, 1).replace(".", ","), Money.SYMBOL]
	return Money.amount(value)
