class_name AttackCursor
extends RefCounted

## Curseur « épées croisées » de la carte de campagne : affiché quand l'armée sélectionnée du
## joueur survole une cible qu'un clic droit attaquerait (armée, place). Rouge cerné d'encre
## pour se lire sur le parchemin comme sur le relief.

const ICON := "res://assets/icons/lorc-crossed-swords.svg"
const SIZE := 36
const FILL := Color(0.78, 0.1, 0.08)
const OUTLINE := Color(0.12, 0.06, 0.03)

static var _image: Image = null
static var _shown := false


## Image du curseur (construite une fois) ; null si l'icône manque.
static func image() -> Image:
	if _image != null:
		return _image
	var texture: Texture2D = load(ICON)
	if texture == null:
		return null
	var source := texture.get_image()
	if source.is_compressed():
		source.decompress()
	source.convert(Image.FORMAT_RGBA8)
	source.resize(SIZE - 2, SIZE - 2, Image.INTERPOLATE_BILINEAR)
	var result := Image.create(SIZE, SIZE, false, Image.FORMAT_RGBA8)
	# Contour : l'alpha dilaté d'un pixel, peint en encre, puis la lame en rouge par-dessus.
	for y in SIZE:
		for x in SIZE:
			var alpha := 0.0
			for dy in range(-2, 1):
				for dx in range(-2, 1):
					var sx := x + dx
					var sy := y + dy
					if sx >= 0 and sy >= 0 and sx < SIZE - 2 and sy < SIZE - 2:
						alpha = maxf(alpha, source.get_pixel(sx, sy).a)
			var inner := 0.0
			if x >= 1 and y >= 1 and x < SIZE - 1 and y < SIZE - 1:
				inner = source.get_pixel(x - 1, y - 1).a
			var color := OUTLINE.lerp(FILL, inner)
			color.a = alpha
			result.set_pixel(x, y, color)
	_image = result
	return _image


## Affiche (vrai) ou retire (faux) le curseur d'attaque ; sans effet s'il est déjà dans cet état.
static func show_attack(enabled: bool) -> void:
	if enabled == _shown:
		return
	_shown = enabled
	if DisplayServer.get_name() == "headless":
		return
	if enabled and image() != null:
		Input.set_custom_mouse_cursor(image(), Input.CURSOR_ARROW, Vector2(SIZE, SIZE) * 0.5)
	else:
		Input.set_custom_mouse_cursor(null, Input.CURSOR_ARROW)


static func is_shown() -> bool:
	return _shown
