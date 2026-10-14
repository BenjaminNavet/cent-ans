class_name ReliefExaggerationProfile
extends Resource

## Relief exagéré « façon Total War » sur la carte de campagne. Purement (ADR 0036)
## visuel (`res://resources/relief_exaggeration.tres`, voisin de `close_camera.tres`).
##
## Hauteur affichée (unités monde) en un point (x, z) d'altitude h (m) :
##     y = s(d) · (h + g(d) · max(h − fond(x, z), 0))
## - `s(d)` : échelle verticale dynamique (`MapData.vertical_scale()`), dont le plancher
##   de près devient `near_exaggeration` (au lieu de `CloseCameraProfile.exaggeration_near`).
## - `g(d)` : gain de relief LOCAL, de `gain_far` (vue stratégique) à `gain_near` (au ras du sol),
##   fonction de l'échelle quantifiée (même paliers : un seul signal de recalage).
## - `fond` : fond de vallée lissé (min puis flou, ~10 km), grille basse résolution calculée une
##   fois au chargement (`ReliefFloor`), toujours ≥ 0 : la côte (h = 0) ne bouge pas, la mer non
##   plus ; plaines et fonds de vallée (h ≈ fond) restent plats, rivières et ponts aussi.
## Interrupteur : `enabled = false` (ou gains nuls et `near_exaggeration` = 1,5) rend exactement
## le comportement.

@export var enabled: bool = true
## Plancher de l'exagération verticale de près (× relief vrai).
@export var near_exaggeration: float = 2.5
## Gain de relief local en vue stratégique et au plus près.
@export var gain_far: float = 0.3
@export var gain_near: float = 0.8

## Fond de vallée : taille d'une cellule (pixels de la carte 4096, 1 px = 719 m), pas
## d'échantillonnage dans la cellule, rayon du filtre min (cellules) puis flou (passes de boîte
## de rayon `floor_blur_radius`).
@export var floor_cell_px: int = 8
@export var floor_sample_step: int = 2
@export var floor_min_radius: int = 1
@export var floor_blur_radius: int = 2
@export var floor_blur_passes: int = 2
## Plafond du relief local exagéré (m). Le fond est relevé à `sommets voisins − plafond`,
## donc `h − fond` ≲ plafond : collines, coteaux et falaises (< 300 m) gardent tout leur gain, les
## montagnes n'en reçoivent que sur leurs `plafond` derniers mètres (sinon aiguilles et murs aux
## paliers vallée et site). 0 : pas de plafond (rendu ZG8 d'origine).
@export var local_relief_cap_m: float = 350.0

## Écrasement des montagnes. Amplitude régionale A = sommets voisins − fond non plafonné
## (`base`) ; au-delà du genou `mountain_knee_m`, le relief au-dessus de la base n'est plus affiché
## qu'à `mountain_ratio` : amplitude affichée D = genou + (A − genou)·ratio, facteur k = 1 − D/A
## (0 pour les collines, falaises et plaines). Hauteur affichée :
##     y = s·(h − K·max(h − base, 0) + g·(1 − K)·max(h − fond, 0)), K = c(s)·k
## (le gain local ZG8 est écrasé d'autant : pas d'aiguilles sur des montagnes aplaties)
## `c(s)` va de `mountain_squash_far` (échelle stratégique) à 1 à l'exagération
## `mountain_squash_full_exaggeration` (palier vallée) et en deçà. Genou ≤ 0 : désactivé.
@export var mountain_knee_m: float = 350.0
@export var mountain_ratio: float = 0.3
@export var mountain_squash_far: float = 0.0
@export var mountain_squash_full_exaggeration: float = 3.5
## Borne du facteur d'écrasement des montagnes (au-delà, les Alpes s'aplanissent au palier site).
@export var mountain_squash_max: float = 0.55
## RE (relief exagéré façon Total War) : au lieu d'écraser les montagnes, le facteur k devient
## négatif et AMPLIFIE le relief au-dessus de la base : 1 − k = 1 + `mountain_lift` quand
## l'amplitude régionale atteint `mountain_lift_full_m` (montée en smoothstep depuis
## `mountain_lift_start_m`), au-delà croît encore en (A / full)^`mountain_lift_power` − 1 (les
## grandes chaînes plus que les moyennes montagnes), borné par `mountain_lift_max`. Les plaines
## (amplitude ≤ `plain_flatten_full_m`, fondu jusqu'à `plain_flatten_end_m`) gardent k =
## `plain_flatten` > 0 : le micro-relief est ramené vers le fond. 0 : désactivé.
@export var mountain_lift: float = 0.0
@export var mountain_lift_start_m: float = 500.0
@export var mountain_lift_full_m: float = 1600.0
@export var mountain_lift_power: float = 0.0
@export var mountain_lift_max: float = 6.0
## RE : relevé LISSE des massifs (m), dôme basse fréquence ajouté à l'altitude affichée :
## D = `massif_lift` × poids(A) × max(L − base, 0), L = altitude moyenne lissée de la cellule
## (`massif_smooth_radius` cellules de flou), poids(A) = même montée que `mountain_lift`
## (smoothstep start → full, puissance `mountain_lift_power`), borné par `mountain_lift_max`.
## Les chaînes montent en bloc, leurs crêtes gardent leur dessin (pas d'aiguilles). 0 : coupé.
@export var massif_lift: float = 0.0
@export var massif_smooth_radius: int = 1
## Fondu du relevé lisse vers la côte (cellules) : nul sur toute cellule touchant la mer.
@export var massif_coast_fade_cells: int = 2
@export var plain_flatten: float = 0.0
@export var plain_flatten_full_m: float = 120.0
@export var plain_flatten_end_m: float = 350.0
## Autour des villes emblématiques 1:1 (VH4) : facteur d'écrasement du relief au-dessus de la base
## (0,7 ≈ échelle vraie aux paliers vallée et site), plein jusqu'au rayon de la ville +
## `true_scale_full_units`, fondu sur `true_scale_fade_units` (unités carte). 0 : coupé.
@export var true_scale_squash: float = 0.7
@export var true_scale_full_units: float = 4.0
@export var true_scale_fade_units: float = 6.0

## Falaises : pentes du relief exagéré localement (pente vraie × (1 + gain), m/m) où la roche
## remplace progressivement la couverture du sol (terrain.gdshader).
@export var cliff_slope_start: float = 0.45
@export var cliff_slope_full: float = 1.0
## RE-R3 : part de neige retirée sur les falaises affichées (0 : neige inchangée, 1 : roche nue).
@export_range(0.0, 1.0) var snow_cliff_shed: float = 0.85
## Lumière plus rasante : hauteur du soleil (degrés) de la carte de campagne, azimut conservé ;
## ≤ 0 : soleil de la scène inchangé.
@export var sun_elevation_deg: float = 34.0

static var _default: ReliefExaggerationProfile = null


## Réglages par défaut (mis en cache), ou une instance neuve si la ressource est absente.
static func load_default() -> ReliefExaggerationProfile:
	if _default != null:
		return _default
	var path := "res://resources/relief_exaggeration.tres"
	if ResourceLoader.exists(path):
		_default = load(path) as ReliefExaggerationProfile
	if _default == null:
		_default = ReliefExaggerationProfile.new()
	return _default


## Remplace le profil par défaut (tests, réglages à chaud) ; null : relit la ressource.
static func set_default(profile: ReliefExaggerationProfile) -> void:
	_default = profile


## Plancher effectif de l'exagération de près : `near_exaggeration` si actif, sinon celui de ZG4.
func near_floor(zg4_near: float) -> float:
	return near_exaggeration if enabled else zg4_near


## Gain de relief local pour une échelle verticale (unités monde par mètre) : `gain_far` à
## `MapData.HEIGHT_SCALE`, `gain_near` au plancher `s_near`, interpolé en logarithme de l'échelle.
func gain_for_scale(scale: float, s_near: float) -> float:
	if not enabled:
		return 0.0
	var far := MapData.HEIGHT_SCALE
	if s_near >= far * 0.999:
		return gain_far
	var t := clampf(log(far / maxf(scale, 1e-9)) / log(far / s_near), 0.0, 1.0)
	return lerpf(gain_far, gain_near, t)


## Plus grand gain possible (boîtes englobantes conservatrices).
func max_gain() -> float:
	return maxf(gain_far, gain_near) if enabled else 0.0


## Facteur k d'une amplitude régionale A (m) : écrasement SZ1 dans [0, mountain_squash_max],
## moins le relevé RE des montagnes (k < 0 : amplification), plus l'aplanissement des plaines.
func mountain_squash_of(amplitude_m: float) -> float:
	if not enabled:
		return 0.0
	var k := 0.0
	if mountain_knee_m > 0.0 and amplitude_m > mountain_knee_m:
		var shown := mountain_knee_m + (amplitude_m - mountain_knee_m) * mountain_ratio
		k = clampf(1.0 - shown / amplitude_m, 0.0, mountain_squash_max)
	return k - mountain_lift_of(amplitude_m) + plain_flatten_of(amplitude_m)


## Relevé RE (≥ 0) d'une amplitude régionale A (m) : 1 + relevé = facteur du relief au-dessus de la base.
func mountain_lift_of(amplitude_m: float) -> float:
	if not enabled or mountain_lift <= 0.0 or amplitude_m <= mountain_lift_start_m:
		return 0.0
	var span := maxf(mountain_lift_full_m - mountain_lift_start_m, 1.0)
	var t := clampf((amplitude_m - mountain_lift_start_m) / span, 0.0, 1.0)
	var lift := mountain_lift * t * t * (3.0 - 2.0 * t)
	if mountain_lift_power > 0.0 and amplitude_m > mountain_lift_full_m:
		lift = (1.0 + lift) * pow(amplitude_m / mountain_lift_full_m, mountain_lift_power) - 1.0
	return minf(lift, mountain_lift_max)


## Poids (0..mountain_lift_max) du relevé lisse des massifs pour une amplitude régionale A (m).
func massif_lift_of(amplitude_m: float) -> float:
	if not enabled or massif_lift <= 0.0 or amplitude_m <= mountain_lift_start_m:
		return 0.0
	var span := maxf(mountain_lift_full_m - mountain_lift_start_m, 1.0)
	var t := clampf((amplitude_m - mountain_lift_start_m) / span, 0.0, 1.0)
	var lift := massif_lift * t * t * (3.0 - 2.0 * t)
	if mountain_lift_power > 0.0 and amplitude_m > mountain_lift_full_m:
		lift *= pow(amplitude_m / mountain_lift_full_m, mountain_lift_power)
	return minf(lift, mountain_lift_max)


## Aplanissement RE des plaines (0..plain_flatten) d'une amplitude régionale A (m).
func plain_flatten_of(amplitude_m: float) -> float:
	if not enabled or plain_flatten <= 0.0 or amplitude_m >= plain_flatten_end_m:
		return 0.0
	var span := maxf(plain_flatten_end_m - plain_flatten_full_m, 1.0)
	var t := clampf((plain_flatten_end_m - amplitude_m) / span, 0.0, 1.0)
	return plain_flatten * t * t * (3.0 - 2.0 * t)


## Vrai si le champ k peut être non nul (écrasement, relevé, aplanissement ou zones 1:1).
func shapes_relief() -> bool:
	return enabled and (mountain_knee_m > 0.0 or mountain_lift > 0.0 or massif_lift > 0.0 or plain_flatten > 0.0 or true_scale_squash > 0.0)


## Poids c de l'écrasement pour une échelle verticale (unités monde par mètre) :
## `mountain_squash_far` à `MapData.HEIGHT_SCALE`, 1 à `s_full` et en deçà (logarithme de l'échelle).
func squash_weight_for_scale(scale: float, s_full: float) -> float:
	if not shapes_relief():
		return 0.0
	var far := MapData.HEIGHT_SCALE
	if s_full >= far * 0.999:
		return 1.0
	var t := clampf(log(far / maxf(scale, 1e-9)) / log(far / s_full), 0.0, 1.0)
	return lerpf(mountain_squash_far, 1.0, t)
