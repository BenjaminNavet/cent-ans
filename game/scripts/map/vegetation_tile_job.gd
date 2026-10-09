class_name VegetationTileJob
extends RefCounted

## Requête de semis d'une tuile de végétation, exécutée dans un `WorkerThreadPool`.
## Le semis lui-même (candidats, essences, haies, empaquetage) est fait par
## `VegetationScatter` (crate Rust `vegetation`, ADR 0204) ; ici : grille grossière, paramètres
## de la requête, installation du résultat, dégagements. Description historique du semis :
##
## 1. Grille grossière (`coarse_step` px) des masques (`VegetationMask.sample`).
## 2. Grille de candidats espacés de `spacing` px, décalés aléatoirement (RNG déterministe par
##    tuile) ; chaque candidat devient un feuillu, un conifère, un arbre de bosquet ou un
##    buisson de haie selon les densités interpolées.
## 3. Tampons `MultiMesh` (transform 3×4 + données custom : teinte RVB, graine A) triés par
##    graine décroissante : un préfixe du tampon est un sous-échantillon uniforme, ce qui permet
##    d'éclaircir une tuile lointaine avec `visible_instance_count`.
##
## Aucun accès à l'arbre de scène : sûr hors du fil principal.

## Lot V4 (A1-10) : essences — chêne (chênaies, bocage, arbres des champs), hêtre (hêtraies),
## conifère de montagne (sapin, épicéa), haie.
enum Kind { OAK, BEECH, CONIFER, HEDGE }
const KIND_COUNT := 4
## Lot V4 : au cœur des massifs, houppiers élargis jusqu'à se toucher (canopée continue).
const CANOPY_SPREAD := 0.4
## La tuile est rendue en PARTS_SIDE × PARTS_SIDE parties (culling et LOD plus fins).
const PARTS_SIDE := 2
const PARTS := PARTS_SIDE * PARTS_SIDE
const FLOATS_PER_INSTANCE := 16
## Enfoncement du pied (part de la hauteur) : le tronc ne flotte pas sur une pente.
const GROUND_SINK := 0.08
## Lot V4 : distance minimale (px carte) à la berge d'un fleuve pour planter.
const RIVER_CLEARANCE := 0.3

var mask: VegetationMask
var tile_index: int = 0
var origin_px: Vector2i = Vector2i.ZERO
var size_px: int = 256
var spacing: float = 1.5
var coarse_step: int = 4
var tree_scale: float = 1.0
## Cercles d'exclusion (villes) : Vector3(x, y, rayon) en pixels de carte.
var exclusions: PackedVector3Array = PackedVector3Array()
## Lot C7b : grille de hauteurs du maillage de terrain affiché (`TerrainBuilder.surface_grid`)
## au lancement du semis ; les arbres y sont posés (vide : heightmap 4096 bilinéaire). Le tri
## terre / mer reste fait sur la heightmap : même semis quel que soit le niveau de relief.
var ground_grid: Dictionary = {}
## Lot HC1 (ADR 0161) : dégagements des arbres généralisés (houppiers hors de l'eau et des routes
## principales), appliqués aux tampons après le semis (`apply_clearance`) ; null : aucun.
var clearance: TreeClearance.TileFilter = null
## Lot HC1 : style généralisé — pas de buissons de haie alignés sur la trame du parcellaire (des
## arbres grossis ne peuvent pas dessiner des enclos d'un à deux px) : les emplacements `Kind.HEDGE`
## sont vidés après le semis ; les arbres épars du bocage viennent du rôle « isolé » (`hedge_boost`).
var drop_hedges: bool = false
## Lot HC1 : seuils du bruit des bosquets (grille grossière `_grove`).
var grove_low: float = 0.28
var grove_high: float = 0.42

## Résultats : un tampon et un nombre d'instances par emplacement `part * KIND_COUNT + kind`.
var buffers: Array[PackedFloat32Array] = []
var counts: PackedInt32Array = PackedInt32Array()
var build_ms: float = 0.0

var _forest := PackedFloat32Array()
var _crops := PackedFloat32Array()
var _conifer := PackedFloat32Array()
var _beech := PackedFloat32Array()
var _hedge := PackedFloat32Array()
var _grove := PackedFloat32Array()
var _region := PackedFloat32Array()
## Lot HB4 : biome par cellule grossière (indice, plus proche).
var _biome := PackedFloat32Array()
var _side: int = 0


func run() -> void:
	var t0 := Time.get_ticks_usec()
	var noise := VegetationMask.make_noise(1337)
	var grove_noise := VegetationMask.make_noise(4242)
	grove_noise.frequency = 1.0 / 9.0
	grove_noise.fractal_octaves = 2
	_sample_coarse(noise, grove_noise)
	build_ms = (Time.get_ticks_usec() - t0) / 1000.0


## Lot HC1 : retire les arbres dont le houppier grossi déborde sur l'eau ou une route principale
## (sûr hors du fil principal ; sans effet sans `clearance`).
func apply_clearance() -> void:
	if drop_hedges:
		for part in PARTS:
			var slot := part * KIND_COUNT + Kind.HEDGE
			if slot < buffers.size():
				buffers[slot] = PackedFloat32Array()
				counts[slot] = 0
	if clearance == null:
		return
	clearance.apply(buffers, counts)
	build_ms += clearance.filter_ms


## Lot PB2 : paramètres de `VegetationScatter.request` (après `run` en mode `coarse_only`).
func native_params() -> Dictionary:
	return {
		"tile_index": tile_index, "origin_x": float(origin_px.x), "origin_y": float(origin_px.y),
		"size_px": float(size_px), "spacing": spacing, "coarse_step": float(coarse_step),
		"tree_scale": tree_scale, "vertical_scale": MapData.vertical_scale(), "relief_gain": MapData.relief_gain(),
		"relief_squash": MapData.relief_squash(),
		"side": _side,
		"coarse": [_forest, _crops, _conifer, _beech, _hedge, _grove, _region],
		"exclusions": exclusions, "ground_grid": ground_grid, "biome": _biome,
	}


## Lot PB2 : résultat de `VegetationScatter.poll` (tampons et nombres par emplacement).
func apply_native(result: Dictionary) -> void:
	buffers.clear()
	for buffer: PackedFloat32Array in result["buffers"]:
		buffers.append(buffer)
	counts = result["counts"]
	build_ms += float(result["ms"])


## Lot SZ4b : grilles grossières gardées pour la forêt dense (`ForestDetail`), {} si absentes.
func coarse_params() -> Dictionary:
	if _side < 2:
		return {}
	return {
		"origin_x": float(origin_px.x), "origin_y": float(origin_px.y), "size_px": float(size_px),
		"coarse_step": float(coarse_step), "side": _side,
		"coarse": [_forest, _crops, _conifer, _beech, _hedge, _grove, _region], "biome": _biome,
	}


func instance_total() -> int:
	var total := 0
	for count in counts:
		total += count
	return total


func _sample_coarse(noise: FastNoiseLite, grove_noise: FastNoiseLite) -> void:
	_side = size_px / coarse_step + 2
	var n := _side * _side
	for array in [_forest, _crops, _conifer, _beech, _hedge, _grove, _region, _biome]:
		array.resize(n)
	var k := 0
	for j in _side:
		var y := float(origin_px.y + j * coarse_step)
		for i in _side:
			var x := float(origin_px.x + i * coarse_step)
			var s := mask.sample(x, y, noise)
			_forest[k] = s["forest"]
			_crops[k] = s["crops"]
			_conifer[k] = s["conifer"]
			_beech[k] = s["beech"]
			_hedge[k] = s["hedge"]
			_grove[k] = smoothstep(grove_low, grove_high, grove_noise.get_noise_2d(x, y))
			# Région (frontière des deux trames de parcelles) : variation bien plus lente que le
			# pas de la grille grossière (erreur d'interpolation très inférieure à la zone morte de
			# 0,02 dans le semis des haies (Rust)) → on évite le coût des 4 sinus par candidat de haie.
			_region[k] = VegetationFields.region_value(x, y)
			_biome[k] = float(mask.biome_at(x, y))
			k += 1


## Lot C7b : repose les instances d'un tampon (16 flottants par instance) sur la grille `grid` d'une tuile dont le
## coin est en `origin` (px carte) : même enfoncement que le semis Rust (8 % de la hauteur,
## longueur de la colonne Y de la base). Rend un nouveau tampon ; sûr hors du fil principal.
static func reground(buffer: PackedFloat32Array, grid: Dictionary, origin: Vector2) -> PackedFloat32Array:
	var result := buffer.duplicate()
	if grid.is_empty():
		return result
	var k := 0
	var size := result.size()
	while k < size:
		var height := Vector3(result[k + 1], result[k + 5], result[k + 9]).length()
		var ground := maxf(TerrainBuilder.grid_height(grid, result[k + 3] - origin.x, result[k + 11] - origin.y), 0.0)
		result[k + 7] = ground - GROUND_SINK * height
		k += FLOATS_PER_INSTANCE
	return result

