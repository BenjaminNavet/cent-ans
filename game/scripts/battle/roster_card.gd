class_name RosterCard
extends Control

## Carte de régiment des écrans d'avant- et d'après-bataille (UB1) : illustration
## peinte du type d'unité (`assets/illustrations/<type>.jpg`) ou, à défaut, blason + icône,
## bandeau aux couleurs du camp, effectif en gros, chevrons d'expérience, barre d'effectif.
## En mode « bilan » (`set_losses`), la part tombée est voilée de rouge et la carte affiche
## « −pertes » et les ennemis tués. Le nom complet et le détail sont dans l'infobulle.
## Aucune règle : n'affiche que les dictionnaires reçus.

signal pressed(card: RosterCard)

const WIDTH := 58.0
const HEIGHT := 84.0

var unit: Dictionary = {}
var side_color := Color(0.3, 0.3, 0.6)
var faction := ""
var illustration: Texture2D = null
var heraldry: Texture2D = null
var class_icon: Texture2D = null
var losses: int = -1  # -1 : carte d'avant-bataille
var killed: int = 0
var fate: String = ""
var hero: bool = false
var general: bool = false


## `data` : entrée `units` d'un `BattleSetup` (unit_type, name, soldiers, max_soldiers,
## morale, experience, category).
func setup(data: Dictionary, color: Color, faction_id: String, is_general: bool = false) -> RosterCard:
	unit = data
	side_color = color
	faction = faction_id
	general = is_general
	custom_minimum_size = Vector2(WIDTH, HEIGHT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	var type_id := str(data.get("unit_type", data.get("type", "")))
	illustration = UnitCard.illustration_for(type_id)
	heraldry = PortraitLoader.heraldry_texture(faction_id)
	var library := HudStyle.icon_library()
	if library != null:
		class_icon = library.call("get_icon", type_id, "unit")
	_refresh_tooltip()
	return self


## Bilan : `lost` hommes perdus, `enemy_killed` ennemis tués, `unit_fate` (« tient le champ »…).
func set_losses(lost: int, enemy_killed: int, unit_fate: String, is_hero: bool = false) -> void:
	losses = lost
	killed = enemy_killed
	fate = unit_fate
	hero = is_hero
	_refresh_tooltip()
	queue_redraw()


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.pressed and (event as InputEventMouseButton).button_index == MOUSE_BUTTON_LEFT:
		pressed.emit(self)


func _refresh_tooltip() -> void:
	var name_text := str(unit.get("name", "?"))
	var soldiers := int(unit.get("soldiers", 0))
	var lines := PackedStringArray()
	lines.append(name_text + (" (général)" if general else ""))
	lines.append("Effectif : %d / %d" % [soldiers, int(unit.get("max_soldiers", soldiers))])
	if unit.has("morale"):
		lines.append("Moral : %d · expérience : %d" % [int(unit.get("morale", 0)), int(unit.get("experience", 0))])
	if losses >= 0:
		lines.append("Pertes : %d · ennemis abattus : %d" % [losses, killed])
		if fate != "":
			lines.append("Sort : %s" % fate)
		if hero:
			lines.append("Héros de la bataille")
	tooltip_text = "\n".join(lines)


func _draw() -> void:
	var rect := Rect2(Vector2.ZERO, size)
	if illustration != null:
		BattleUiKit.draw_cover(self, illustration, rect, Color.WHITE, 0.2)
	else:
		draw_rect(rect, side_color.darkened(0.35))
		if heraldry != null:
			draw_texture_rect(heraldry, Rect2(size.x * 0.12, size.y * 0.12, size.x * 0.76, size.x * 0.88), false, Color(1, 1, 1, 0.35))
		if class_icon != null:
			var icon := size.x * 0.56
			var center := Vector2(size.x * 0.5, size.y * 0.42)
			draw_circle(center, icon * 0.55, Color(0.95, 0.9, 0.78, 0.9))
			draw_texture_rect(class_icon, Rect2(center - Vector2(icon, icon) * 0.5, Vector2(icon, icon)), false)
	draw_rect(Rect2(0, 0, size.x, 4), side_color)
	if illustration != null and class_icon != null:
		draw_rect(Rect2(1, 5, 16, 16), Color(0.95, 0.9, 0.78, 0.92))
		draw_texture_rect(class_icon, Rect2(2, 6, 14, 14), false)
	var soldiers := int(unit.get("soldiers", 0))
	var max_soldiers := maxi(int(unit.get("max_soldiers", soldiers)), 1)
	# Bilan : la part tombée voilée de rouge, depuis le haut.
	if losses > 0:
		var before := maxi(int(unit.get("engaged", soldiers + losses)), 1)
		var fraction := clampf(float(losses) / float(before), 0.0, 1.0)
		draw_rect(Rect2(0, 0, size.x, size.y * fraction), Color(0.55, 0.05, 0.03, 0.45))
	var font := BattleUiKit.body_font(700)
	if font == null:
		font = get_theme_default_font()
	# Chevrons d'expérience (1 par tranche de 3 points, jusqu'à 3).
	var chevrons := mini(int(unit.get("experience", 0)) / 3, 3)
	for i in chevrons:
		var y := 26.0 + i * 5.0
		draw_polyline(PackedVector2Array([Vector2(3, y), Vector2(7, y - 3), Vector2(11, y)]), BattleUiKit.GOLD, 2.0)
	if general:
		_draw_star(Vector2(size.x - 8, 12), 6.0)
	if hero:
		_draw_laurel(Vector2(size.x - 9, 28))
	# Bas : dégradé, effectif (ou pertes), barre d'effectif.
	var shade_top := size.y - 32.0
	draw_polygon(PackedVector2Array([Vector2(0, shade_top), Vector2(size.x, shade_top), Vector2(size.x, size.y), Vector2(0, size.y)]),
		PackedColorArray([Color(0, 0, 0, 0), Color(0, 0, 0, 0), Color(0, 0, 0, 0.85), Color(0, 0, 0, 0.85)]))
	var main_text := str(soldiers)
	var main_color := Color(1, 0.97, 0.88)
	if losses >= 0:
		main_text = "−%d" % losses if losses > 0 else "0"
		main_color = Color(1, 0.62, 0.55) if losses > 0 else Color(0.8, 0.95, 0.75)
	draw_string_outline(font, Vector2(3, size.y - 9), main_text, HORIZONTAL_ALIGNMENT_LEFT, -1, 17, 4, Color(0.08, 0.05, 0.02))
	draw_string(font, Vector2(3, size.y - 9), main_text, HORIZONTAL_ALIGNMENT_LEFT, -1, 17, main_color)
	if losses >= 0 and killed > 0:
		# Épées croisées dessinées (pas de glyphe : la police n'en a pas), puis le nombre de tués.
		var o := Vector2(4, size.y - 36)
		var gold := Color(1, 0.85, 0.4)
		for line in [[o, o + Vector2(9, 9)], [o + Vector2(9, 0), o + Vector2(0, 9)]]:
			draw_line(line[0], line[1], Color(0.08, 0.05, 0.02), 3.5)
			draw_line(line[0], line[1], gold, 1.6)
		var kills := str(killed)
		draw_string_outline(font, Vector2(15, size.y - 27), kills, HORIZONTAL_ALIGNMENT_LEFT, -1, 12, 3, Color(0.08, 0.05, 0.02))
		draw_string(font, Vector2(15, size.y - 27), kills, HORIZONTAL_ALIGNMENT_LEFT, -1, 12, gold)
	var bar := Rect2(2, size.y - 5, size.x - 4, 3)
	draw_rect(bar, Color(0.15, 0.1, 0.06, 0.9))
	draw_rect(Rect2(bar.position, Vector2(bar.size.x * clampf(float(soldiers) / float(max_soldiers), 0.0, 1.0), bar.size.y)), Color(0.85, 0.78, 0.55))
	if soldiers <= 0 and losses >= 0:
		draw_rect(rect, Color(0.25, 0.22, 0.2, 0.55))
		var ink := Color(0.1, 0.06, 0.04, 0.8)
		draw_line(Vector2(size.x * 0.25, size.y * 0.2), Vector2(size.x * 0.75, size.y * 0.6), ink, 3.0)
		draw_line(Vector2(size.x * 0.75, size.y * 0.2), Vector2(size.x * 0.25, size.y * 0.6), ink, 3.0)
	draw_rect(rect, BattleUiKit.GOLD if hero else Color(0.42, 0.29, 0.16), false, 2.0)


func _draw_star(center: Vector2, radius: float) -> void:
	var points := PackedVector2Array()
	for i in 10:
		var angle := -PI * 0.5 + i * PI / 5.0
		points.append(center + Vector2(cos(angle), sin(angle)) * (radius if i % 2 == 0 else radius * 0.45))
	draw_colored_polygon(points, BattleUiKit.GOLD)
	draw_polyline(points + PackedVector2Array([points[0]]), BattleUiKit.INK, 1.0)


## Petite couronne de laurier dorée (héros de la bataille).
func _draw_laurel(center: Vector2) -> void:
	draw_arc(center, 6.0, PI * 0.15, PI * 0.85, 8, BattleUiKit.GOLD, 2.0)
	draw_arc(center, 6.0, PI * 1.15, PI * 1.85, 8, BattleUiKit.GOLD, 2.0)
	draw_circle(center, 2.0, BattleUiKit.GOLD)
