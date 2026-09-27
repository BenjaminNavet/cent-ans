class_name StanceBadge
extends RefCounted

## Lot CV3-4 : posture visible sur le marqueur d'armée — pastille parchemin à l'icône à l'encre
## de la posture (DA5) au sommet de la hampe (enfant du fleuron, elle suit le porte-étendard),
## absente en posture normale ; en embuscade, l'armée du joueur est dessinée semi-transparente
## (l'ennemi ne la voit pas : la vision du cœur la cache). Appelé en fin de `ArmyMarker.setup`.

const NODE_NAME := "StanceBadge"
## Transparence des figurines et de l'étendard d'une armée du joueur en embuscade.
const AMBUSH_TRANSPARENCY := 0.55
const TEXTURE_SIZE := 64
## Largeur de la pastille dans l'espace du marqueur (le drapeau fait 3,4).
const WORLD_WIDTH := 2.4

static var _textures: Dictionary = {}  # posture → ImageTexture


static func apply(marker: Node3D, stance: String, is_player: bool) -> void:
	var finial := marker.get_node_or_null("Finial") as Node3D
	if finial == null:
		return
	var badge := finial.get_node_or_null(NODE_NAME) as Sprite3D
	var texture := texture_for(stance) if stance != "normal" and stance != "" else null
	if texture == null:
		if badge != null:
			badge.queue_free()
	else:
		if badge == null:
			badge = Sprite3D.new()
			badge.name = NODE_NAME
			badge.billboard = BaseMaterial3D.BILLBOARD_ENABLED
			badge.shaded = false
			badge.double_sided = true
			badge.texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS
			badge.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
			badge.layers = 2
			badge.render_priority = 2
			badge.pixel_size = WORLD_WIDTH / float(TEXTURE_SIZE)
			badge.position = Vector3(0.0, 1.5, 0.0)
			finial.add_child(badge)
		badge.texture = texture
		badge.set_meta("stance", stance)
	var ghost := AMBUSH_TRANSPARENCY if stance == "ambush" and is_player else 0.0
	marker.set_meta("stance_transparency", ghost)
	for node in marker.find_children("*", "GeometryInstance3D", true, false):
		if node.name == NODE_NAME:
			continue
		(node as GeometryInstance3D).transparency = ghost


## Pastille de la posture : disque parchemin cerclé d'or, icône à l'encre au centre ; null si
## l'icône manque (pas de bibliothèque d'icônes, posture inconnue).
static func texture_for(stance: String) -> Texture2D:
	if _textures.has(stance):
		return _textures[stance]
	var icon := HudStyle.icon("hud_stance_" + stance, "hud")
	if icon == null:
		return null
	var glyph := icon.get_image()
	if glyph == null:
		return null
	glyph = glyph.duplicate()
	if glyph.is_compressed():
		glyph.decompress()
	glyph.convert(Image.FORMAT_RGBA8)
	var inner := int(TEXTURE_SIZE * 0.66)
	glyph.resize(inner, inner, Image.INTERPOLATE_LANCZOS)
	var image := Image.create(TEXTURE_SIZE, TEXTURE_SIZE, false, Image.FORMAT_RGBA8)
	var center := Vector2(TEXTURE_SIZE - 1, TEXTURE_SIZE - 1) * 0.5
	var radius := TEXTURE_SIZE * 0.5 - 1.0
	for y in TEXTURE_SIZE:
		for x in TEXTURE_SIZE:
			var r := Vector2(x, y).distance_to(center)
			var edge := clampf(radius - r, 0.0, 1.0)
			var color := HudStyle.PARCHMENT_LIGHT if r < radius - 4.0 else HudStyle.GOLD
			image.set_pixel(x, y, Color(color, edge))
	var offset := (TEXTURE_SIZE - inner) / 2
	image.blend_rect(glyph, Rect2i(0, 0, inner, inner), Vector2i(offset, offset))
	image.generate_mipmaps()
	_textures[stance] = ImageTexture.create_from_image(image)
	return _textures[stance]
