class_name ReliefExaggerationProfile
extends Resource

## Lot ZG8 (ADR 0036) : relief exagéré « façon Total War » sur la carte de campagne. Purement
## visuel (`res://resources/relief_exaggeration.tres`, voisin de `close_camera.tres`).
##
## Hauteur affichée (unités monde) en un point (x, z) d'altitude h (m) :
##     y = s(d) · (h + g(d) · max(h − fond(x, z), 0))
## - `s(d)` : échelle verticale dynamique du lot ZG4 (`MapData.vertical_scale()`), dont le plancher
##   de près devient `near_exaggeration` (au lieu de `CloseCameraProfile.exaggeration_near`).
## - `g(d)` : gain de relief LOCAL, de `gain_far` (vue stratégique) à `gain_near` (au ras du sol),
##   fonction de l'échelle quantifiée (même paliers : un seul signal de recalage).
## - `fond` : fond de vallée lissé (min puis flou, ~10 km), grille basse résolution calculée une
##   fois au chargement (`ReliefFloor`), toujours ≥ 0 : la côte (h = 0) ne bouge pas, la mer non
##   plus ; plaines et fonds de vallée (h ≈ fond) restent plats, rivières et ponts aussi.
## Interrupteur : `enabled = false` (ou gains nuls et `near_exaggeration` = 1,5) rend exactement
## le comportement du lot ZG4.

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
## ZG7c : plafond du relief local exagéré (m). Le fond est relevé à `sommets voisins − plafond`,
## donc `h − fond` ≲ plafond : collines, coteaux et falaises (< 300 m) gardent tout leur gain, les
## montagnes n'en reçoivent que sur leurs `plafond` derniers mètres (sinon aiguilles et murs aux
## paliers vallée et site). 0 : pas de plafond (rendu ZG8 d'origine).
@export var local_relief_cap_m: float = 350.0

## SZ1 : écrasement des montagnes. Amplitude régionale A = sommets voisins − fond non plafonné
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
## Borne du facteur d'écrasement (garde l'inverse bien conditionné).
@export var mountain_squash_max: float = 0.85

## Falaises : pentes du relief exagéré localement (pente vraie × (1 + gain), m/m) où la roche
## remplace progressivement la couverture du sol (terrain.gdshader).
@export var cliff_slope_start: float = 0.45
@export var cliff_slope_full: float = 1.0
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
	# `--no-relief-exaggeration` (après `--`) : comportement du lot ZG4 (captures « avant »).
	if _default.enabled and OS.get_cmdline_user_args().has("--no-relief-exaggeration"):
		_default = _default.duplicate()
		_default.enabled = false
	# `--no-mountain-squash` : sans l'écrasement des montagnes du lot SZ1 (captures « avant »).
	if _default.mountain_knee_m > 0.0 and OS.get_cmdline_user_args().has("--no-mountain-squash"):
		_default = _default.duplicate()
		_default.mountain_knee_m = 0.0
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


## SZ1 : facteur d'écrasement k d'une amplitude régionale A (m), dans [0, mountain_squash_max].
func mountain_squash_of(amplitude_m: float) -> float:
	if not enabled or mountain_knee_m <= 0.0 or amplitude_m <= mountain_knee_m:
		return 0.0
	var shown := mountain_knee_m + (amplitude_m - mountain_knee_m) * mountain_ratio
	return clampf(1.0 - shown / amplitude_m, 0.0, mountain_squash_max)


## SZ1 : poids c de l'écrasement pour une échelle verticale (unités monde par mètre) :
## `mountain_squash_far` à `MapData.HEIGHT_SCALE`, 1 à `s_full` et en deçà (logarithme de l'échelle).
func squash_weight_for_scale(scale: float, s_full: float) -> float:
	if not enabled or mountain_knee_m <= 0.0:
		return 0.0
	var far := MapData.HEIGHT_SCALE
	if s_full >= far * 0.999:
		return 1.0
	var t := clampf(log(far / maxf(scale, 1e-9)) / log(far / s_full), 0.0, 1.0)
	return lerpf(mountain_squash_far, 1.0, t)
