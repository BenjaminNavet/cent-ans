class_name CampaignCursor
extends RefCounted

## DN ui-prod : curseurs 32 px de la carte de campagne (`hand`, `move`, `attack`, `siege`, `embark`,
## `forbidden`), même mécanisme que `BattleCursor` : fichiers `res://assets/ui/cursors_campaign/<nom>.png`
## dérivés des icônes à l'encre par `cent-ans assets ink-icons` (section `cursors_campaign` de
## `data/ui/icons_ink.json`), point chaud au centre. `image` rend null si le fichier manque
## (l'appelant garde son curseur de repli).

const SIZE := 32
const HOTSPOT := Vector2(16, 16)
const CURSOR_DIR := "res://assets/ui/cursors_campaign/"
const CONTEXTS := ["hand", "move", "attack", "siege", "embark", "forbidden"]

static var _images: Dictionary = {}  # nom -> Image (chargée une fois)


## Image du curseur `name`, null si inconnu ou absent.
static func image(name: String) -> Image:
	if not CONTEXTS.has(name):
		return null
	if _images.has(name):
		return _images[name]
	var path := CURSOR_DIR + name + ".png"
	if not ResourceLoader.exists(path):
		return null
	var texture: Texture2D = load(path)
	if texture == null:
		return null
	var result := texture.get_image()
	if result.is_compressed():
		result.decompress()
	result.convert(Image.FORMAT_RGBA8)
	if result.get_size() != Vector2i(SIZE, SIZE):
		result.resize(SIZE, SIZE, Image.INTERPOLATE_BILINEAR)
	_images[name] = result
	return result


## Pose le curseur `name` (flèche du système s'il manque ou si `name` est vide).
static func apply(name: String) -> void:
	if DisplayServer.get_name() == "headless":
		return
	var cursor := image(name) if name != "" else null
	if cursor == null:
		Input.set_custom_mouse_cursor(null, Input.CURSOR_ARROW)
	else:
		Input.set_custom_mouse_cursor(cursor, Input.CURSOR_ARROW, HOTSPOT)
