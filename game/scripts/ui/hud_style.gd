class_name HudStyle
extends RefCounted

## Palette et petits outils de dessin communs aux composants du HUD de campagne (lot F10a) :
## registre « manuscrit enluminé » — parchemin opaque, encre sépia, rubriques rouges, filets
## d'or sobres, cire à sceller. Les couleurs reprennent `parchment_theme.tres`.
##
## Icônes : si l'autoload `IconLibrary` (lot F2) est présent, `icon(id, category)` lui délègue ;
## sinon `draw_glyph` dessine un pictogramme sobre au trait (aucune police spéciale requise).

const PARCHMENT := Color(0.93, 0.87, 0.72, 1.0)
const PARCHMENT_LIGHT := Color(0.97, 0.93, 0.82, 1.0)
const PARCHMENT_DARK := Color(0.80, 0.70, 0.50, 1.0)
const INK := Color(0.22, 0.14, 0.07, 1.0)
const INK_SOFT := Color(0.42, 0.29, 0.16, 1.0)
const INK_FADED := Color(0.50, 0.44, 0.36, 1.0)
const RUBRIC := Color(0.62, 0.13, 0.08, 1.0)
const GOLD := Color(0.72, 0.56, 0.24, 1.0)
const GOLD_PALE := Color(0.84, 0.72, 0.42, 1.0)
const WAX := Color(0.55, 0.11, 0.08, 1.0)
const WAX_DARK := Color(0.36, 0.06, 0.04, 1.0)
const WAX_LIGHT := Color(0.70, 0.22, 0.14, 1.0)
const WAX_GREEN := Color(0.24, 0.34, 0.18, 1.0)
const SHADOW := Color(0.0, 0.0, 0.0, 0.30)
## Barres d'effectif / de moral : encre verte-de-gris (bon), ocre (moyen), rouge (bas).
const GOOD := Color(0.33, 0.42, 0.20, 1.0)
const FAIR := Color(0.72, 0.52, 0.16, 1.0)
const POOR := Color(0.62, 0.13, 0.08, 1.0)

const FONT_SMALL := 12
const FONT_BODY := 14
const FONT_TITLE := 17


## Kit enluminé (lot UI1, `tools/cent_ans_tools/ui_illumination.py`) : textures 9-slice.
const KIT_DIR := "res://assets/ui/illumination/"
const PAGE_MARGIN := 20
const ILLUMINATED_MARGIN := 46


## Boîte texturée du kit enluminé : `texture_margin` = bordure peinte, centre et bords
## répétés (la texture est périodique), `margin` = marge de contenu supplémentaire.
static func kit_box(texture_name: String, texture_margin: int, margin: int) -> StyleBox:
	var texture := load(KIT_DIR + texture_name + ".png") as Texture2D
	if texture == null:
		return StyleBoxFlat.new()
	var box := StyleBoxTexture.new()
	box.texture = texture
	box.texture_margin_left = texture_margin
	box.texture_margin_top = texture_margin
	box.texture_margin_right = texture_margin
	box.texture_margin_bottom = texture_margin
	box.axis_stretch_horizontal = StyleBoxTexture.AXIS_STRETCH_MODE_TILE_FIT
	box.axis_stretch_vertical = StyleBoxTexture.AXIS_STRETCH_MODE_TILE_FIT
	box.set_content_margin_all(margin)
	return box


## Panneau « page de vélin » : filet d'encre, bande d'or, filet vermillon, bossettes dorées.
## `margin` : marge de contenu (au moins 10 px, largeur du cadre peint).
static func panel_box(margin: int = 10, _radius: int = 3) -> StyleBox:
	return kit_box("panel", PAGE_MARGIN, maxi(margin, 10))


## Grande page enluminée : rinceaux de lierre, baguette azur et gueules, coins dorés.
static func illuminated_box(margin: int = 10) -> StyleBox:
	return kit_box("panel_illuminated", ILLUMINATED_MARGIN, margin + 44)


## Note marginale (bulles, petites fenêtres) : vélin clair, double filet or et vermillon.
static func note_box(margin: int = 8) -> StyleBox:
	return kit_box("tooltip", 12, margin + 3)


## Fond plat (cartes d'unité, lettres) : parchemin, bord fin.
static func card_box(bg: Color = PARCHMENT_LIGHT, border: Color = INK_SOFT, width: int = 1) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.bg_color = bg
	box.border_color = border
	box.set_border_width_all(width)
	box.set_corner_radius_all(2)
	box.set_content_margin_all(4)
	return box


## Couleur d'une jauge 0..1 (effectif, moral, ravitaillement).
static func gauge_color(ratio: float) -> Color:
	if ratio >= 0.66:
		return GOOD
	if ratio >= 0.33:
		return FAIR
	return POOR


## Nombre avec espace fine des milliers (« 139 838 »).
static func thousands(value: int) -> String:
	var text := str(absi(value))
	var out := ""
	while text.length() > 3:
		out = " " + text.substr(text.length() - 3) + out
		text = text.substr(0, text.length() - 3)
	return ("-" if value < 0 else "") + text + out


## Étiquette configurée (police, couleur, taille), sans ombre.
static func label(text: String, size: int = FONT_BODY, color: Color = INK) -> Label:
	var node := Label.new()
	node.text = text
	node.add_theme_font_size_override("font_size", size)
	node.add_theme_color_override("font_color", color)
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return node


## Autoload `IconLibrary` (F2) ou null.
static func icon_library() -> Node:
	var loop := Engine.get_main_loop()
	if loop is SceneTree:
		return (loop as SceneTree).root.get_node_or_null("/root/IconLibrary")
	return null


## Texture d'icône fournie par `IconLibrary` pour `id` exactement, ou null (le composant essaie
## alors son propre repli, puis dessine `draw_glyph`) : le repli générique de catégorie
## d'`IconLibrary` donnerait la même icône à la posture, au ravitaillement et au mouvement.
static func icon(id: String, category: String = "") -> Texture2D:
	var library := icon_library()
	if library == null or not library.has_method("get_icon"):
		return null
	if library.has_method("has_icon") and not bool(library.call("has_icon", id)):
		return null
	var result: Variant = library.call("get_icon", id, category)
	return result as Texture2D if result is Texture2D else null


## Points d'un cercle (polygone) — sceaux, pastilles, découpe circulaire des portraits.
static func circle_points(center: Vector2, radius: float, segments: int = 48) -> PackedVector2Array:
	var points := PackedVector2Array()
	for i in segments:
		var angle := TAU * float(i) / float(segments)
		points.append(center + Vector2(cos(angle), sin(angle)) * radius)
	return points


## Galette de cire : bord légèrement irrégulier (déterministe selon `seed_value`).
static func wax_points(center: Vector2, radius: float, seed_value: int = 7, segments: int = 40) -> PackedVector2Array:
	var points := PackedVector2Array()
	for i in segments:
		var angle := TAU * float(i) / float(segments)
		var wobble := 1.0 + 0.045 * sin(angle * 5.0 + float(seed_value)) + 0.03 * sin(angle * 11.0 + float(seed_value) * 1.7)
		points.append(center + Vector2(cos(angle), sin(angle)) * radius * wobble)
	return points


## Sceau de cire complet : galette, ombre, anneau estampé, champ intérieur.
## Renvoie le rayon du champ intérieur (pour y poser un écu ou un portrait).
static func draw_wax_seal(canvas: CanvasItem, center: Vector2, radius: float, wax: Color = WAX, seed_value: int = 7) -> float:
	canvas.draw_colored_polygon(wax_points(center + Vector2(1.5, 2.5), radius, seed_value), SHADOW)
	canvas.draw_colored_polygon(wax_points(center, radius, seed_value), wax)
	var inner := radius * 0.80
	canvas.draw_arc(center, radius * 0.90, 0.0, TAU, 48, wax.darkened(0.35), maxf(1.0, radius * 0.05), true)
	canvas.draw_arc(center, inner, 0.0, TAU, 48, wax.lightened(0.18), maxf(1.0, radius * 0.03), true)
	return inner


## Écu (texture) posé proportionnellement dans un carré de côté `size` centré sur `center`.
static func draw_texture_fit(canvas: CanvasItem, texture: Texture2D, center: Vector2, size: float, modulate: Color = Color.WHITE) -> void:
	if texture == null:
		return
	var tex_size := texture.get_size()
	var scale := size / maxf(tex_size.x, tex_size.y)
	var draw_size := tex_size * scale
	canvas.draw_texture_rect(texture, Rect2(center - draw_size * 0.5, draw_size), false, modulate)


## Texture découpée en disque (portrait) — polygone texturé, sans shader.
static func draw_texture_disc(canvas: CanvasItem, texture: Texture2D, center: Vector2, radius: float) -> void:
	if texture == null:
		return
	var points := circle_points(center, radius, 64)
	var uvs := PackedVector2Array()
	var tex_size := texture.get_size()
	# Recadrage carré centré (haut du visage privilégié).
	var side := minf(tex_size.x, tex_size.y)
	var origin := Vector2((tex_size.x - side) * 0.5, (tex_size.y - side) * 0.25)
	for point in points:
		var local := (point - center) / (radius * 2.0) + Vector2(0.5, 0.5)
		uvs.append((origin + local * side) / tex_size)
	canvas.draw_colored_polygon(points, Color.WHITE, uvs, texture)


## Pictogrammes de repli dessinés au trait (encre), dans un carré de côté `size`.
## Ids reconnus : classes `cavalry`, `infantry`, `ranged`, `siege` ; alertes `enemy_army`,
## `siege_alert`, `construction_done`, `research_done`, `debt`, `chronicle_decision`,
## `idle_character` ; statut `supply`, `movement`, `stance_normal`, `stance_raid`, `stance_siege`.
## `contrast` : couleur des évidements (fond sur lequel le pictogramme est posé).
static func draw_glyph(canvas: CanvasItem, id: String, center: Vector2, size: float, color: Color = INK, contrast: Color = PARCHMENT_LIGHT) -> void:
	var s := size * 0.5
	var w := maxf(1.5, size * 0.08)
	match id:
		"cavalry":
			# Fer à cheval, ouverture en bas, et ses clous.
			canvas.draw_arc(center + Vector2(0, -s * 0.05), s * 0.62, PI * 0.95, TAU + PI * 0.05, 24, color, w * 1.6, true)
			for i in 3:
				var a := PI + PI * (0.25 + 0.25 * float(i))
				canvas.draw_circle(center + Vector2(0, -s * 0.05) + Vector2(cos(a), sin(a)) * s * 0.62, w * 0.35, contrast)
			canvas.draw_line(center + Vector2(-s * 0.62, -s * 0.05), center + Vector2(-s * 0.5, s * 0.7), color, w * 1.6, true)
			canvas.draw_line(center + Vector2(s * 0.62, -s * 0.05), center + Vector2(s * 0.5, s * 0.7), color, w * 1.6, true)
		"infantry":
			# Écu en pointe et épée verticale derrière.
			canvas.draw_line(center + Vector2(0, -s * 0.95), center + Vector2(0, s * 0.95), color, w, true)
			canvas.draw_line(center + Vector2(-s * 0.35, -s * 0.62), center + Vector2(s * 0.35, -s * 0.62), color, w, true)
			var shield := PackedVector2Array([
				center + Vector2(-s * 0.55, -s * 0.35), center + Vector2(s * 0.55, -s * 0.35),
				center + Vector2(s * 0.52, s * 0.15), center + Vector2(0, s * 0.72),
				center + Vector2(-s * 0.52, s * 0.15)])
			canvas.draw_colored_polygon(shield, color)
		"ranged":
			# Arc tendu, corde et flèche.
			canvas.draw_arc(center + Vector2(-s * 0.35, 0), s * 0.95, -PI * 0.38, PI * 0.38, 20, color, w * 1.4, true)
			var top := center + Vector2(-s * 0.35, 0) + Vector2(cos(-PI * 0.38), sin(-PI * 0.38)) * s * 0.95
			var bottom := center + Vector2(-s * 0.35, 0) + Vector2(cos(PI * 0.38), sin(PI * 0.38)) * s * 0.95
			canvas.draw_line(top, bottom, color, w * 0.6, true)
			canvas.draw_line(center + Vector2(-s * 0.9, 0), center + Vector2(s * 0.95, 0), color, w, true)
			canvas.draw_colored_polygon(PackedVector2Array([
				center + Vector2(s * 0.98, 0), center + Vector2(s * 0.6, -s * 0.2), center + Vector2(s * 0.6, s * 0.2)]), color)
		"siege":
			# Trébuchet : chevalet, verge, contrepoids.
			canvas.draw_line(center + Vector2(-s * 0.6, s * 0.8), center + Vector2(0, -s * 0.1), color, w, true)
			canvas.draw_line(center + Vector2(s * 0.6, s * 0.8), center + Vector2(0, -s * 0.1), color, w, true)
			canvas.draw_line(center + Vector2(-s * 0.8, s * 0.8), center + Vector2(s * 0.8, s * 0.8), color, w, true)
			canvas.draw_line(center + Vector2(-s * 0.35, s * 0.25), center + Vector2(s * 0.9, -s * 0.85), color, w * 1.2, true)
			canvas.draw_rect(Rect2(center + Vector2(-s * 0.62, s * 0.2), Vector2(s * 0.4, s * 0.38)), color)
		"enemy_army":
			# Épées croisées.
			for sign_x in [-1.0, 1.0]:
				var a := center + Vector2(-s * 0.8 * sign_x, s * 0.8)
				var b := center + Vector2(s * 0.8 * sign_x, -s * 0.8)
				canvas.draw_line(a, b, color, w * 1.2, true)
				var guard := a.lerp(b, 0.25)
				var normal := (b - a).normalized().orthogonal() * s * 0.25
				canvas.draw_line(guard - normal, guard + normal, color, w * 1.2, true)
		"siege_alert", "tower":
			# Tour crénelée.
			canvas.draw_rect(Rect2(center + Vector2(-s * 0.5, -s * 0.45), Vector2(s, s * 1.35)), color)
			for i in 3:
				canvas.draw_rect(Rect2(center + Vector2(-s * 0.5 + s * 0.4 * float(i), -s * 0.8), Vector2(s * 0.22, s * 0.4)), color)
			canvas.draw_rect(Rect2(center + Vector2(-s * 0.14, s * 0.4), Vector2(s * 0.28, s * 0.5)), contrast)
		"construction_done":
			# Marteau de maçon.
			canvas.draw_line(center + Vector2(-s * 0.6, s * 0.85), center + Vector2(s * 0.25, -s * 0.2), color, w * 1.4, true)
			var head := PackedVector2Array([
				center + Vector2(-s * 0.15, -s * 0.75), center + Vector2(s * 0.05, -s * 0.95),
				center + Vector2(s * 0.95, -s * 0.05), center + Vector2(s * 0.75, s * 0.15)])
			canvas.draw_colored_polygon(head, color)
		"research_done":
			# Livre ouvert.
			var left := PackedVector2Array([
				center + Vector2(0, -s * 0.45), center + Vector2(-s * 0.9, -s * 0.65),
				center + Vector2(-s * 0.9, s * 0.55), center + Vector2(0, s * 0.75)])
			var right := PackedVector2Array([
				center + Vector2(0, -s * 0.45), center + Vector2(s * 0.9, -s * 0.65),
				center + Vector2(s * 0.9, s * 0.55), center + Vector2(0, s * 0.75)])
			canvas.draw_colored_polygon(left, color)
			canvas.draw_colored_polygon(right, color.lightened(0.15))
			canvas.draw_line(center + Vector2(0, -s * 0.45), center + Vector2(0, s * 0.75), contrast, w * 0.6, true)
		"debt":
			# Denier barré.
			canvas.draw_arc(center, s * 0.72, 0.0, TAU, 28, color, w * 1.3, true)
			canvas.draw_arc(center, s * 0.45, 0.0, TAU, 24, color, w * 0.7, true)
			canvas.draw_line(center + Vector2(-s * 0.85, s * 0.85), center + Vector2(s * 0.85, -s * 0.85), RUBRIC if color != RUBRIC else INK, w * 1.4, true)
		"chronicle_decision":
			# Rouleau : feuille, deux rouleaux, lignes d'écriture.
			canvas.draw_rect(Rect2(center + Vector2(-s * 0.5, -s * 0.6), Vector2(s * 1.0, s * 1.2)), color)
			for y in [-s * 0.78, s * 0.62]:
				canvas.draw_rect(Rect2(center + Vector2(-s * 0.72, y), Vector2(s * 1.44, s * 0.2)), color)
			for i in 3:
				var y := -s * 0.3 + s * 0.28 * float(i)
				canvas.draw_line(center + Vector2(-s * 0.3, y), center + Vector2(s * 0.3, y), contrast, w * 0.7, true)
		"idle_character":
			# Buste.
			canvas.draw_circle(center + Vector2(0, -s * 0.42), s * 0.34, color)
			var shoulders := PackedVector2Array()
			for i in 13:
				var a := PI + PI * float(i) / 12.0
				shoulders.append(center + Vector2(0, s * 0.85) + Vector2(cos(a) * s * 0.75, sin(a) * s * 0.75))
			canvas.draw_colored_polygon(shoulders, color)
		"supply":
			# Gerbe de blé (ravitaillement).
			for dx in [-0.3, 0.0, 0.3]:
				canvas.draw_line(center + Vector2(s * dx * 0.3, s * 0.85), center + Vector2(s * dx, -s * 0.5), color, w, true)
				canvas.draw_circle(center + Vector2(s * dx, -s * 0.62), s * 0.18, color)
			canvas.draw_line(center + Vector2(-s * 0.35, s * 0.25), center + Vector2(s * 0.35, s * 0.25), color, w * 1.2, true)
		"movement":
			# Éperon : molette et branche.
			canvas.draw_line(center + Vector2(-s * 0.85, s * 0.1), center + Vector2(s * 0.2, s * 0.1), color, w * 1.3, true)
			canvas.draw_arc(center + Vector2(-s * 0.85, -s * 0.2), s * 0.3, PI * 0.5, PI * 1.5, 10, color, w, true)
			for i in 8:
				var a := TAU * float(i) / 8.0
				canvas.draw_line(center + Vector2(s * 0.5, s * 0.1), center + Vector2(s * 0.5, s * 0.1) + Vector2(cos(a), sin(a)) * s * 0.42, color, w * 0.7, true)
		"stance_normal":
			# Bannière carrée sur hampe.
			canvas.draw_line(center + Vector2(-s * 0.6, -s * 0.9), center + Vector2(-s * 0.6, s * 0.9), color, w, true)
			canvas.draw_rect(Rect2(center + Vector2(-s * 0.55, -s * 0.85), Vector2(s * 1.3, s * 0.95)), color)
		"stance_raid":
			# Flamme de chevauchée.
			var flame := PackedVector2Array([
				center + Vector2(0, -s * 0.95), center + Vector2(s * 0.55, -s * 0.1),
				center + Vector2(s * 0.45, s * 0.6), center + Vector2(0, s * 0.9),
				center + Vector2(-s * 0.45, s * 0.6), center + Vector2(-s * 0.55, 0.0),
				center + Vector2(-s * 0.2, -s * 0.3)])
			canvas.draw_colored_polygon(flame, color)
		"stance_siege":
			draw_glyph(canvas, "siege_alert", center, size, color, contrast)
		_:
			canvas.draw_circle(center, s * 0.35, color)
