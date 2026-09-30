class_name FrontEndStyle
extends RefCounted

## Lot MM1 — style des écrans d'accueil (menu, choix de faction, introduction, chargement) :
## polices (IM Fell English pour les titres, EB Garamond pour le texte), couleurs (or, azur,
## encre, vélin), cadres enluminés et boutons de menu (texte seul, filet
## d'or au survol). Rendu seulement.

const TITLE_FONT_PATH := "res://assets/third_party/fonts/im_fell_english/IMFeENrm28P.ttf"
const TITLE_ITALIC_PATH := "res://assets/third_party/fonts/im_fell_english/IMFeENit28P.ttf"
const BODY_FONT_PATH := "res://assets/third_party/fonts/eb_garamond/EBGaramond-VariableFont_wght.ttf"
const BODY_ITALIC_PATH := "res://assets/third_party/fonts/eb_garamond/EBGaramond-Italic-VariableFont_wght.ttf"

const GOLD := Color(0.93, 0.78, 0.42)
const GOLD_DARK := Color(0.62, 0.46, 0.18)
const AZURE := Color(0.10, 0.17, 0.42)
const GULES := Color(0.58, 0.10, 0.08)
const VELLUM := Color(0.95, 0.90, 0.78)
const INK := Color(0.20, 0.13, 0.07)
const FADED_INK := Color(0.40, 0.30, 0.18)
const NIGHT := Color(0.05, 0.04, 0.035)

static var _fonts: Dictionary = {}


static func font(path: String) -> Font:
	if not _fonts.has(path):
		_fonts[path] = load(path) if ResourceLoader.exists(path) else ThemeDB.fallback_font
	return _fonts[path]


static func title_font() -> Font:
	return font(TITLE_FONT_PATH)


static func title_italic() -> Font:
	return font(TITLE_ITALIC_PATH)


static func body_font() -> Font:
	return font(BODY_FONT_PATH)


static func body_italic() -> Font:
	return font(BODY_ITALIC_PATH)


## Libellé stylé (police, taille, couleur, contour sombre facultatif).
static func label(text: String, size: int, color: Color, font_value: Font = null, outline: int = 0) -> Label:
	var result := Label.new()
	result.text = text
	result.add_theme_font_override("font", font_value if font_value != null else body_font())
	result.add_theme_font_size_override("font_size", size)
	result.add_theme_color_override("font_color", color)
	if outline > 0:
		result.add_theme_constant_override("outline_size", outline)
		result.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.75))
	return result


## Panneau vélin opaque bordé d'or (cartes, fiches).
static func vellum_panel(border: Color = GOLD_DARK, radius: int = 3) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = VELLUM
	box.border_color = border
	box.set_border_width_all(2)
	box.set_corner_radius_all(radius)
	box.set_content_margin_all(14)
	box.shadow_color = Color(0, 0, 0, 0.45)
	box.shadow_size = 10
	return box


## Panneau sombre translucide (bandeaux posés sur le décor 3D).
static func night_panel(alpha: float = 0.78) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = Color(NIGHT, alpha)
	box.border_color = Color(GOLD_DARK, 0.9)
	box.border_width_top = 1
	box.border_width_bottom = 1
	box.set_content_margin_all(12)
	return box


## Bouton de menu principal : texte doré sur fond transparent, filet d'or et halo au survol.
## `vpad` : marge verticale des cartouches (4 par défaut ; le menu principal la réduit quand la
## fenêtre est basse, `StartMenu._fit_column`).
static func style_menu_button(button: Button, size: int = 28, vpad: int = 4) -> void:
	button.flat = false
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	button.add_theme_font_override("font", title_font())
	button.add_theme_font_size_override("font_size", size)
	button.add_theme_color_override("font_color", Color(0.93, 0.88, 0.76))
	button.add_theme_color_override("font_hover_color", GOLD)
	button.add_theme_color_override("font_focus_color", GOLD)
	button.add_theme_color_override("font_pressed_color", Color(1.0, 0.92, 0.66))
	button.add_theme_color_override("font_disabled_color", Color(0.55, 0.52, 0.46))
	button.add_theme_constant_override("outline_size", 6)
	button.add_theme_color_override("font_outline_color", Color(0, 0, 0, 0.7))
	var normal := StyleBoxEmpty.new()
	normal.content_margin_left = 22
	normal.content_margin_right = 22
	normal.content_margin_top = vpad
	normal.content_margin_bottom = vpad
	var hover := StyleBoxFlat.new()
	hover.bg_color = Color(0.0, 0.0, 0.0, 0.0)
	hover.bg_color = Color(GOLD_DARK, 0.18)
	hover.border_color = GOLD
	hover.border_width_left = 3
	hover.content_margin_left = 22
	hover.content_margin_right = 22
	hover.content_margin_top = vpad
	hover.content_margin_bottom = vpad
	button.add_theme_stylebox_override("normal", normal)
	button.add_theme_stylebox_override("hover", hover)
	button.add_theme_stylebox_override("focus", hover)
	button.add_theme_stylebox_override("pressed", hover)
	button.add_theme_stylebox_override("disabled", normal)


## Bouton d'action (Commencer, Retour…) : cartouche vélin ou or plein.
static func style_action_button(button: Button, primary: bool, size: int = 22) -> void:
	button.add_theme_font_override("font", title_font())
	button.add_theme_font_size_override("font_size", size)
	var normal := StyleBoxFlat.new()
	normal.bg_color = GULES if primary else Color(NIGHT, 0.85)
	normal.border_color = GOLD
	normal.set_border_width_all(2)
	normal.set_corner_radius_all(2)
	normal.content_margin_left = 28
	normal.content_margin_right = 28
	normal.content_margin_top = 8
	normal.content_margin_bottom = 8
	var hover := normal.duplicate() as StyleBoxFlat
	hover.bg_color = normal.bg_color.lightened(0.15)
	hover.shadow_color = Color(GOLD, 0.35)
	hover.shadow_size = 8
	button.add_theme_stylebox_override("normal", normal)
	button.add_theme_stylebox_override("hover", hover)
	button.add_theme_stylebox_override("focus", hover)
	button.add_theme_stylebox_override("pressed", hover)
	for key in ["font_color", "font_hover_color", "font_focus_color", "font_pressed_color"]:
		button.add_theme_color_override(key, Color(1.0, 0.95, 0.82) if key == "font_color" else GOLD)
