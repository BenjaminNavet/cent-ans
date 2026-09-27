class_name Lettrine
extends Control

## Lot UI1 — lettrine enluminée pour les titres de fenêtres : initiale d'or cernée d'encre sur
## champ d'azur semé de points d'or (filet d'or, filet de gueules), suite du titre en IM Fell.
##
## Posée en surimpression sur un `Label` existant (`Lettrine.attach(label)`) : le label garde
## son texte, sa couleur et sa place dans la mise en page (les scripts et les tests continuent
## de lire `label.text`) ; il est seulement rendu transparent et la lettrine redessine le titre.
## Suit les changements de texte et de couleur du label. Rendu seulement, aucune image.

## Q5 : 5 px faisaient lire « D iplomatie » (recettes Q3 et Q5) ; l'initiale colle au mot.
const GAP := 1.0
## Les lettrines médiévales ne portent pas d'accents (et l'accent déborderait du champ) :
## l'initiale est dessinée sans, le reste du titre les garde.
const UNACCENTED := {
	"À": "A", "Â": "A", "Ä": "A", "Ç": "C", "É": "E", "È": "E", "Ê": "E", "Ë": "E",
	"Î": "I", "Ï": "I", "Ô": "O", "Ö": "O", "Ù": "U", "Û": "U", "Ü": "U", "Ÿ": "Y",
}

var _label: Label
var _box := 0.0
var _text := ""
var _color := Color.BLACK
var _font_size := 22


## Habille `label` d'une lettrine. `box` : côté du champ d'azur (défaut : 1,5 × la taille de
## police du label). Idempotent : un second appel renvoie la lettrine déjà posée.
static func attach(label: Label, box: float = 0.0) -> Lettrine:
	for child in label.get_children():
		if child is Lettrine:
			return child
	var lettrine := Lettrine.new()
	lettrine.name = "Lettrine"
	lettrine._label = label
	lettrine._font_size = label.get_theme_font_size("font_size")
	lettrine._box = box if box > 0.0 else roundf(float(lettrine._font_size) * 1.5)
	label.add_child(lettrine)
	return lettrine


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_label.self_modulate = Color(1, 1, 1, 0)
	# CV3-0 (#10) : l'ancienne marge posée sur le style "normal" du label (invisible, donc sans
	# effet visuel) faisait doublon avec `custom_minimum_size` ci-dessous et perturbait le calcul
	# de hauteur minimale propre du label (mesuré : -8 px de haut avec la marge posée). Seul
	# `custom_minimum_size`, recalculé par `_sync`, réserve la place de la lettrine.
	_sync()


func _process(_delta: float) -> void:
	if is_visible_in_tree():
		_sync()


## Relit texte et couleur du label ; ajuste sa taille minimale à la largeur dessinée.
func _sync() -> void:
	var color := _label.get_theme_color("font_color")
	if _label.text == _text and color == _color:
		return
	_text = _label.text
	_color = color
	var rest := _text.substr(1) if _has_initial() else _text
	var width := FrontEndStyle.title_font().get_string_size(rest, HORIZONTAL_ALIGNMENT_LEFT, -1, _font_size).x
	_label.custom_minimum_size = Vector2(_box + GAP + width + 2.0, _box + 2.0)
	queue_redraw()


func _has_initial() -> bool:
	if _text.is_empty():
		return false
	var first := _text.substr(0, 1)
	return first.to_upper() != first.to_lower()


func _draw() -> void:
	var font := FrontEndStyle.title_font()
	var box := Rect2(Vector2(0.0, (size.y - _box) * 0.5), Vector2(_box, _box))
	var rest := _text
	if _has_initial():
		rest = _text.substr(1)
		var initial := _text.substr(0, 1).to_upper()
		_draw_initial(font, box, str(UNACCENTED.get(initial, initial)))
	# Suite du titre : même couleur que le label (couleur de faction éventuelle), centrée
	# verticalement sur le champ.
	var baseline := box.get_center().y + font.get_ascent(_font_size) - font.get_height(_font_size) * 0.5
	draw_string(font, Vector2(_box + GAP, baseline), rest, HORIZONTAL_ALIGNMENT_LEFT, -1, _font_size, _color)


func _draw_initial(font: Font, box: Rect2, letter: String) -> void:
	draw_rect(Rect2(box.position + Vector2(2, 2), box.size), Color(0, 0, 0, 0.30))
	draw_rect(box, FrontEndStyle.AZURE)
	# Semis de losanges d'or (une case sur deux, 4 × 4).
	var step := _box / 4.0
	var d := maxf(1.2, _box * 0.045)
	for j in 4:
		for i in 4:
			if (i + j) % 2 == 0:
				continue
			var c := box.position + Vector2((i + 0.5) * step, (j + 0.5) * step)
			draw_colored_polygon(PackedVector2Array([c + Vector2(0, -d), c + Vector2(d, 0), c + Vector2(0, d), c + Vector2(-d, 0)]), Color(FrontEndStyle.GOLD, 0.45))
	draw_rect(box, FrontEndStyle.GOLD, false, 1.5)
	draw_rect(box.grow(-3.0), Color(FrontEndStyle.GULES, 0.9), false, 1.0)
	# Initiale cernée d'encre, capitale d'environ deux tiers du champ.
	var letter_size := int(_box * 1.1)
	var letter_width := font.get_string_size(letter, HORIZONTAL_ALIGNMENT_LEFT, -1, letter_size).x
	var baseline := box.get_center().y + font.get_ascent(letter_size) * 0.36
	var pos := Vector2(box.position.x + (_box - letter_width) * 0.5, baseline)
	draw_string_outline(font, pos, letter, HORIZONTAL_ALIGNMENT_LEFT, -1, letter_size, 4, Color(0.08, 0.04, 0.02, 0.9))
	draw_string(font, pos, letter, HORIZONTAL_ALIGNMENT_LEFT, -1, letter_size, FrontEndStyle.GOLD)
