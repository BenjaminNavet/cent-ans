class_name HeraldryAtlas
extends RefCounted

## Lot DV2 (ADR 0124) : atlas des écus de faction, composé à l'exécution depuis
## `PortraitLoader.heraldry_texture` (extrait de `SettlementMarkers`, lot DA3). Sert à l'écu posé
## au-dessus du nom des lieux (`settlement_icon.gdshader`). Rendu seulement.

## Taille d'une case de l'atlas (px) et nombre de colonnes.
const CELL := 64
const COLUMNS := 8

var texture: Texture2D
## faction → case de l'atlas (absente : pas d'armoiries).
var index: Dictionary = {}
## Factions demandées au dernier `build` (avec ou sans armoiries) : pas de recomposition pour
## une faction sans écu à chaque rafraîchissement.
var requested: Dictionary = {}
var columns: int = COLUMNS
var rows: int = 1


## Recompose l'atlas des factions données (ordre stable). Une faction sans armoiries n'a pas de
## case (`shield_of` → -1, le shader n'en dessine rien).
func build(factions: Array) -> void:
	index.clear()
	requested.clear()
	var images: Array[Image] = []
	for faction in factions:
		var id := str(faction)
		if id == "" or requested.has(id):
			continue
		requested[id] = true
		var heraldry := PortraitLoader.heraldry_texture(id)
		if heraldry == null:
			continue
		var image := heraldry.get_image()
		if image == null or image.is_empty():
			continue
		image = image.duplicate() as Image
		if image.is_compressed():
			image.decompress()
		image.convert(Image.FORMAT_RGBA8)
		image.resize(CELL, CELL, Image.INTERPOLATE_LANCZOS)
		index[id] = images.size()
		images.append(image)
	rows = maxi((images.size() + columns - 1) / columns, 1)
	var sheet := Image.create(columns * CELL, rows * CELL, true, Image.FORMAT_RGBA8)
	for i in images.size():
		sheet.blit_rect(images[i], Rect2i(0, 0, CELL, CELL), Vector2i((i % columns) * CELL, (i / columns) * CELL))
	sheet.generate_mipmaps()
	texture = ImageTexture.create_from_image(sheet)


## Vrai si la faction a été demandée au dernier `build` (qu'elle ait ou non des armoiries).
func has(faction: String) -> bool:
	return requested.has(faction)


func shield_of(faction: String) -> int:
	return int(index.get(faction, -1))


## Grille de l'atlas (colonnes, rangées) pour le shader.
func grid() -> Vector2:
	return Vector2(columns, rows)


## Rectangle (UV 0-1) d'une case.
func uv_rect(cell: int) -> Rect2:
	var size := Vector2(1.0 / columns, 1.0 / rows)
	return Rect2(Vector2(cell % columns, cell / columns) * size, size)
