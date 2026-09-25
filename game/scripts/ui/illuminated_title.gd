class_name IlluminatedTitle
extends Control

## Lot MM1 — titre enluminé « Cent Ans » : lettrine d'or sur champ d'azur semé de points d'or
## (double filet), suite du titre en capitales de livre, sous-titre italique et filet orné.
## Dessiné en code (aucune image) ; un lent reflet parcourt la lettrine.

@export var title_text: String = "Cent Ans"
@export var subtitle_text: String = "La guerre de Cent Ans, 1337-1453"
@export var letter_box: float = 132.0
@export var title_size: int = 92

var _time := 0.0


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	custom_minimum_size = Vector2(letter_box + 380.0, letter_box + 58.0)


func _process(delta: float) -> void:
	_time += delta
	queue_redraw()


func _draw() -> void:
	var box := Rect2(Vector2(0, 0), Vector2(letter_box, letter_box))
	# Ombre portée, champ d'azur, semis de points d'or, double filet.
	draw_rect(Rect2(box.position + Vector2(6, 7), box.size), Color(0, 0, 0, 0.45))
	draw_rect(box, FrontEndStyle.AZURE)
	var step := letter_box / 7.0
	for j in 7:
		for i in 7:
			if (i + j) % 2 == 0:
				continue
			var c := box.position + Vector2((i + 0.5) * step, (j + 0.5) * step)
			var d := 2.4
			draw_colored_polygon(PackedVector2Array([c + Vector2(0, -d), c + Vector2(d, 0), c + Vector2(0, d), c + Vector2(-d, 0)]), Color(FrontEndStyle.GOLD, 0.55))
	draw_rect(box, FrontEndStyle.GOLD, false, 3.0)
	draw_rect(box.grow(-7.0), Color(FrontEndStyle.GOLD, 0.8), false, 1.2)
	# Lettrine.
	var letter := title_text.substr(0, 1)
	var rest := title_text.substr(1)
	var title_font := FrontEndStyle.title_font()
	var letter_size := int(letter_box * 1.02)
	var letter_width := title_font.get_string_size(letter, HORIZONTAL_ALIGNMENT_LEFT, -1, letter_size).x
	var baseline := box.position.y + letter_box * 0.5 + title_font.get_ascent(letter_size) * 0.36
	var letter_pos := Vector2(box.position.x + (letter_box - letter_width) * 0.5, baseline)
	draw_string_outline(title_font, letter_pos, letter, HORIZONTAL_ALIGNMENT_LEFT, -1, letter_size, 8, Color(0.08, 0.04, 0.02, 0.85))
	var glint := 0.5 + 0.5 * sin(_time * 0.6)
	draw_string(title_font, letter_pos, letter, HORIZONTAL_ALIGNMENT_LEFT, -1, letter_size, FrontEndStyle.GOLD.lerp(Color(1.0, 0.95, 0.75), glint * 0.35))
	# Suite du titre, alignée sur le bas de la lettrine.
	var rest_pos := Vector2(box.end.x + 14.0, box.end.y - 16.0)
	draw_string_outline(title_font, rest_pos + Vector2(3, 4), rest, HORIZONTAL_ALIGNMENT_LEFT, -1, title_size, 10, Color(0, 0, 0, 0.35))
	draw_string_outline(title_font, rest_pos, rest, HORIZONTAL_ALIGNMENT_LEFT, -1, title_size, 7, Color(0.1, 0.06, 0.03, 0.9))
	draw_string(title_font, rest_pos, rest, HORIZONTAL_ALIGNMENT_LEFT, -1, title_size, Color(0.97, 0.92, 0.80))
	# Filet orné et sous-titre.
	var rule_y := box.end.y + 16.0
	var rule_end := box.end.x + 14.0 + title_font.get_string_size(rest, HORIZONTAL_ALIGNMENT_LEFT, -1, title_size).x
	draw_line(Vector2(0, rule_y), Vector2(rule_end, rule_y), Color(FrontEndStyle.GOLD, 0.85), 1.5)
	var mid := Vector2(rule_end * 0.5, rule_y)
	draw_colored_polygon(PackedVector2Array([mid + Vector2(0, -5), mid + Vector2(8, 0), mid + Vector2(0, 5), mid + Vector2(-8, 0)]), FrontEndStyle.GULES)
	var italic := FrontEndStyle.title_italic()
	draw_string_outline(italic, Vector2(2, rule_y + 32.0), subtitle_text, HORIZONTAL_ALIGNMENT_LEFT, -1, 24, 6, Color(0, 0, 0, 0.7))
	draw_string(italic, Vector2(2, rule_y + 32.0), subtitle_text, HORIZONTAL_ALIGNMENT_LEFT, -1, 24, Color(0.93, 0.86, 0.70))
