class_name BattleUiKit
extends RefCounted

## Outils partagés des écrans de bataille d'UB1 (avant-bataille, HUD, écran de fin) : polices
## libres (IM Fell English pour les titres, EB Garamond pour le texte, OFL, dans
## `assets/third_party/fonts/`), styles parchemin, étiquettes, barre d'équilibre, verdict.
## Aucune règle : l'équilibre et les chances viennent du cœur (`get_battle_forecast`).

const TITLE_FONT_PATH := "res://assets/third_party/fonts/im_fell_english/IMFeENrm28P.ttf"
const TITLE_ITALIC_PATH := "res://assets/third_party/fonts/im_fell_english/IMFeENit28P.ttf"
const BODY_FONT_PATH := "res://assets/third_party/fonts/eb_garamond/EBGaramond-VariableFont_wght.ttf"

const INK := Color(0.22, 0.14, 0.07)
const INK_SOFT := Color(0.42, 0.29, 0.16)
const INK_FADED := Color(0.50, 0.44, 0.36)
const RUBRIC := Color(0.62, 0.13, 0.08)
const GOLD := Color(0.72, 0.56, 0.24)
const GOOD := Color(0.26, 0.40, 0.16)
const PARCHMENT := Color(0.93, 0.87, 0.72)
const PARCHMENT_LIGHT := Color(0.97, 0.93, 0.82)
const PARCHMENT_DARK := Color(0.80, 0.70, 0.50)

static var _fonts: Dictionary = {}


static func _font(path: String, weight: int = 0) -> Font:
	var key := "%s#%d" % [path, weight]
	if _fonts.has(key):
		return _fonts[key]
	var base: Font = load(path) if ResourceLoader.exists(path) else null
	var font: Font = base
	if base != null and weight > 0:
		var variation := FontVariation.new()
		variation.base_font = base
		variation.variation_opentype = {"wght": weight}
		font = variation
	_fonts[key] = font
	return font


static func title_font() -> Font:
	return _font(TITLE_FONT_PATH)


static func title_italic_font() -> Font:
	return _font(TITLE_ITALIC_PATH)


static func body_font(weight: int = 0) -> Font:
	return _font(BODY_FONT_PATH, weight)


## Étiquette en EB Garamond (`bold` : graisse 600) ou en IM Fell English (`title`).
static func label(text: String, size: int = 16, color: Color = INK, title: bool = false, bold: bool = false) -> Label:
	var label_node := Label.new()
	label_node.text = text
	var font := title_font() if title else body_font(600 if bold else 0)
	if font != null:
		label_node.add_theme_font_override("font", font)
	label_node.add_theme_font_size_override("font_size", size)
	label_node.add_theme_color_override("font_color", color)
	return label_node


## Style parchemin à double filet (panneaux des écrans d'UB1).
static func parchment_box(margin: int = 16, bg: Color = PARCHMENT, border: Color = INK_SOFT, width: int = 2) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = bg
	box.border_color = border
	box.set_border_width_all(width)
	box.set_corner_radius_all(4)
	box.set_content_margin_all(margin)
	box.shadow_color = Color(0, 0, 0, 0.35)
	box.shadow_size = 10
	return box


## Page de vélin enluminée (kit UI1, bande d'or et bossettes) aux mêmes marges de contenu
## que `parchment_box` : seul le dessin change, pas la mise en page.
static func page_box(margin: int = 16) -> StyleBox:
	return HudStyle.kit_box("panel", HudStyle.PAGE_MARGIN, margin)


## Grande page à rinceaux (écrans de bilan, dialogue d'avant-bataille).
static func illuminated_box(margin: int = 16) -> StyleBox:
	return HudStyle.kit_box("panel_illuminated", HudStyle.ILLUMINATED_MARGIN, margin)


static func button_font(button: Button, size: int = 18) -> void:
	var font := body_font(600)
	if font != null:
		button.add_theme_font_override("font", font)
	button.add_theme_font_size_override("font_size", size)


## Filet doré horizontal (séparateur enluminé).
static func rule(width: float = 0.0) -> Control:
	var line := Control.new()
	line.custom_minimum_size = Vector2(width, 9)
	line.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	line.mouse_filter = Control.MOUSE_FILTER_IGNORE
	line.draw.connect(func() -> void:
		var y := line.size.y * 0.5
		line.draw_line(Vector2(0, y - 1.5), Vector2(line.size.x, y - 1.5), GOLD, 1.0)
		line.draw_line(Vector2(0, y + 1.5), Vector2(line.size.x, y + 1.5), GOLD, 1.0)
		var c := Vector2(line.size.x * 0.5, y)
		line.draw_colored_polygon(PackedVector2Array([c + Vector2(0, -4), c + Vector2(6, 0), c + Vector2(0, 4), c + Vector2(-6, 0)]), RUBRIC))
	return line


static func faction_color(faction_id: String, fallback: Color = Color(0.5, 0.45, 0.35)) -> Color:
	var tree := Engine.get_main_loop() as SceneTree
	var facade: Node = tree.root.get_node_or_null("SimFacade") if tree != null else null
	if facade == null or faction_id == "" or not facade.has_method("faction_color"):
		return fallback
	var color: Color = facade.call("faction_color", faction_id)
	return fallback if color == Color(0.5, 0.5, 0.5) else color


## Verdict en clair d'une chance de victoire (0-1) du camp du joueur.
static func verdict(chance: float) -> String:
	if chance >= 0.95:
		return "Victoire presque certaine"
	if chance >= 0.7:
		return "Victoire probable"
	if chance > 0.3:
		return "Issue incertaine"
	if chance > 0.05:
		return "Défaite probable"
	return "Défaite presque certaine"


static func verdict_color(chance: float) -> Color:
	if chance >= 0.7:
		return GOOD
	if chance > 0.3:
		return Color(0.55, 0.40, 0.08)
	return RUBRIC


## Barre d'équilibre à la TW : part gauche `share` (0-1) en `left`, reste en `right`, curseur
## doré au point d'équilibre, graduation au milieu.
static func draw_balance(canvas: Control, share: float, left: Color, right: Color) -> void:
	var size := canvas.size
	var split := size.x * clampf(share, 0.0, 1.0)
	canvas.draw_rect(Rect2(0, 0, split, size.y), left)
	canvas.draw_rect(Rect2(split, 0, size.x - split, size.y), right)
	# Reflet et ombre pour un relief d'émail.
	canvas.draw_rect(Rect2(0, 0, size.x, size.y * 0.35), Color(1, 1, 1, 0.16))
	canvas.draw_rect(Rect2(0, size.y * 0.7, size.x, size.y * 0.3), Color(0, 0, 0, 0.15))
	canvas.draw_line(Vector2(size.x * 0.5, -3), Vector2(size.x * 0.5, size.y + 3), INK, 1.0)
	canvas.draw_rect(Rect2(Vector2.ZERO, size), INK, false, 2.0)
	var marker := PackedVector2Array([Vector2(split - 7, size.y + 7), Vector2(split + 7, size.y + 7), Vector2(split, size.y - 3)])
	canvas.draw_colored_polygon(marker, GOLD)
	canvas.draw_polyline(marker + PackedVector2Array([marker[0]]), INK, 1.0)


## Texture recadrée à la manière de « cover » (remplit `rect`, centre gardé).
static func draw_cover(canvas: CanvasItem, texture: Texture2D, rect: Rect2, modulate: Color = Color.WHITE, focus_y: float = 0.5) -> void:
	if texture == null:
		return
	var tex := texture.get_size()
	var scale := maxf(rect.size.x / tex.x, rect.size.y / tex.y)
	var src_size := rect.size / scale
	var src := Rect2(Vector2((tex.x - src_size.x) * 0.5, (tex.y - src_size.y) * focus_y), src_size)
	canvas.draw_texture_rect_region(texture, rect, src, modulate)
