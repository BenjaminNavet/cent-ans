class_name PortraitFrame
extends Control

## DA2 (ADR 0063) : portrait vivant encadré. Image choisie par `LivingPortrait` (portrait fixe,
## variante âgée ou archétype) ; marques procédurales sans nouvelle génération :
## - cadre selon le rang (or et azur pour un souverain, or pour un grand noble, encre et filet d'or
##   pour un chevalier, or et pourpre pour un prélat, encre simple pour un bourgeois) ;
## - couronne ou mitre si l'image n'en porte pas ; écu de la faction sur un archétype ;
## - grisaille (défunt), pâleur (maladif), bandeau de sable (deuil), barreaux (captif), trait de
##   gueules (blessé).
## Sans image : écu de la faction, puis initiales (ultime repli). Rendu seulement.

const SHADER := preload("res://shaders/portrait_marks.gdshader")
# Teintes héraldiques canoniques (bible DA § 3.1, `heraldry.py` TINCTURES).
const OR := Color("#F2C230")
const AZUR := Color("#1F3A93")
const GUEULES := Color("#B0182B")
const SABLE := Color("#1A1A1A")
const POURPRE := Color("#6B2A7A")
const ARGENT := Color("#F5F1E6")

var character: Dictionary = {}
var resolved: Dictionary = {}
var marks: Dictionary = {}
var faction_id := ""
## Écu de la faction en bas à droite d'un archétype (faux si l'hôte l'affiche déjà).
var show_arms := true

var _picture: TextureRect
var _overlay: Control
var _initials: Label
var _arms: Texture2D
var _material: ShaderMaterial


func _init() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	clip_contents = false
	_picture = TextureRect.new()
	_picture.name = "Picture"
	_picture.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	_picture.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	_picture.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_picture.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(_picture)
	_initials = Label.new()
	_initials.name = "Initials"
	_initials.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_initials.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_initials.add_theme_color_override("font_color", Color(1, 1, 1))
	_initials.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_initials.visible = false
	add_child(_initials)
	_overlay = Control.new()
	_overlay.name = "Marks"
	_overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_overlay.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	_overlay.draw.connect(_draw_marks)
	add_child(_overlay)
	resized.connect(_on_resized)


## Affiche `data` (dictionnaire `get_character` ou nœud de `get_family_tree`). `context` : contexte
## de rang (`LivingPortrait.context_for` si vide). `lookup` : id → dictionnaire (deuil). Renvoie
## vrai si une image (portrait ou écu) est posée.
func show_character(data: Dictionary, faction: String = "", context: Dictionary = {}, lookup: Callable = Callable()) -> bool:
	character = data
	faction_id = faction if faction != "" else str(data.get("faction", ""))
	if context.is_empty():
		context = LivingPortrait.context_for(data)
	resolved = LivingPortrait.resolve(data, context)
	marks = LivingPortrait.marks_for(data, resolved, lookup)
	var path := str(resolved.get("path", ""))
	var texture: Texture2D = PortraitLoader.load_texture(path) if path != "" else null
	_arms = PortraitLoader.house_heraldry_texture(str(character.get("house", "")), faction_id)  # DA1 : armes de la maison
	var has_portrait := texture != null
	if texture == null:
		texture = _arms
		_picture.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	else:
		_picture.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	_picture.texture = texture
	# Visages types : miroir décidé par l'identifiant, deux frères de la même case ne sont plus
	# des clones (les archétypes ne portent ni armoiries ni texte).
	_picture.flip_h = str(resolved.get("kind", "")) == "archetype" \
			and (hash(str(data.get("id", ""))) >> 7) & 1 == 1
	_picture.material = _marks_material() if has_portrait else null
	_initials.text = initials(str(data.get("name", "?")))
	_initials.visible = texture == null
	_on_resized()
	var text := LivingPortrait.marks_text(marks, str(data.get("sex", "")) == "female")
	tooltip_text = text
	_overlay.queue_redraw()
	return texture != null


func has_portrait() -> bool:
	return str(resolved.get("path", "")) != ""


func _marks_material() -> ShaderMaterial:
	var grey := 1.0 if marks.get("dead", false) else 0.0
	var pallor := 0.8 if marks.get("sick", false) else 0.0
	var dim := 1.0 if marks.get("captive", false) else 0.0
	if grey == 0.0 and pallor == 0.0 and dim == 0.0:
		return null
	if _material == null:
		_material = ShaderMaterial.new()
		_material.shader = SHADER
	_material.set_shader_parameter("grey", grey)
	_material.set_shader_parameter("pallor", pallor)
	_material.set_shader_parameter("dim", dim)
	return _material


func _on_resized() -> void:
	var inset := _frame_width()
	_picture.offset_left = inset
	_picture.offset_top = inset
	_picture.offset_right = -inset
	_picture.offset_bottom = -inset
	_initials.add_theme_font_size_override("font_size", maxi(10, int(size.y * 0.36)))
	_overlay.queue_redraw()


func _unit() -> float:
	return maxf(size.x, 24.0) / 64.0


func _frame_width() -> float:
	if resolved.is_empty():
		return 0.0
	return roundf(clampf(3.0 * _unit(), 1.0, 8.0))


func frame_style() -> String:
	var rank := str(resolved.get("rank", ""))
	for entry in LivingPortrait.config().get("ranks", []):
		if str(entry["id"]) == rank:
			return str(entry.get("frame", "noble"))
	return "noble"


## Couleurs du cadre (extérieur, filet intérieur) par rang ; encre éteinte pour un défunt.
func frame_colors() -> Array:
	if marks.get("dead", false):
		return [HudStyle.INK_SOFT, Color(0.55, 0.52, 0.48)]
	match frame_style():
		"sovereign":
			return [OR, AZUR]
		"prelate":
			return [OR, POURPRE]
		"knight":
			return [HudStyle.INK, OR]
		"burgher":
			return [HudStyle.INK, HudStyle.INK_SOFT]
		_:
			return [OR, HudStyle.INK]


func _draw_marks() -> void:
	if resolved.is_empty():
		return
	var rect := Rect2(Vector2.ZERO, size)
	var u := _unit()
	var width := _frame_width()
	var inner := rect.grow(-width)
	# Barreaux de captivité (fer) sur l'image.
	if marks.get("captive", false):
		var bars := 4
		for i in bars:
			var x := inner.position.x + inner.size.x * (i + 0.5) / bars
			_overlay.draw_line(Vector2(x, inner.position.y), Vector2(x, inner.end.y), Color(SABLE, 0.78), maxf(1.5, 2.6 * u))
		_overlay.draw_line(Vector2(inner.position.x, inner.position.y + inner.size.y * 0.3), Vector2(inner.end.x, inner.position.y + inner.size.y * 0.3), Color(SABLE, 0.6), maxf(1.0, 1.8 * u))
	# Blessure : trait de gueules en bas à gauche, bandé d'argent.
	if marks.get("wounded", false):
		var a := inner.position + Vector2(inner.size.x * 0.10, inner.size.y * 0.78)
		var b := a + Vector2(inner.size.x * 0.20, -inner.size.y * 0.12)
		_overlay.draw_line(a, b, ARGENT, maxf(2.0, 4.0 * u))
		_overlay.draw_line(a, b, GUEULES, maxf(1.0, 1.6 * u))
	# Écu de la faction sur un archétype (l'image n'a pas d'armoiries).
	if show_arms and _arms != null and str(resolved.get("kind", "")) == "archetype" and size.x >= 40.0:
		var side := inner.size.x * 0.28
		var arms_rect := Rect2(inner.end - Vector2(side, side * 1.15) - Vector2(2, 2) * u, Vector2(side, side * 1.15))
		_overlay.draw_texture_rect(_arms, arms_rect, false)
	# Cadre : filet extérieur (couleur du rang), filet intérieur, fin trait d'encre.
	var colors := frame_colors()
	if width > 0.0:
		_overlay.draw_rect(rect.grow(-width * 0.5), colors[0], false, width)
		_overlay.draw_rect(inner.grow(-maxf(1.0, u) * 0.5), colors[1], false, maxf(1.0, 1.2 * u))
		_overlay.draw_rect(rect.grow(-0.5), HudStyle.INK, false, 1.0)
		if frame_style() == "sovereign" and not marks.get("dead", false):
			for corner in [rect.position, Vector2(rect.end.x, rect.position.y), rect.end, Vector2(rect.position.x, rect.end.y)]:
				_overlay.draw_circle(corner + (rect.get_center() - corner).normalized() * width * 0.7, width * 0.7, AZUR)
				_overlay.draw_circle(corner + (rect.get_center() - corner).normalized() * width * 0.7, width * 0.35, OR)
	# Deuil : bandeau de sable en travers du coin haut gauche.
	if marks.get("mourning", false):
		var s := inner.size.x * 0.30
		var o := inner.position
		_overlay.draw_colored_polygon(PackedVector2Array([o + Vector2(s * 0.55, 0), o + Vector2(s, 0), o + Vector2(0, s), o + Vector2(0, s * 0.55)]), Color(SABLE, 0.92))
	if marks.get("crown", false):
		_draw_crown(Vector2(size.x * 0.5, 0.0), clampf(size.x * 0.32, 10.0, 40.0))
	elif marks.get("mitre", false):
		_draw_mitre(Vector2(size.x * 0.5, 0.0), clampf(size.x * 0.22, 8.0, 30.0))


## Couronne d'or à trois fleurons, posée à cheval sur le haut du cadre.
func _draw_crown(base_center: Vector2, crown_width: float) -> void:
	var half := crown_width * 0.5
	var h := crown_width * 0.6
	var base := base_center + Vector2(0, h * 0.35)
	var points := PackedVector2Array([
		base + Vector2(-half, 0), base + Vector2(-half, -h * 0.7), base + Vector2(-half * 0.5, -h * 0.35),
		base + Vector2(0, -h), base + Vector2(half * 0.5, -h * 0.35), base + Vector2(half, -h * 0.7),
		base + Vector2(half, 0)])
	_overlay.draw_colored_polygon(points, OR)
	var outline := points.duplicate()
	outline.append(points[0])
	_overlay.draw_polyline(outline, HudStyle.INK, maxf(1.0, crown_width * 0.07), true)
	for tip in [points[1], points[3], points[5]]:
		_overlay.draw_circle(tip, crown_width * 0.08, GUEULES)


## Mitre d'argent orfrayée d'or.
func _draw_mitre(base_center: Vector2, mitre_width: float) -> void:
	var half := mitre_width * 0.5
	var h := mitre_width * 1.1
	var base := base_center + Vector2(0, h * 0.3)
	var points := PackedVector2Array([base + Vector2(-half, 0), base + Vector2(-half * 0.8, -h * 0.6),
		base + Vector2(0, -h), base + Vector2(half * 0.8, -h * 0.6), base + Vector2(half, 0)])
	_overlay.draw_colored_polygon(points, ARGENT)
	_overlay.draw_line(base + Vector2(0, 0), base + Vector2(0, -h), OR, maxf(1.0, mitre_width * 0.14))
	_overlay.draw_line(base + Vector2(-half, -h * 0.12), base + Vector2(half, -h * 0.12), OR, maxf(1.0, mitre_width * 0.14))
	var outline := points.duplicate()
	outline.append(points[0])
	_overlay.draw_polyline(outline, HudStyle.INK, maxf(1.0, mitre_width * 0.08), true)


static func initials(name: String) -> String:
	var text := ""
	for part in name.split(" ", false):
		if part.length() > 0 and text.length() < 2 and part[0] == part[0].to_upper():
			text += part.substr(0, 1).to_upper()
	return text if text != "" else "?"
