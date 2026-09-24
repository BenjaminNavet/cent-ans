class_name TerroirMask
extends RefCounted

## Lot CV1 : masque des terroirs autour des colonies, lu par `terrain.gdshader`
## (`campaign_life.gdshaderinc`). Image RGBA8 `SIZE`² couvrant la carte :
## R = mise en culture (champs à la place des prés et friches), G = vigne (régions viticoles,
## selon l'occupation du sol), B = brûlis (dévastation), A = pâtures (couronne autour des champs).
## Rayon et intensité selon le type de colonie et la population de la province (rendu
## seulement : la population et la dévastation viennent de `get_province_state`).

const SIZE := 1024
## Rayon de base du finage (pixels carte) par type de colonie.
const RADIUS := {"city": 13.0, "town": 9.0, "castle": 6.0, "abbey": 7.0, "village": 6.0}
const INTENSITY := {"city": 1.0, "town": 0.9, "castle": 0.6, "abbey": 0.8, "village": 0.75}
const HAMLET_RADIUS := 3.2
## Population de province de référence (échelle 1 du rayon).
const REFERENCE_POPULATION := 60000.0
## Dévastation (%) à partir de laquelle les terres brûlent.
const BURN_THRESHOLD := 8.0

var image: Image
var texture: ImageTexture
var build_ms: int = 0
var _scale: float = 0.25  # pixels masque par pixel carte


## `settlements` : `SettlementData.settlements` ; `hamlets` : `SettlementData.hamlets` ;
## `province_states` : id → {devastation, population} ; `landuse` : image d'occupation du sol
## (R vigne, B bocage) ; `map_size` en pixels carte.
func build(settlements: Array, hamlets: Array, province_states: Dictionary, landuse: Image, map_size: Vector2) -> void:
	var t0 := Time.get_ticks_msec()
	_scale = float(SIZE) / maxf(map_size.x, map_size.y)
	var data := PackedByteArray()
	data.resize(SIZE * SIZE * 4)
	data.fill(0)
	for entry in settlements:
		var kind := str(entry.get("kind", "village"))
		var state: Dictionary = province_states.get(str(entry.get("province", "")), {})
		var population := float(state.get("population", REFERENCE_POPULATION))
		var devastation := float(state.get("devastation", 0.0))
		var growth := clampf(sqrt(population / REFERENCE_POPULATION), 0.6, 1.6)
		var radius: float = RADIUS.get(kind, 6.0) * growth
		var intensity: float = INTENSITY.get(kind, 0.7) * clampf(0.75 + 0.25 * growth, 0.0, 1.0)
		var px: Vector2 = entry["px"]
		var lu := _landuse_at(landuse, px, map_size)
		_paint(data, px, radius, intensity, lu.r, lu.b, devastation)
	for hamlet in hamlets:
		var state_h: Dictionary = province_states.get(str(hamlet.get("province", "")), {})
		var hpx: Vector2 = hamlet["px"]
		var lu_h := _landuse_at(landuse, hpx, map_size)
		_paint(data, hpx, HAMLET_RADIUS, 0.55, lu_h.r, lu_h.b, float(state_h.get("devastation", 0.0)))
	image = Image.create_from_data(SIZE, SIZE, false, Image.FORMAT_RGBA8, data)
	if texture == null:
		texture = ImageTexture.create_from_image(image)
	else:
		texture.update(image)
	build_ms = Time.get_ticks_msec() - t0


static func _landuse_at(landuse: Image, px: Vector2, map_size: Vector2) -> Color:
	if landuse == null:
		return Color(0, 0, 0, 0)
	var x := clampi(int(px.x / map_size.x * landuse.get_width()), 0, landuse.get_width() - 1)
	var y := clampi(int(px.y / map_size.y * landuse.get_height()), 0, landuse.get_height() - 1)
	return landuse.get_pixel(x, y)


## Disque de finage : champs (R) au centre, vigne (G) si région viticole, pâtures (A) en
## couronne, brûlis (B) selon la dévastation. Combinaison par maximum.
func _paint(data: PackedByteArray, px: Vector2, radius: float, intensity: float, vine: float, bocage: float, devastation: float) -> void:
	var center := px * _scale
	var outer := radius * 1.6 * _scale
	var inner := radius * _scale
	var burn := 0.0
	if devastation >= BURN_THRESHOLD:
		burn = clampf(devastation / 100.0, 0.0, 1.0)
	var vine_amount := clampf(vine * 1.8, 0.0, 1.0)
	var pasture := clampf(0.35 + 0.65 * bocage, 0.0, 1.0)
	var x0 := maxi(int(center.x - outer) - 1, 0)
	var x1 := mini(int(center.x + outer) + 1, SIZE - 1)
	var y0 := maxi(int(center.y - outer) - 1, 0)
	var y1 := mini(int(center.y + outer) + 1, SIZE - 1)
	for y in range(y0, y1 + 1):
		for x in range(x0, x1 + 1):
			var d := Vector2(x + 0.5, y + 0.5).distance_to(center)
			if d > outer:
				continue
			var field := intensity * (1.0 - smoothstep(inner * 0.55, inner, d))
			var ring := pasture * intensity * smoothstep(inner * 0.6, inner, d) * (1.0 - smoothstep(inner * 1.25, outer, d))
			var scorched := burn * (1.0 - smoothstep(inner * 0.8, outer, d))
			var offset := (y * SIZE + x) * 4
			data[offset] = maxi(data[offset], int(field * 255.0))
			data[offset + 1] = maxi(data[offset + 1], int(field * vine_amount * 255.0))
			data[offset + 2] = maxi(data[offset + 2], int(scorched * 255.0))
			data[offset + 3] = maxi(data[offset + 3], int(ring * 255.0))


## Valeurs (0-1) du masque à une position carte (tests).
func sample(px: Vector2) -> Color:
	if image == null:
		return Color(0, 0, 0, 0)
	var x := clampi(int(px.x * _scale), 0, SIZE - 1)
	var y := clampi(int(px.y * _scale), 0, SIZE - 1)
	return image.get_pixel(x, y)
