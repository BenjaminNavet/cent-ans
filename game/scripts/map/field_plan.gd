class_name FieldPlan
extends RefCounted

## Lot DN-CHAMPS (ADR 0220) : placement des champs en modèles 3D générés sur la carte de campagne.
## Pur calcul, sans nœud, déterministe (hachage entier : le `sin` du shader n'est pas reproductible
## côté CPU). Les données sont celles du sol peint : poids de cultures par biome
## (`ground_biome_mix.json`) et par paysage régional (`agri_landscapes.json` + masque), splat
## (cultures, forêt) ; la table matière -> modèles vit dans `data/art/dn_fields.json`.
## - Régions de culture dominante (Voronoi large) : un tirage commun à 70 % des parcelles, comme le
##   shader (`hb_region_cells`) ; les autres tirent seules.
## - Parcelles de Voronoi à sites jitterés (`parcel_px`) ; seules celles dont le centre est cultivé
##   (part de cultures du splat, hors forêt, hors villes et fleuves) portent des modèles.
## - Modèles en rangées dans le repère de la région (champs parallèles par terroir), jachère de
##   bordure (`headland_px`) entre parcelles ; arbres des vergers et vignes sur le même réseau.
## - Un accent par parcelle au plus (pressoir, pergola), tirage par donnée.
## Sortie de `plan_cell` : id de modèle -> PackedFloat32Array de (x, z, lacet, taille en mètres).

const STRIDE := 4
const SALT_REGION := 11
const SALT_PARCEL := 23

var config: Dictionary = {}
var map_size := Vector2.ONE
var meters_per_px := 718.9765625

var _plan: Dictionary = {}
var _crops: Dictionary = {}
var _mix: Dictionary = {}  # clé de ligne (str) -> {crops, farm}
var _rows: Dictionary = {}  # ligne agricole (int) -> id de paysage
var _splat: Image
var _biomes: Image
var _agri: Image
var _towns := PackedVector2Array()
var _map: MapData


## `mix` : `ground_biome_mix.json` ; `landscapes` : `agri_landscapes.json` ; images optionnelles
## (null : pas de cultures là où elles manquent).
func setup(p_config: Dictionary, p_map: MapData, mix: Dictionary, landscapes: Dictionary, biomes: Image, agri: Image, towns: PackedVector2Array) -> void:
	config = p_config
	_plan = config.get("plan", {})
	_crops = config.get("crops", {})
	_map = p_map
	map_size = Vector2(p_map.size) if p_map != null else Vector2.ONE
	meters_per_px = p_map.meters_per_px if p_map != null else meters_per_px
	_splat = p_map.splat_image if p_map != null else null
	_biomes = biomes
	_agri = agri
	_towns = towns
	_mix = {}
	for key: String in mix.get("biomes", {}):
		_mix[key] = mix["biomes"][key]
	var row := HbGround.FIRST_LANDSCAPE_ROW
	for id: String in landscapes.get("landscapes", {}):
		_rows[row] = id
		_mix[str(row)] = landscapes["landscapes"][id]
		row += 1


## Vrai si le plan peut produire quelque chose (images et table présentes).
func is_ready() -> bool:
	return _splat != null and _biomes != null and not _crops.is_empty()


# --- Hachage ---------------------------------------------------------------------------------


static func hash01(a: int, b: int, salt: int) -> float:
	var h := (a * 374761393 + b * 668265263 + salt * 2147483629) & 0xFFFFFFFF
	h = ((h ^ (h >> 13)) * 1274126177) & 0xFFFFFFFF
	h = (h ^ (h >> 16)) & 0xFFFFFFFF
	h = ((h ^ (h >> 15)) * 2246822519) & 0xFFFFFFFF
	return float((h ^ (h >> 13)) & 0xFFFFFF) / 16777216.0


# --- Lectures de carte -----------------------------------------------------------------------


func _sample(image: Image, px: Vector2) -> Color:
	var x := clampi(int(px.x / map_size.x * image.get_width()), 0, image.get_width() - 1)
	var y := clampi(int(px.y / map_size.y * image.get_height()), 0, image.get_height() - 1)
	return image.get_pixel(x, y)


## Ligne de la table du sol au point : paysage agricole régional s'il y en a un, sinon biome.
func ground_row(px: Vector2) -> int:
	if _agri != null:
		var row := int(round(_sample(_agri, px).r * 255.0))
		if _rows.has(row):
			return row
	if _biomes == null:
		return 0
	var biome := int(round(_sample(_biomes, px).r * 255.0))
	return clampi(biome, 1, 7) if biome > 0 else 0


## Part de cultures du splat au point, avec la conversion prairie -> cultures du paysage régional
## (`open_to_farm`, ME8) ; 0 en forêt dense.
func farm_share(px: Vector2, row: int) -> float:
	var splat := _sample(_splat, px)
	if splat.b > float(_plan.get("forest_max", 0.4)):
		return 0.0
	var entry: Dictionary = _mix.get(str(row), {})
	var share := splat.g + splat.r * float(entry.get("open_to_farm", 0.0))
	return clampf(share * float(entry.get("farm", 1.0)) * float(_plan.get("farm_gain", 1.0)), 0.0, 1.0)


# --- Régions et parcelles --------------------------------------------------------------------


## Région de culture dominante du point : {id: Vector2i, roll, angle}.
func region_at(px: Vector2) -> Dictionary:
	var size := float(_plan.get("region_px", 16.0))
	var u := px / size
	var cell := Vector2i(floori(u.x), floori(u.y))
	var best := Vector2i.ZERO
	var best_d := INF
	for j in range(-1, 2):
		for i in range(-1, 2):
			var id := cell + Vector2i(i, j)
			var site := Vector2(id) + Vector2(0.1 + 0.8 * hash01(id.x, id.y, 3), 0.1 + 0.8 * hash01(id.x, id.y, 5))
			var d := u.distance_to(site) - (hash01(id.x, id.y, 7) - 0.5) * 0.7
			if d < best_d:
				best_d = d
				best = id
	return {"id": best, "roll": hash01(best.x, best.y, SALT_REGION), "angle": hash01(best.x, best.y, SALT_REGION + 1) * PI}


## Site (px carte) de la parcelle de maille `id`.
func parcel_site(id: Vector2i) -> Vector2:
	var size := float(_plan.get("parcel_px", 0.8))
	return (Vector2(id) + Vector2(0.1 + 0.8 * hash01(id.x, id.y, 31), 0.1 + 0.8 * hash01(id.x, id.y, 37))) * size


func _parcel_weight(id: Vector2i) -> float:
	return (hash01(id.x, id.y, 41) - 0.5) * 0.7 * float(_plan.get("parcel_px", 0.8))


## Matière de culture tirée dans les poids de la ligne `row` pour le tirage `roll` (0 à 1) ;
## chaîne vide si la ligne n'a pas de cultures.
func pick_material(row: int, roll: float) -> String:
	var crops: Dictionary = (_mix.get(str(row), {}) as Dictionary).get("crops", {})
	var total := 0.0
	for id: String in crops:
		total += float(crops[id])
	if total <= 0.0:
		return ""
	var target := clampf(roll, 0.0, 0.9999) * total
	var cumulative := 0.0
	for id: String in crops:
		cumulative += float(crops[id])
		if target <= cumulative:
			return id
	return ""


## Tirage de culture de la parcelle `id` : tirage de la région pour `dominant_share` des
## parcelles (écart `dominant_jitter`), tirage propre pour les autres.
func parcel_roll(id: Vector2i, region_roll: float) -> float:
	var own := hash01(id.x, id.y, 43)
	if hash01(id.x, id.y, 47) < float(_plan.get("dominant_share", 0.7)):
		return clampf(region_roll + (own - 0.5) * float(_plan.get("dominant_jitter", 0.25)) * 2.0, 0.0, 1.0)
	return own


## Matière de culture dominante de la région au point (tests, comparaison avec le sol).
func dominant_material(px: Vector2) -> String:
	var row := ground_row(px)
	return pick_material(row, float(region_at(px)["roll"])) if row > 0 else ""


func _near_town(px: Vector2) -> bool:
	var clear := float(_plan.get("town_clear_px", 0.4))
	for town in _towns:
		if absf(town.x - px.x) < clear and absf(town.y - px.y) < clear and town.distance_to(px) < clear:
			return true
	return false


## Parcelle cultivée de maille `id` : {site, material, crop, variant, angle} ; vide sinon.
func parcel(id: Vector2i) -> Dictionary:
	var site := parcel_site(id)
	if site.x < 0.0 or site.y < 0.0 or site.x >= map_size.x or site.y >= map_size.y:
		return {}
	var row := ground_row(site)
	if row <= 0:
		return {}
	if hash01(id.x, id.y, 53) >= farm_share(site, row):
		return {}
	if _map != null and (not _map.is_land_px(int(site.x), int(site.y)) or _map.river_sd_at(site.x, site.y) < float(_plan.get("river_clear_px", 0.12))):
		return {}
	if _near_town(site):
		return {}
	var region := region_at(site)
	var material := pick_material(row, parcel_roll(id, float(region["roll"])))
	if not _crops.has(material):
		return {}
	var crop: Dictionary = _crops[material]
	var angle := float(region["angle"])
	if hash01(id.x, id.y, 59) < float(_plan.get("cross_share", 0.15)):
		angle += PI * 0.5
	return {"site": site, "material": material, "crop": crop, "variant": _pick_variant(crop["variants"], hash01(id.x, id.y, 61)), "angle": angle}


static func _pick_variant(variants: Array, roll: float) -> Dictionary:
	var total := 0.0
	for variant: Dictionary in variants:
		total += float(variant["weight"])
	var target := roll * total
	var cumulative := 0.0
	for variant: Dictionary in variants:
		cumulative += float(variant["weight"])
		if target <= cumulative:
			return variant
	return variants[variants.size() - 1]


# --- Cellules --------------------------------------------------------------------------------


## Instances des parcelles dont le site tombe dans la cellule (`cell_px`) de coordonnées `cell`.
func plan_cell(cell: Vector2i) -> Dictionary:
	var out := {}
	var cell_px := float(config.get("render", {}).get("cell_px", 2.0))
	var size := float(_plan.get("parcel_px", 0.8))
	var low := Vector2(cell) * cell_px
	var first := Vector2i(floori(low.x / size) - 1, floori(low.y / size) - 1)
	var last := Vector2i(floori((low.x + cell_px) / size) + 1, floori((low.y + cell_px) / size) + 1)
	for gy in range(first.y, last.y + 1):
		for gx in range(first.x, last.x + 1):
			var id := Vector2i(gx, gy)
			var site := parcel_site(id)
			if site.x < low.x or site.y < low.y or site.x >= low.x + cell_px or site.y >= low.y + cell_px:
				continue
			var found := parcel(id)
			if not found.is_empty():
				_fill_parcel(id, found, out)
	return out


func _append(out: Dictionary, key: String, x: float, z: float, yaw: float, size_m: float) -> void:
	var buffer: PackedFloat32Array = out.get(key, PackedFloat32Array())
	buffer.append(x)
	buffer.append(z)
	buffer.append(yaw)
	buffer.append(size_m)
	out[key] = buffer


func _fill_parcel(id: Vector2i, found: Dictionary, out: Dictionary) -> void:
	var size := float(_plan.get("parcel_px", 0.8))
	var site: Vector2 = found["site"]
	var crop: Dictionary = found["crop"]
	var variant: Dictionary = found["variant"]
	var angle: float = found["angle"]
	var spacing := float(crop["spacing_m"]) / meters_per_px
	var rows := float(crop["row_m"]) / meters_per_px
	var headland := maxf(float(_plan.get("headland_px", 0.05)), 0.35 * float(variant["size_m"]) / meters_per_px)
	# Sites voisins (poids additifs) : une instance appartient à la parcelle la plus proche.
	var neighbours: Array = []
	for j in range(-1, 2):
		for i in range(-1, 2):
			if i == 0 and j == 0:
				continue
			var other := Vector2i(id.x + i, id.y + j)
			neighbours.append([parcel_site(other), _parcel_weight(other)])
	var own_weight := _parcel_weight(id)
	var rot := Transform2D(angle, Vector2.ZERO)
	var reach := size * 0.95
	var variant_size := float(variant["size_m"])
	var us := int(reach / spacing)
	var vs := int(reach / rows)
	var half := 0.5 * spacing
	for vi in range(-vs, vs + 1):
		var shift := half if (vi & 1) == 1 else 0.0
		for ui in range(-us, us + 1):
			var q := site + rot * Vector2(ui * spacing + shift, vi * rows)
			var own := q.distance_to(site) - own_weight
			var wins := true
			for n: Array in neighbours:
				if q.distance_to(n[0]) - float(n[1]) < own + headland:
					wins = false
					break
			if not wins:
				continue
			# Un seul hachage par instance : jitter, taille et lacet en dérivent.
			var h := hash01(id.x * 131 + ui, id.y * 137 + vi, 67)
			var jitter := Vector2(h - 0.5, fposmod(h * 17.31, 1.0) - 0.5) * 0.3
			q += rot * Vector2(jitter.x * spacing, jitter.y * rows)
			var grow := 0.85 + 0.3 * fposmod(h * 53.7, 1.0)
			_append(out, str(variant["id"]), q.x, q.y, -angle + (fposmod(h * 131.9, 1.0) - 0.5) * 0.12, variant_size * grow)
	# Accent : au plus un par parcelle, au site.
	for accent: Dictionary in crop.get("accents", []):
		if hash01(id.x, id.y, 79 + int(accent["chance"] * 1000.0)) < float(accent["chance"]):
			_append(out, str(accent["id"]), site.x, site.y, hash01(id.x, id.y, 83) * TAU, float(accent["size_m"]))
			break
