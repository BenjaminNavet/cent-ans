class_name Accessibility
extends RefCounted

## Lot U12 (audit A3 § 6) : lecture des réglages d'accessibilité (onglet « Accessibilité » des
## réglages, autoload `Settings`) et petits outils partagés : motifs du mode daltonien, symboles
## de relation et de moral, encre renforcée du mode contrasté. Aucune règle de jeu.

const KEY_COLORBLIND := "access/colorblind"
const KEY_REDUCE_MOTION := "access/reduce_motion"
const KEY_HIGH_CONTRAST := "access/high_contrast"

## Symbole de chaque relation diplomatique (carte, panneau, légende), lisible sans les couleurs.
const RELATION_SYMBOLS := {
	"self": "♔", "war": "⚔", "truce": "⌛", "peace": "·", "alliance": "⚭", "vassal": "⚜",
	"suzerain": "⚜", "enemy": "⚔", "ally": "⚭", "neutral": "·",
}


static func _settings() -> Node:
	var tree := Engine.get_main_loop() as SceneTree
	return tree.root.get_node_or_null("/root/Settings") if tree != null else null


static func _flag(key: String) -> bool:
	var settings := _settings()
	return settings != null and bool(settings.call("get_value", key))


## Mode daltonien : motifs et symboles en plus des couleurs (relations, moral, jauges).
static func colorblind() -> bool:
	return _flag(KEY_COLORBLIND)


## « Réduire les animations » : pas de fondu, pas de secousse ni de travelling de caméra.
static func reduce_motion() -> bool:
	return _flag(KEY_REDUCE_MOTION)


## Contraste renforcé : encre plus sombre, bords plus épais, parchemin plus clair.
static func high_contrast() -> bool:
	return _flag(KEY_HIGH_CONTRAST)


## Contraste renforcé sur un thème (parchemin) : encres sombres assombries, parchemins clairs
## éclaircis, bords épaissis et foncés. Réversible (valeurs d'origine gardées en métadonnées).
static func apply_contrast(theme: Theme, on: bool) -> void:
	if theme == null or bool(theme.get_meta("high_contrast", false)) == on:
		return
	theme.set_meta("high_contrast", on)
	if not theme.has_meta("contrast_colors"):
		var colors := {}
		for type_name in theme.get_color_type_list():
			for color_name in theme.get_color_list(type_name):
				colors["%s/%s" % [type_name, color_name]] = theme.get_color(color_name, type_name)
		theme.set_meta("contrast_colors", colors)
	var originals: Dictionary = theme.get_meta("contrast_colors")
	for key: String in originals:
		var original: Color = originals[key]
		var parts := key.split("/")
		var color := original
		if on and original.get_luminance() < 0.5:
			color = original.lerp(Color.BLACK, 0.55)
		theme.set_color(parts[1], parts[0], color)
	var seen := {}
	for type_name in theme.get_stylebox_type_list():
		for box_name in theme.get_stylebox_list(type_name):
			var box := theme.get_stylebox(box_name, type_name) as StyleBoxFlat
			if box == null or seen.has(box.get_instance_id()):
				continue
			seen[box.get_instance_id()] = true
			_contrast_box(box, on)


static func _contrast_box(box: StyleBoxFlat, on: bool) -> void:
	if not box.has_meta("contrast_original"):
		box.set_meta("contrast_original", {
			"bg": box.bg_color, "border": box.border_color,
			"widths": [box.border_width_left, box.border_width_top, box.border_width_right, box.border_width_bottom]})
	var original: Dictionary = box.get_meta("contrast_original")
	var widths: Array = original["widths"]
	var extra := 1 if on else 0
	box.border_width_left = int(widths[0]) + (extra if int(widths[0]) > 0 else 0)
	box.border_width_top = int(widths[1]) + (extra if int(widths[1]) > 0 else 0)
	box.border_width_right = int(widths[2]) + (extra if int(widths[2]) > 0 else 0)
	box.border_width_bottom = int(widths[3]) + (extra if int(widths[3]) > 0 else 0)
	var border: Color = original["border"]
	box.border_color = border.lerp(Color.BLACK, 0.5) if on else border
	var bg: Color = original["bg"]
	box.bg_color = bg.lerp(Color(1, 0.98, 0.92, bg.a), 0.45) if (on and bg.get_luminance() > 0.6) else bg


## Symbole d'une relation (`war`, `alliance`…), vide si inconnue.
static func relation_symbol(relation: String) -> String:
	return str(RELATION_SYMBOLS.get(relation, ""))


## Symbole d'un niveau 0..1 (moral, attitude) : ▲ bon, ■ moyen, ▼ bas.
static func level_symbol(ratio: float) -> String:
	if ratio >= 0.6:
		return "▲"
	if ratio >= 0.3:
		return "■"
	return "▼"

