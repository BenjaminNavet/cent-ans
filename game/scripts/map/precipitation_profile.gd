class_name PrecipitationProfile
extends Resource

## Lot SZ5 (suite de ZG7c, défaut S7) : réglages de la pluie et de la neige de près de la carte de
## campagne (`res://resources/precipitation.tres`), lues par `CampaignWeatherView`. Purement
## visuel, aucune règle de jeu.
##
## Contexte (1 unité monde ≈ `MapData.meters_per_px`, ≈ 719 m) : l'ancienne formule dimensionnait
## les gouttes proportionnellement à la distance caméra sur toute la plage (`taille = base × distance
## / 100`), cohérente en vue lointaine (une strie de quelques dizaines de mètres, floutée, se lit
## comme une nappe de pluie vue de loin) mais absurde au palier site : à `distance ≈ 1,8` (seuil
## `ZoomTiers.site_threshold`, ≈ 1,3 km de terrain visible), elle produisait des stries **réelles**
## de plusieurs mètres à plusieurs dizaines de mètres (« bâtonnets géants »).
##
## Cette ressource ancre la taille des gouttes sur une valeur réaliste (mètres) à `near_distance`
## (palier site) et raccorde en douceur (interpolation lisse en logarithme de la distance) avec la
## formule d'origine à `far_distance` (déjà validée en vue vallée / lointaine, ZG7c) : au-delà de
## `far_distance`, le comportement reste **strictement identique** à l'ancien (`far_*_ratio`,
## repris des anciennes constantes). En deçà de `near_distance`, le ratio (taille par unité de
## distance) est maintenu constant : la taille continue de décroître avec la distance (jamais de
## palier plat), donc rien de figé ni de disproportionné tout au fond d'un canyon.
##
## `valeur(distance) = distance × ratio(distance)`, `ratio` interpolé (linéaire en `log(distance)`,
## continu, sans saut) entre `near_*_ratio` (dérivé de `*_m_at_near / near_distance`) à
## `near_distance` et `far_*_ratio` à `far_distance`.

@export var enabled: bool = true

## Distance caméra (unités) où les tailles réalistes (mètres) ci-dessous s'appliquent : le seuil du
## palier site (`ZoomTiers.site_threshold`).
@export var near_distance: float = 1.8
## Distance caméra au-delà de laquelle le comportement d'origine (ZG7c) reprend exactement (ancien
## palier « plat » de la formule remplacée, vue vallée / lointaine déjà validée).
@export var far_distance: float = 30.0

## Pluie : largeur / longueur réelles (m) de la strie au palier site.
@export var rain_width_m_at_near: float = 0.06
@export var rain_length_m_at_near: float = 0.4
## Pluie : ratio (taille / distance) en vue lointaine, repris de l'ancienne formule
## (`0,035 / 100` et `1,1 / 100`).
@export var rain_width_far_ratio: float = 0.00035
@export var rain_length_far_ratio: float = 0.011

## Neige : côté réel (m) du flocon au palier site, puis ratio lointain (ancien `0,18 / 100`).
@export var snow_size_m_at_near: float = 0.03
@export var snow_size_far_ratio: float = 0.0018

## Boîte d'émission (horizontale / verticale) : ratio (rayon / distance) au palier site et en vue
## lointaine (ancien `80 / 100` et `8 / 100`). Resserrée de près pour ne pas faire tomber de gouttes
## à distance de la caméra elle-même.
@export var box_horizontal_near_ratio: float = 0.22
@export var box_horizontal_far_ratio: float = 0.8
@export var box_vertical_near_ratio: float = 0.05
@export var box_vertical_far_ratio: float = 0.08

## Vitesse de chute (ratio vitesse / distance) : ancien `60 / 100` à `75 / 100` (pluie),
## `8 / 100` à `12 / 100` (neige) en vue lointaine ; de près, vitesse plus faible en valeur relative
## (mêmes ordres de grandeur réels : la pluie tombe à la même vitesse quel que soit le zoom, mais la
## scène étant compressée à distance courte, un ratio plus faible évite des gouttes qui traversent
## la boîte en une fraction de seconde).
@export var rain_speed_near_ratio: Vector2 = Vector2(0.35, 0.45)
@export var rain_speed_far_ratio: Vector2 = Vector2(0.6, 0.75)
@export var snow_speed_near_ratio: Vector2 = Vector2(0.05, 0.08)
@export var snow_speed_far_ratio: Vector2 = Vector2(0.08, 0.12)
@export var snow_sway_near_ratio: float = 0.015
@export var snow_sway_far_ratio: float = 0.03

## Fraction de `amount` réellement émise (`GPUParticles3D.amount_ratio`, sans redémarrage du
## système) : densité de près (gouttes fines, il en faut plus pour rester lisible) et de loin
## (nappe déjà couverte par de grosses stries, moins de particules suffisent).
@export var amount_ratio_near: float = 1.0
@export var amount_ratio_far: float = 0.6

static var _default: PrecipitationProfile = null


## Réglages par défaut (mis en cache), ou une instance neuve si la ressource est absente.
static func load_default() -> PrecipitationProfile:
	if _default != null:
		return _default
	var path := "res://resources/precipitation.tres"
	if ResourceLoader.exists(path):
		_default = load(path) as PrecipitationProfile
	if _default == null:
		_default = PrecipitationProfile.new()
	return _default


## Remplace le profil par défaut (tests, réglages à chaud) ; `null` : relit la ressource.
static func set_default(profile: PrecipitationProfile) -> void:
	_default = profile


## Interpolation lisse (continue, monotone) du ratio en logarithme de la distance : `near_ratio`
## constant sous `near_distance`, `far_ratio` constant au-delà de `far_distance`, transition
## `smoothstep` entre les deux (jamais de saut ni de palier intermédiaire artificiel).
func _blend(distance: float, near_ratio: float, far_ratio: float) -> float:
	var lo := log(maxf(near_distance, 1e-4))
	var hi := log(maxf(far_distance, near_distance * 1.01))
	var t := smoothstep(lo, hi, log(maxf(distance, 1e-4)))
	return lerpf(near_ratio, far_ratio, t)


func _blend_vec(distance: float, near_ratio: Vector2, far_ratio: Vector2) -> Vector2:
	return Vector2(_blend(distance, near_ratio.x, far_ratio.x), _blend(distance, near_ratio.y, far_ratio.y))


## Taille (unités monde) d'une strie de pluie (largeur, longueur) à la distance caméra donnée.
func rain_size(distance: float) -> Vector2:
	var near_width := rain_width_m_at_near / maxf(near_distance, 1e-4)
	var near_length := rain_length_m_at_near / maxf(near_distance, 1e-4)
	var width := distance * _blend(distance, near_width, rain_width_far_ratio)
	var length := distance * _blend(distance, near_length, rain_length_far_ratio)
	return Vector2(width, length)


## Côté (unités monde) d'un flocon de neige à la distance caméra donnée.
func snow_size(distance: float) -> float:
	var near_ratio := snow_size_m_at_near / maxf(near_distance, 1e-4)
	return distance * _blend(distance, near_ratio, snow_size_far_ratio)


## Demi-étendue de la boîte d'émission (horizontale, verticale) à la distance caméra donnée.
func box_extents(distance: float) -> Vector2:
	var horizontal := distance * _blend(distance, box_horizontal_near_ratio, box_horizontal_far_ratio)
	var vertical := distance * _blend(distance, box_vertical_near_ratio, box_vertical_far_ratio)
	return Vector2(horizontal, vertical)


## Vitesse de chute (min, max) à la distance caméra donnée.
func speed(distance: float, snow: bool) -> Vector2:
	if snow:
		return distance * _blend_vec(distance, snow_speed_near_ratio, snow_speed_far_ratio)
	return distance * _blend_vec(distance, rain_speed_near_ratio, rain_speed_far_ratio)


## Amplitude de l'ondulation de la neige à la distance caméra donnée.
func sway(distance: float) -> float:
	return distance * _blend(distance, snow_sway_near_ratio, snow_sway_far_ratio)


## Fraction de particules actives (`amount_ratio`) à la distance caméra donnée.
func amount_ratio(distance: float) -> float:
	return _blend(distance, amount_ratio_near, amount_ratio_far)
