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
## Cette ressource ancre la taille des gouttes sur une valeur réaliste (mètres, convertie en unités
## monde via `meters_per_unit`) à `near_distance` (palier site) et la raccorde à la formule
## d'origine à `far_distance` (déjà validée en vue vallée / lointaine, ZG7c) par une interpolation
## en loi de puissance (linéaire en `log(valeur)` vs `log(distance)`) : la valeur croît en continu,
## sans jamais dépasser l'une ou l'autre des deux valeurs d'ancrage entre les deux distances (donc
## pas de « rebond » ni de géant local). Au-delà de `far_distance`, le comportement reste
## **strictement identique** à l'ancien (`far_*_ratio`, repris des anciennes constantes, proportion
## constante à la distance). En deçà de `near_distance`, le ratio (taille / distance) est maintenu
## constant à sa valeur d'ancrage : la taille continue de décroître avec la distance (jamais de
## palier plat), donc rien de figé ni de disproportionné tout au fond d'un canyon.

@export var enabled: bool = true

## Distance caméra (unités) où les tailles réalistes (mètres) ci-dessous s'appliquent : le seuil du
## palier site (`ZoomTiers.site_threshold`).
@export var near_distance: float = 1.8
## Distance caméra au-delà de laquelle le comportement d'origine (ZG7c) reprend exactement (ancien
## palier « plat » de la formule remplacée, vue vallée / lointaine déjà validée).
@export var far_distance: float = 30.0
## Mètres réels par unité monde (`MapData.meters_per_px`, ≈ 719) ; passé par l'appelant à chaque
## image (dépend de la carte chargée), valeur par défaut si absent.
@export var meters_per_unit: float = 719.0

## Pluie : largeur / longueur réelles (m) de la strie au palier site. Une goutte réellement fidèle
## (millimétrique) serait sous un pixel à la distance caméra la plus proche (≈ 200 m au sol) : ces
## valeurs restent stylisées (quelques dizaines de cm), mais très en deçà de l'ancien comportement
## (des dizaines de mètres, d'où les « bâtonnets géants »).
@export var rain_width_m_at_near: float = 0.35
@export var rain_length_m_at_near: float = 2.5
## Pluie : ratio (taille / distance) en vue lointaine, repris de l'ancienne formule
## (`0,035 / 100` et `1,1 / 100`).
@export var rain_width_far_ratio: float = 0.00035
@export var rain_length_far_ratio: float = 0.011

## Neige : côté réel (m) du flocon au palier site, puis ratio lointain (ancien `0,18 / 100`).
@export var snow_size_m_at_near: float = 0.2
@export var snow_size_far_ratio: float = 0.0018

## Boîte d'émission (horizontale / verticale) : réelle (m) au palier site, ratio (rayon / distance)
## en vue lointaine (ancien `80 / 100` et `8 / 100`). De près, une boîte de quelques dizaines de
## mètres suffit à couvrir le champ visuel sans faire tomber de gouttes à distance de la caméra.
@export var box_horizontal_m_at_near: float = 40.0
@export var box_horizontal_far_ratio: float = 0.8
@export var box_vertical_m_at_near: float = 8.0
@export var box_vertical_far_ratio: float = 0.08

## Hauteur (m réels au palier site, ratio × distance en vue lointaine, ancien `0,35`) du centre de
## la boîte au-dessus du point visé. De près, centrée juste au-dessus du sol (dans le champ visible
## de la caméra rapprochée) ; l'ancienne valeur (`0,35 × distance`) plaçait la pluie très haut dans
## le ciel une fois la boîte réduite à une échelle réaliste (invisible depuis le palier site).
@export var height_m_at_near: float = 25.0
@export var height_far_ratio: float = 0.35

## Vitesse de chute : réelle (m/s) au palier site (vitesse physique de la pluie / neige, quel que
## soit le zoom), ratio (vitesse / distance) en vue lointaine (ancien `60/100`-`75/100` pluie,
## `8/100`-`12/100` neige : la scène y est fortement compressée, la vitesse y suit la distance pour
## rester lisible).
@export var rain_speed_ms_at_near: Vector2 = Vector2(6.0, 9.0)
@export var rain_speed_far_ratio: Vector2 = Vector2(0.6, 0.75)
@export var snow_speed_ms_at_near: Vector2 = Vector2(0.8, 1.4)
@export var snow_speed_far_ratio: Vector2 = Vector2(0.08, 0.12)
@export var snow_sway_ms_at_near: float = 0.3
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


## Valeur (unités monde) à la distance caméra donnée, ancrée sur `near_value` (unités monde) à
## `near_distance` et sur `far_ratio × far_distance` à `far_distance`. Sous `near_distance` : ratio
## constant (`near_value / near_distance`), la valeur continue de décroître avec la distance.
## Entre les deux : loi de puissance (linéaire en logarithme des deux axes), donc monotone et bornée
## par les deux ancrages, sans jamais les dépasser. Au-delà de `far_distance` : ratio constant
## `far_ratio` (comportement d'origine, inchangé).
func _value(distance: float, near_value: float, far_ratio: float) -> float:
	var d1 := maxf(near_distance, 1e-4)
	var d2 := maxf(far_distance, d1 * 1.01)
	var d := maxf(distance, 1e-4)
	var far_value := far_ratio * d2
	if d <= d1:
		return near_value * (d / d1)
	if d >= d2:
		return far_ratio * d
	var t := (log(d) - log(d1)) / (log(d2) - log(d1))
	var lo := maxf(near_value, 1e-9)
	var hi := maxf(far_value, 1e-9)
	return lo * pow(hi / lo, t)


func _value_vec(distance: float, near_value: Vector2, far_ratio: Vector2) -> Vector2:
	return Vector2(_value(distance, near_value.x, far_ratio.x), _value(distance, near_value.y, far_ratio.y))


## Taille (unités monde) d'une strie de pluie (largeur, longueur) à la distance caméra donnée.
func rain_size(distance: float) -> Vector2:
	var mpu := maxf(meters_per_unit, 1e-3)
	var width := _value(distance, rain_width_m_at_near / mpu, rain_width_far_ratio)
	var length := _value(distance, rain_length_m_at_near / mpu, rain_length_far_ratio)
	return Vector2(width, length)


## Côté (unités monde) d'un flocon de neige à la distance caméra donnée.
func snow_size(distance: float) -> float:
	var mpu := maxf(meters_per_unit, 1e-3)
	return _value(distance, snow_size_m_at_near / mpu, snow_size_far_ratio)


## Demi-étendue de la boîte d'émission (horizontale, verticale) à la distance caméra donnée.
func box_extents(distance: float) -> Vector2:
	var mpu := maxf(meters_per_unit, 1e-3)
	var horizontal := _value(distance, box_horizontal_m_at_near / mpu, box_horizontal_far_ratio)
	var vertical := _value(distance, box_vertical_m_at_near / mpu, box_vertical_far_ratio)
	return Vector2(horizontal, vertical)


## Hauteur (unités monde) du centre de la boîte d'émission au-dessus du point visé.
func height(distance: float) -> float:
	var mpu := maxf(meters_per_unit, 1e-3)
	return _value(distance, height_m_at_near / mpu, height_far_ratio)


## Vitesse de chute (min, max, unités monde / s) à la distance caméra donnée.
func speed(distance: float, snow: bool) -> Vector2:
	var mpu := maxf(meters_per_unit, 1e-3)
	if snow:
		return _value_vec(distance, snow_speed_ms_at_near / mpu, snow_speed_far_ratio)
	return _value_vec(distance, rain_speed_ms_at_near / mpu, rain_speed_far_ratio)


## Amplitude de l'ondulation de la neige à la distance caméra donnée.
func sway(distance: float) -> float:
	var mpu := maxf(meters_per_unit, 1e-3)
	return _value(distance, snow_sway_ms_at_near / mpu, snow_sway_far_ratio)


## Fraction de particules actives (`amount_ratio`) à la distance caméra donnée : interpolation
## simple (linéaire en logarithme de la distance), pas de conversion physique nécessaire.
func amount_ratio(distance: float) -> float:
	var lo := log(maxf(near_distance, 1e-4))
	var hi := log(maxf(far_distance, near_distance * 1.01))
	var t := smoothstep(lo, hi, log(maxf(distance, 1e-4)))
	return lerpf(amount_ratio_near, amount_ratio_far, t)
