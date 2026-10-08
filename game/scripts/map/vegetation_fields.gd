class_name VegetationFields
extends RefCounted

## Parcellaire partagé entre le shader de terrain (`terrain.gdshader`, fonction `field_at`) et le
## semis des haies (`VegetationTileJob`), lot V2b. Purement visuel.
##
## - Deux trames de parcelles (orientations différentes) selon une région lente `layout_at` ;
##   la limite entre régions devient un chemin de terre.
## - Trame : grille biaisée (u, v) = ((x + k y) / fu, (y − k x) / fv) d'un repère déformé
##   (`warp`) ; les rangées (v) sont décalées d'une colonne à l'autre (jonctions en T, pas de
##   quadrillage) ; chaque bord d'enclos est planté de haie selon un tirage (`Hash.lcg01`).
## - Le shader inverse la déformation par point fixe : mêmes constantes, même hachage entier
##   (bits de poids faible identiques en 64 bits GDScript et en `uint` GLSL). Toute modification
##   ici doit être reportée dans `terrain.gdshader` et dans le semis natif
##   (`core/crates/vegetation`, lot PB2, ADR 0062).
##
## Occupation du sol par province (`landuse`) : image RGBA8 basse résolution floutée, dans le
## repère de la carte ; R = part de vigne, G = sécheresse (climat méditerranéen, sud),
## B = bocage (densité de haies), A = 1. Calculée depuis `data/provinces/*.json`
## (terrain, climat, ressources, bâtiments, latitude de la capitale).

## Paramètres des deux trames : [k (biais), fu, fv (taille d'enclos, px carte), phase de déformation].
const LAYOUTS: Array = [[0.35, 5.0, 4.2, 0.0], [-0.8, 4.6, 3.8, 2.1]]
const LANDUSE_SIZE := 128
const LANDUSE_SAMPLES := 192


## Fonction de région (lente) : > 0 → trame 1, sinon trame 0.
static func region_value(x: float, y: float) -> float:
	return sin(0.0131 * x + 1.3 * sin(0.0093 * y)) + sin(0.0117 * y + 1.1 * sin(0.0171 * x)) - 0.25


static func layout_at(x: float, y: float) -> int:
	return 1 if region_value(x, y) > 0.0 else 0


## Déplacement doux du repère des parcelles (bords sinueux, enclos de tailles variées).
static func warp(x0: float, y0: float, phase: float) -> Vector2:
	var ox := 1.7 * sin(0.071 * y0 + 1.9 * sin(0.027 * x0) + phase) + 0.55 * sin(0.23 * y0 + 1.3 * sin(0.061 * x0))
	var oy := 1.7 * sin(0.063 * x0 + 1.7 * sin(0.023 * y0) + phase * 1.3) + 0.55 * sin(0.19 * x0 + 1.1 * sin(0.047 * y0))
	return Vector2(ox, oy)


## Point carte du repère (u, v) de la trame `layout`.
static func to_map(layout: int, u: float, v: float) -> Vector2:
	var params: Array = LAYOUTS[layout]
	var k: float = params[0]
	var a: float = u * float(params[1])
	var b: float = v * float(params[2])
	var det := 1.0 + k * k
	var x0 := (a - k * b) / det
	var y0 := (b + k * a) / det
	return Vector2(x0, y0) + warp(x0, y0, params[3])


## (u, v) non déformés d'un point du repère intermédiaire (x0, y0).
static func to_uv(layout: int, x0: float, y0: float) -> Vector2:
	var params: Array = LAYOUTS[layout]
	var k: float = params[0]
	return Vector2((x0 + k * y0) / float(params[1]), (y0 - k * x0) / float(params[2]))


## Décalage de rangée de la colonne `column` (jonctions en T).
static func row_offset(layout: int, column: int) -> float:
	return Hash.lcg01(column * 5023 + 17 + layout * 101)


## Tirage d'un bord à u constant (`line`), segment `segment` = floor(v).
static func roll_u_edge(layout: int, line: int, segment: int) -> float:
	return Hash.lcg01(line * 7919 + segment * 104729 + layout * 7 + 1)


## Tirage d'un bord de rangée (`row` dans le repère décalé de la colonne `column`).
static func roll_v_edge(layout: int, row: int, column: int) -> float:
	return Hash.lcg01(row * 7919 + column * 104729 + layout * 7 + 31)


## Probabilité qu'un bord d'enclos porte une haie (même formule dans le shader).
static func hedge_probability(open_land: float, bocage: float) -> float:
	return open_land * lerpf(0.1, 0.75, bocage)


# --- Occupation du sol ---


## Image d'occupation du sol (voir en-tête), mise en cache dans `data.landuse_image`.
static func landuse(data: MapData) -> Image:
	if data.landuse_image != null:
		return data.landuse_image
	var values := _province_landuse(data)
	var n := LANDUSE_SAMPLES
	var cells: Array[Color] = []
	cells.resize(n * n)
	var known := PackedByteArray()
	known.resize(n * n)
	var step_x := float(data.size.x) / n
	var step_y := float(data.size.y) / n
	for j in n:
		for i in n:
			var index := data.province_index_at((i + 0.5) * step_x, (j + 0.5) * step_y)
			if values.has(index):
				cells[j * n + i] = values[index]
				known[j * n + i] = 1
	# Mer : valeurs des terres voisines (dilatation), pour ne pas diluer les côtes au flou.
	for _pass in 8:
		var next := known.duplicate()
		for j in n:
			for i in n:
				if known[j * n + i] == 1:
					continue
				var sum := Color(0, 0, 0, 0)
				var count := 0
				for o: Vector2i in [Vector2i(1, 0), Vector2i(-1, 0), Vector2i(0, 1), Vector2i(0, -1)]:
					var a := i + o.x
					var b := j + o.y
					if a >= 0 and b >= 0 and a < n and b < n and known[b * n + a] == 1:
						sum += cells[b * n + a]
						count += 1
				if count > 0:
					cells[j * n + i] = sum / count
					next[j * n + i] = 1
		known = next
	var image := Image.create(n, n, false, Image.FORMAT_RGBA8)
	for j in n:
		for i in n:
			var c: Color = cells[j * n + i] if known[j * n + i] == 1 else Color(0, 0, 0, 1)
			c.a = 1.0
			image.set_pixel(i, j, c)
	# Flou : réduction avec moyenne (mipmaps) → transitions douces entre provinces.
	image.resize(LANDUSE_SIZE, LANDUSE_SIZE, Image.INTERPOLATE_TRILINEAR)
	data.landuse_image = image
	return image


static func _province_landuse(data: MapData) -> Dictionary:
	var result := {}
	var provinces_dir := data.map_dir.get_base_dir().path_join("provinces")
	for index in data.provinces:
		var province: Dictionary = data.provinces[index]
		var path := provinces_dir.path_join(str(province.get("id", "")) + ".json")
		if not FileAccess.file_exists(path):
			continue
		var parsed: Variant = DataFile.parse_file(path)
		if not (parsed is Dictionary):
			continue
		result[index] = landuse_of(parsed)
	return result


## Occupation du sol d'une province (données `data/provinces/<id>.json`).
static func landuse_of(province: Dictionary) -> Color:
	var climate := str(province.get("climate", ""))
	var terrain := str(province.get("terrain", ""))
	var resources: Array = province.get("resources", [])
	var buildings: Array = province.get("buildings", [])
	var lat := 46.0
	var geo: Variant = province.get("geo", {})
	if geo is Dictionary and (geo as Dictionary).get("capital_lonlat", []) is Array:
		var lonlat: Array = (geo as Dictionary).get("capital_lonlat", [])
		if lonlat.size() >= 2:
			lat = float(lonlat[1])
	# Vigne : ressource vin, pressoir ; plus rare vers le nord et en montagne.
	var vine := 0.0
	if resources.has("res_wine"):
		vine += 0.45
	if buildings.has("bld_vineyard_press"):
		vine += 0.35
	var climate_factor: float = {"mediterranean": 1.0, "continental": 0.8, "oceanic": 0.75, "mountain": 0.35}.get(climate, 0.6)
	vine *= climate_factor * (1.0 - smoothstep(47.5, 50.5, lat))
	# Sécheresse : climat méditerranéen, intérieur ibérique, gradient de latitude.
	var dry := 0.0
	if climate == "mediterranean":
		dry = 0.85
	elif lat < 43.5:
		dry = 0.6
	dry = maxf(dry, 0.5 * (1.0 - smoothstep(42.5, 45.5, lat)))
	var bocage := 1.0 if terrain == "bocage" else 0.0
	return Color(clampf(vine, 0.0, 1.0), clampf(dry, 0.0, 1.0), bocage, 1.0)
