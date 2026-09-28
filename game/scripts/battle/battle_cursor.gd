class_name BattleCursor
extends RefCounted

## CB-M2 : curseur contextuel de bataille. Le cœur dit ce que ferait un clic droit
## (`BattleSim.hover_context` : `move`, `melee`, `ranged`, `ranged_blocked`, `siege`, `forbidden`,
## `none`) ; ce script ne fait que choisir l'image et la poser par `Input.set_custom_mouse_cursor`
## (32 px, point chaud au centre). `none` rend la flèche du système.
##
## Images : `res://assets/ui/cursors/<contexte>.png` (lot CB, dérivées des icônes DA5 par
## `cent-ans assets ink-icons`, section `cursors` de `data/ui/icons_ink.json`) si elles existent ;
## sinon un substitut construit en code à partir des icônes game-icons déjà livrées (encre cernée,
## comme `AttackCursor` de la campagne), ou une forme simple (cercle barré de `forbidden`).

const SIZE := 32
const HOTSPOT := Vector2(16, 16)
const CURSOR_DIR := "res://assets/ui/cursors/"
const CONTEXTS := ["move", "melee", "ranged", "ranged_blocked", "siege", "forbidden"]
const INK := Color(0.12, 0.06, 0.03)
const RED := Color(0.8, 0.1, 0.08)
const GREEN := Color(0.86, 0.8, 0.52)  # parchemin clair : se lit sur l'herbe comme sur la boue
const GREY := Color(0.55, 0.52, 0.48)
## Substituts : contexte -> [icône game-icons, couleur, barré].
const PLACEHOLDERS := {
	"move": ["res://assets/icons/lorc-boot-prints.svg", GREEN, false],
	"melee": ["res://assets/icons/lorc-crossed-swords.svg", RED, false],
	"ranged": ["res://assets/icons/lorc-target-arrows.svg", RED, false],
	"ranged_blocked": ["res://assets/icons/lorc-target-arrows.svg", GREY, true],
	"siege": ["res://assets/icons/delapouite-siege-tower.svg", RED, false],
	"forbidden": ["", RED, true],
}

static var _images: Dictionary = {}  # contexte -> Image (construite une fois)

## Contexte affiché, "" avant le premier appel.
var context := ""
## Nombre de changements d'image (tests : le curseur ne se repose pas à chaque mouvement).
var changes := 0


## Pose le curseur de `new_context` ; sans effet s'il est déjà affiché.
func apply(new_context: String) -> void:
	if not CONTEXTS.has(new_context):
		new_context = "none"
	if new_context == context:
		return
	context = new_context
	changes += 1
	if DisplayServer.get_name() == "headless":
		return
	if new_context == "none":
		Input.set_custom_mouse_cursor(null, Input.CURSOR_ARROW)
	else:
		Input.set_custom_mouse_cursor(image(new_context), Input.CURSOR_ARROW, HOTSPOT)


## Rend la flèche du système (fin de bataille, sortie de la scène).
func reset() -> void:
	apply("none")


## Image 32 px du curseur `name` : fichier DA5 s'il existe, sinon substitut ; null pour `none`.
static func image(name: String) -> Image:
	if not CONTEXTS.has(name):
		return null
	if _images.has(name):
		return _images[name]
	var result: Image = null
	var path := CURSOR_DIR + name + ".png"
	if ResourceLoader.exists(path):
		var texture: Texture2D = load(path)
		if texture != null:
			result = texture.get_image()
			if result.is_compressed():
				result.decompress()
			result.convert(Image.FORMAT_RGBA8)
			if result.get_size() != Vector2i(SIZE, SIZE):
				result.resize(SIZE, SIZE, Image.INTERPOLATE_BILINEAR)
	if result == null:
		result = placeholder(name)
	_images[name] = result
	return result


## Vrai si le curseur `name` vient du pipeline DA5 (sinon substitut).
static func has_final_art(name: String) -> bool:
	return ResourceLoader.exists(CURSOR_DIR + name + ".png")


## Substitut en code : icône cernée d'encre, teinte du contexte, barre rouge si « barré ».
static func placeholder(name: String) -> Image:
	var spec: Array = PLACEHOLDERS.get(name, ["", RED, true])
	var result := Image.create(SIZE, SIZE, false, Image.FORMAT_RGBA8)
	result.fill(Color(0, 0, 0, 0))
	var icon_path := str(spec[0])
	var fill: Color = spec[1]
	var source: Image = null
	if icon_path != "" and ResourceLoader.exists(icon_path):
		var texture: Texture2D = load(icon_path)
		if texture != null:
			source = texture.get_image()
			if source.is_compressed():
				source.decompress()
			source.convert(Image.FORMAT_RGBA8)
			source.resize(SIZE - 6, SIZE - 6, Image.INTERPOLATE_BILINEAR)
	if source != null:
		var inner := SIZE - 6
		for y in SIZE:
			for x in SIZE:
				# Contour : alpha dilaté de 2 px peint en encre, l'icône teintée par-dessus.
				var alpha := 0.0
				for dy in range(-2, 3, 2):
					for dx in range(-2, 3, 2):
						var sx := x - 3 + dx
						var sy := y - 3 + dy
						if sx >= 0 and sy >= 0 and sx < inner and sy < inner:
							alpha = maxf(alpha, source.get_pixel(sx, sy).a)
				var core := 0.0
				if x >= 3 and y >= 3 and x < inner + 3 and y < inner + 3:
					core = source.get_pixel(x - 3, y - 3).a
				var color := INK.lerp(fill, core)
				color.a = alpha
				result.set_pixel(x, y, color)
	if bool(spec[2]):
		_draw_forbidden(result, source == null)
	return result


## Cercle barré (seul pour `forbidden`, barre seule par-dessus une icône sinon).
static func _draw_forbidden(image: Image, ring: bool) -> void:
	var c := Vector2(SIZE, SIZE) * 0.5 - Vector2(0.5, 0.5)
	var radius := SIZE * 0.5 - 3.0
	for y in SIZE:
		for x in SIZE:
			var p := Vector2(x, y) - c
			var d := p.length()
			# Barre de haut-gauche à bas-droite (distance à la diagonale).
			var bar := absf(p.x - p.y) / sqrt(2.0)
			var on_ring := ring and absf(d - radius) <= 2.2
			var on_bar := bar <= 2.2 and d <= radius + 1.0
			var edge := (ring and absf(d - radius) <= 3.4) or (bar <= 3.4 and d <= radius + 2.0)
			if on_ring or on_bar:
				image.set_pixel(x, y, RED)
			elif edge and image.get_pixel(x, y).a < 0.5:
				image.set_pixel(x, y, INK)
