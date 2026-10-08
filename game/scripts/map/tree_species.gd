class_name TreeSpecies
extends RefCounted

## Lot HB4 (ADR 0143) : essences d'arbres de la carte de campagne et leur répartition par biome.
##
## Table compilée `data/art/tree_species.json` (source `data/art/tree_species.yaml`, chaîne
## `tools/blender_scripts/ga3_vegetation_l2.py species`) : une essence = une ligne de l'atlas
## d'imposteurs GA3 (ordre du catalogue, lignes 0-2 : chêne, hêtre, sapin), un maillage de repli
## (`VegetationTileJob.Kind`), une classe de teinte saisonnière, des poids par biome et par rôle.
## Les biomes viennent de `data/map/biomes.png` (`VegetationMask.biome_at`, indices figés
## 1 océanique … 7 semi-aride).
##
## Tirage d'une essence (`pick`, miroir exact de `vegetation::Species::pick` côté Rust) :
## poids = base[biome][rôle] × fenêtre d'altitude × (1 + affinité fleuve × proximité) × part de
## résineux (`forest_kind.png`, pondérée par `conifer_raster`). Tables aplaties (`table()`) pour
## le semis natif (`VegetationScatter.set_species`) et le semis GDScript. Lecture seule après
## chargement : sûr hors du fil principal. Rendu seulement, aucune règle de jeu.

const DATA_FILE := "art/tree_species.json"
## Rôles (ordre figé, partagé avec le Rust).
enum Role { MASSIF, LISIERE, ISOLE, VERGER, RIPISYLVE, GARRIGUE }
const ROLE_NAMES: Array[String] = ["massif", "lisiere", "isole", "verger", "ripisylve", "garrigue"]
const ROLE_COUNT := 6
## Indices de biomes 0..7 (0 = mer).
const BIOME_COUNT := 8
## Paramètres de semis d'un biome, dans cet ordre (`biome_params`).
const BIOME_KEYS: Array[String] = ["forest", "isolated", "grove", "orchard", "orchard_ring_px", "orchard_open", "riparian", "riparian_px", "scrub"]
const BIOME_STRIDE := 9
const MESH_KINDS := {"oak": 0, "beech": 1, "conifer": 2}
## Encodage dans la donnée d'instance (`INSTANCE_CUSTOM`) : r = teinte + 4 × (ligne + 1),
## g = teinte + 4 × (classe de saison + 1) ; décodé par `foliage_common.gdshaderinc`.
const CUSTOM_STRIDE := 4.0
## Distance au lit (px) en deçà de laquelle on ne plante pas (`VegetationTileJob.RIVER_CLEARANCE`).
const RIVER_CLEARANCE := 0.3

static var _cached: TreeSpecies = null

var ok: bool = false
var ids: PackedStringArray = PackedStringArray()
var count: int = 0
## base[(biome * ROLE_COUNT + role) * count + s] = poids biome × poids rôle.
var base := PackedFloat32Array()
var alt_lo := PackedFloat32Array()
var alt_hi := PackedFloat32Array()
var river := PackedFloat32Array()
var conifer := PackedFloat32Array()  # 1 résineux (maillage conifer), 0 feuillu
var kind := PackedInt32Array()  # VegetationTileJob.Kind de repli
var season := PackedInt32Array()
var height := PackedFloat32Array()  # 2 × count (min, max)
var width := PackedFloat32Array()  # 2 × count (ratio à la hauteur, min, max)
var biome_params := PackedFloat32Array()  # BIOME_COUNT × BIOME_STRIDE
var dist: Dictionary = {}


## Table partagée (chargée une fois), `ok` faux si le fichier manque ou est invalide.
static func shared() -> TreeSpecies:
	if _cached == null:
		_cached = TreeSpecies.new()
		_cached.load_file(data_path())
	return _cached


## `data/art/tree_species.json` : dossier de données du jeu (`MapPaths.data_dir`), puis `data/`
## du dépôt ; "" si absent.
static func data_path() -> String:
	return DataFile.path_of(DATA_FILE) if DataFile.exists(DATA_FILE) else ""


## Tests : oublie la table partagée (rechargée au prochain `shared`).
static func reset_shared() -> void:
	_cached = null


func load_file(path: String) -> bool:
	ok = false
	if path == "" or not FileAccess.file_exists(path):
		return false
	var parsed: Variant = DataFile.parse_file(path)
	return parsed is Dictionary and load_dict(parsed)


func load_dict(data: Dictionary) -> bool:
	ok = false
	var list: Array = data.get("species", [])
	dist = data.get("distribution", {})
	count = list.size()
	if count < 3 or dist.is_empty():
		return false
	ids.resize(count)
	base.resize(BIOME_COUNT * ROLE_COUNT * count)
	base.fill(0.0)
	for array: Variant in [alt_lo, alt_hi, river, conifer]:
		array.resize(count)
	kind.resize(count)
	season.resize(count)
	height.resize(2 * count)
	width.resize(2 * count)
	for s in count:
		var sp: Dictionary = list[s]
		ids[s] = str(sp.get("id", ""))
		var alt: Array = sp.get("altitude_m", [0.0, 9000.0])
		alt_lo[s] = float(alt[0])
		alt_hi[s] = float(alt[1])
		river[s] = float(sp.get("river", 0.0))
		kind[s] = int(MESH_KINDS.get(str(sp.get("mesh", "oak")), 0))
		conifer[s] = 1.0 if kind[s] == 2 else 0.0
		season[s] = int(sp.get("season_class", 0))
		var h: Array = sp.get("height", [1.1, 1.7])
		var w: Array = sp.get("width", [1.0, 1.1])
		height[2 * s] = float(h[0])
		height[2 * s + 1] = float(h[1])
		width[2 * s] = float(w[0])
		width[2 * s + 1] = float(w[1])
		var biomes: Dictionary = sp.get("biomes", {})
		var roles: Dictionary = sp.get("roles", {})
		for b in BIOME_COUNT:
			var wb := float(biomes.get(str(b), 0.0))
			if wb <= 0.0:
				continue
			for r in ROLE_COUNT:
				base[(b * ROLE_COUNT + r) * count + s] = wb * float(roles.get(ROLE_NAMES[r], 0.0))
	biome_params.resize(BIOME_COUNT * BIOME_STRIDE)
	biome_params.fill(0.0)
	var biomes_cfg: Dictionary = data.get("biomes", {})
	for b in range(1, BIOME_COUNT):
		var cfg: Dictionary = biomes_cfg.get(str(b), {})
		for k in BIOME_STRIDE:
			biome_params[b * BIOME_STRIDE + k] = float(cfg.get(BIOME_KEYS[k], 0.0))
	ok = true
	return true


func d(key: String, fallback: float = 0.0) -> float:
	return float(dist.get(key, fallback))


## Paramètre `key` (voir `BIOME_KEYS`) du biome `b`.
func biome_param(b: int, key: int) -> float:
	return biome_params[clampi(b, 0, BIOME_COUNT - 1) * BIOME_STRIDE + key]


## Plus grande largeur d'anneau de vergers (px) : clairières à prendre en compte autour d'une tuile.
func max_orchard_ring() -> float:
	var ring := 0.0
	for b in range(1, BIOME_COUNT):
		ring = maxf(ring, biome_param(b, 4))
	return ring


## Tables aplaties pour `VegetationScatter.set_species` (Rust).
func table() -> Dictionary:
	return {
		"count": count, "base": base, "alt_lo": alt_lo, "alt_hi": alt_hi, "river": river,
		"conifer": conifer, "kind": kind, "season": season, "height": height, "width": width,
		"biome_params": biome_params, "distribution": dist,
	}


## Essence tirée pour un candidat (-1 : aucune ne convient). `roll` ∈ [0, 1) ; `river_sd` :
## distance signée au lit (px) ; `conifer_share` : part de résineux du lieu (0..1).
func pick(role: int, biome: int, altitude_m: float, river_sd: float, conifer_share: float, roll: float) -> int:
	if not ok:
		return -1
	var b := clampi(biome, 0, BIOME_COUNT - 1)
	var offset := (b * ROLE_COUNT + role) * count
	var fade := maxf(d("altitude_fade_m", 150.0), 1.0)
	var reach := maxf(d("river_reach_px", 3.0), 0.01)
	var near := 1.0 - smoothstep(RIVER_CLEARANCE, RIVER_CLEARANCE + reach, river_sd)
	var k := d("conifer_raster", 0.0)
	var weights := PackedFloat32Array()
	weights.resize(count)
	var total := 0.0
	for s in count:
		var w := base[offset + s]
		if w <= 0.0:
			continue
		var out := maxf(maxf(alt_lo[s] - altitude_m, altitude_m - alt_hi[s]), 0.0)
		w *= clampf(1.0 - out / fade, 0.0, 1.0)
		w *= maxf(1.0 + river[s] * near, 0.0)
		var share := conifer_share if conifer[s] > 0.5 else 1.0 - conifer_share
		w *= lerpf(1.0, 0.25 + 1.5 * share, k)
		weights[s] = w
		total += w
	if total <= 0.0:
		return -1
	var target := roll * total
	var acc := 0.0
	var last := -1
	for s in count:
		if weights[s] <= 0.0:
			continue
		acc += weights[s]
		last = s
		if target < acc:
			return s
	return last


## Tirage « par peuplement » (cellules de `stand_px`) : même valeur pour tous les arbres d'une
## cellule (`VegetationFields.hash01`, identique au Rust).
func stand_roll(x: float, y: float) -> float:
	var side := maxf(d("stand_px", 3.0), 0.1)
	var ix := floori(x / side)
	var iy := floori(y / side)
	return VegetationFields.hash01(ix * 7919 + iy * 104729 + 17)


## Parcelle de verger : tirage haché de la parcelle (cellules de `orchard_parcel_px`).
func parcel_roll(x: float, y: float) -> float:
	var side := maxf(d("orchard_parcel_px", 2.2), 0.1)
	var ix := floori(x / side)
	var iy := floori(y / side)
	return VegetationFields.hash01(ix * 15731 + iy * 789221 + 3)
