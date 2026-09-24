class_name TerroirMask
extends RefCounted

## Lot CV1 : masque des terroirs autour des colonies, lu par `terrain.gdshader`
## (`campaign_life.gdshaderinc`). Image RGBA8 `SIZE`² couvrant la carte :
## R = mise en culture (champs à la place des prés et friches), G = vigne (régions viticoles,
## selon l'occupation du sol), B = brûlis (dévastation), A = pâtures.
## Rayon et intensité selon le type de colonie et la population de la province.

const SIZE := 1024

var image: Image
var texture: ImageTexture


## `settlements` : `SettlementData.settlements` ; `province_states` : id → {devastation,
## population} ; `landuse` : image d'occupation du sol (R vigne) ; `map_size` en pixels carte.
func build(settlements: Array, hamlets: Array, province_states: Dictionary, landuse: Image, map_size: Vector2) -> void:
	image = Image.create(SIZE, SIZE, false, Image.FORMAT_RGBA8)
	image.fill(Color(0, 0, 0, 0))
	if texture == null:
		texture = ImageTexture.create_from_image(image)
	else:
		texture.update(image)
