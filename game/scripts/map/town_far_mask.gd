class_name TownFarMask
extends RefCounted

## Lot VT-C (ADR 0138) : masque des villes 1:1 construites, lu par `town_far.gdshader`
## (`built_mask`). Image R8 de 64 × 64 = 4 096 villes, index i → texel (i % 64, i / 64) ; 1 : la
## ville 1:1 est construite, son maillage lointain s'enfonce près de la caméra. La texture n'est
## renvoyée à la carte graphique que si le masque a changé.
## Rendu seulement.

const SIDE := 64
const CAPACITY := SIDE * SIDE

var _image: Image
var _texture: ImageTexture
var _dirty := true
## Nombre d'envois de la texture à la carte graphique (création comprise) : tests.
var uploads := 0


func _init() -> void:
	_image = Image.create(SIDE, SIDE, false, Image.FORMAT_R8)
	_image.fill(Color(0, 0, 0))


## Marque (ou démarque) la ville `index` comme construite. Index hors bornes ignoré.
func set_built(index: int, built: bool) -> void:
	if index < 0 or index >= CAPACITY:
		return
	var x := index % SIDE
	var y := index / SIDE
	var value := 1.0 if built else 0.0
	if is_equal_approx(_image.get_pixel(x, y).r, value):
		return
	_image.set_pixel(x, y, Color(value, 0, 0))
	_dirty = true


## TB3 : valeur 0-1 de la ville `index` (masque de suie : un second `TownFarMask`).
func set_value(index: int, value: float) -> void:
	if index < 0 or index >= CAPACITY:
		return
	var x := index % SIDE
	var y := index / SIDE
	var stored := roundf(clampf(value, 0.0, 1.0) * 255.0) / 255.0
	if is_equal_approx(_image.get_pixel(x, y).r, stored):
		return
	_image.set_pixel(x, y, Color(stored, 0, 0))
	_dirty = true


func value_of(index: int) -> float:
	if index < 0 or index >= CAPACITY:
		return 0.0
	return _image.get_pixel(index % SIDE, index / SIDE).r


func is_built(index: int) -> bool:
	if index < 0 or index >= CAPACITY:
		return false
	return _image.get_pixel(index % SIDE, index / SIDE).r > 0.5


## Image R8 du masque (lecture seule ; tests et outils).
func image() -> Image:
	return _image


## Texture du masque (créée au premier appel, mise à jour seulement si le masque a changé).
func texture() -> ImageTexture:
	if _texture == null:
		_texture = ImageTexture.create_from_image(_image)
		_dirty = false
		uploads += 1
	elif _dirty:
		_texture.update(_image)
		_dirty = false
		uploads += 1
	return _texture
