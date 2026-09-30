class_name MapPropScale
extends Resource

## Lot SZ4 (suites du zoom ZG, ADR 0036) : échelle des accessoires dessinés à l'échelle de la carte
## (arbres, moulins, hameaux, fumées ; lot SZ4b : maquettes des colonies) selon la distance du rig
## de caméra (`res://resources/map_prop_scale.tres`). Purement visuel.
##
## Au-delà de `shrink_start` (paliers moyen / Europe, haut du palier comté), les accessoires gardent
## leur taille de carte (lisibles de loin : un arbre fait ~1 km, un moulin ~2 km). En deçà, ils
## rétrécissent continûment jusqu'à leur taille réelle (`*_ratio` = taille réelle / taille carte),
## atteinte à `shrink_end` : le seuil du palier vallée, où les villes 1:1 du lot ZG6 prennent le
## relais des maquettes.
##
## Lot SZ4b : une seule **exagération** E(d) pour toutes les familles (taille affichée / taille
## réelle), de `max_exaggeration` à `shrink_start` jusqu'à 1 à `shrink_end`, `smoothstep` sur le
## logarithme de la distance ; échelle d'une famille = min(1, ratio × E). À une distance donnée, un
## arbre, un moulin, un hameau et une maison de village sont donc grossis du même facteur (plus de
## moulin rétréci à côté d'un village géant), et chaque famille ne quitte sa taille de carte que
## quand E passe sous 1 / ratio.
##
## VT / VT2 / VT3 (ADR 0138) : l'exagération ne s'applique plus qu'aux **incendies** (et, par
## `clutter_scale`, aux touffes d'herbe du lot FC3, hors demande). Hameaux, moulins, panaches de
## cheminée et **arbres** (VT3) sont à l'échelle 1:1 à toute distance (échelle constante), coupés
## au-delà d'une portée de visibilité quand ils deviennent sous-pixel ; au-delà de la portée des
## arbres, la forêt est portée par le terrain (canopée du shader `terrain.gdshader`).

## Distance du rig (unités monde, 1 unité ≈ 719 m) au-dessus de laquelle rien ne change.
@export var shrink_start: float = 28.0
## Distance en deçà de laquelle tout est à sa taille réelle (seuil du palier vallée, SZ4b).
@export var shrink_end: float = 8.0
## Exagération à `shrink_start` (les incendies y sont encore à leur taille de carte).
@export var max_exaggeration: float = 125.0
## Taille réelle / taille carte, par famille (1 unité ≈ 719 m). Hameaux : ~2 unités de large,
## 40-60 m en vrai. Fumées d'incendie : 2 × 9 km sur la carte, ~60 × 400 m en vrai.
@export var hamlet_ratio: float = 0.03
@export var fire_ratio: float = 0.05
## VT3 : arbres à l'échelle 1:1 à toute distance (échelle constante, `tree_scale()`). Hauteurs de
## modèle (`VegetationTileJob._make_instance`, 1 = hauteur du maillage) : chêne 1,1-1,7, hêtre
## 1,35-1,95, conifère 1,3-2,1, haie 0,3-0,46 ; × 0,018 × 719 m : chêne 14-22 m, hêtre 17-25 m,
## conifère 17-27 m, haie 4-6 m (avant VT3 : 0,035, soit 28-53 m, et grossis jusqu'à ×125).
@export var tree_ratio: float = 0.018
## Portée des arbres (distance du rig, unités) : au-delà, plus aucun arbre individuel (cartes,
## maillages, imposteurs) n'est dessiné. Arbre de 20 m en 1080p, fov 55° : ≈ 29 / D px à D unités
## de la caméra, soit 1 px vers D = 29. Fondu sur `visibility_fade` (dernier 20 %).
@export var tree_max_distance: float = 30.0
## Distance caméra → arbre (métrique du shader de feuillage : horizontale + moitié de la hauteur)
## au-delà de laquelle un arbre n'est plus dessiné (≈ 1 px), éteint par graine sur
## `tree_view_fade` × cette portée.
@export var tree_view_range: float = 30.0
@export var tree_view_fade: float = 0.35
## Distance du rig au-delà de laquelle les arbres ne portent plus d'ombre (arbre de 20 m ≈ 3 px
## au point visé à d = 10 en 1080p).
@export var tree_shadow_distance: float = 12.0
## Touffes d'herbe et broussailles (`GroundClutter`, FC3) : exagération d'avant VT3 conservée
## (ancien `tree_ratio`), hors demande.
@export var clutter_ratio: float = 0.035
## VT2 (ADR 0138, addendum) : moulins, panaches de cheminée (et figurants FK, `map_scenes.json`)
## sont à l'échelle 1:1 à toute distance, comme les villes et les hameaux : échelle constante
## `*_ratio` × leur taille de modèle, plus d'exagération. Moulin (`WINDMILL_SCALE` 4,6 × modèle) :
## faîte du corps 0,36 u de modèle → ~11 m, moyeu ~9 m, ailes ~18 m d'envergure (moulin sur pivot
## médiéval : 10-12 m, ailes 18-24 m). Panache (`CHIMNEY_SIZE` 1,3 × 4, ± 20 %) : ~9 × 28 m.
@export var windmill_ratio: float = 0.0093
@export var chimney_ratio: float = 0.0098
## Opacité des panaches de cheminée (un filet de fumée de 30 m n'est qu'un voile).
@export var chimney_real_alpha: float = 0.5
## Portées de visibilité (distance du rig, unités monde) : au-delà, moulins et panaches ne sont
## plus dessinés (moins d'un à deux pixels en 1080p, fov 55°, on ne paie pas un rendu invisible).
## Moulin ~18 m hors tout : ≈ 26 / d px ; panache ~28 m de haut : ≈ 40 / d px.
@export var windmill_max_distance: float = 30.0
@export var chimney_max_distance: float = 25.0
## Part de la portée des panaches sur laquelle leur opacité s'éteint (pas d'apparition brusque).
@export var visibility_fade: float = 0.2
## Variation relative d'échelle en deçà de laquelle les instances recalculées sur le processeur
## ne sont pas réécrites (évite une réécriture par image pendant un zoom).
@export var rewrite_step: float = 0.04

static var _default: MapPropScale = null


## Avancement [0, 1] de la transition taille carte → taille réelle à la distance `distance`.
func progress(distance: float) -> float:
	if distance >= shrink_start:
		return 0.0
	if distance <= shrink_end:
		return 1.0
	var t := log(shrink_start / maxf(distance, 1e-4)) / log(shrink_start / shrink_end)
	return smoothstep(0.0, 1.0, t)


## Exagération commune (taille affichée / taille réelle) à la distance `distance`.
func exaggeration(distance: float) -> float:
	return pow(maxf(max_exaggeration, 1.0), 1.0 - progress(distance))


## Échelle (1 au loin, `ratio` de près) d'une famille de taille réelle `ratio` × sa taille de carte.
func scale_for(ratio: float, distance: float) -> float:
	var r := clampf(ratio, 1e-4, 1.0)
	return minf(1.0, r * exaggeration(distance))


## VT3 : arbres à l'échelle 1:1 à toute distance (`tree_ratio` × leur taille de modèle).
func tree_scale() -> float:
	return clampf(tree_ratio, 1e-4, 1.0)


## VT3 : poids [0, 1] des arbres individuels à la distance du rig `distance` (1 en deçà, 0 au-delà
## de `tree_max_distance`, fondu sur `visibility_fade`).
func trees_weight(distance: float) -> float:
	return range_weight(distance, tree_max_distance)


## VT3 : vrai si des arbres individuels sont dessinés à la distance du rig `distance`.
func trees_visible(distance: float) -> bool:
	return distance < tree_max_distance


## Taille à l'écran (px) d'un objet de `height_m` mètres à `camera_distance` unités de la caméra,
## pour un écran de `screen_px` px de haut et un champ vertical `fov_deg` (réglage des portées).
static func pixels_for(height_m: float, camera_distance: float, screen_px: float = 1080.0, fov_deg: float = 55.0) -> float:
	var px_per_unit := screen_px / (2.0 * maxf(camera_distance, 1e-4) * tan(deg_to_rad(fov_deg) * 0.5))
	return height_m / 719.0 * px_per_unit


## Touffes d'herbe (FC3) : échelle exagérée d'avant VT3 (`clutter_ratio`).
func clutter_scale(distance: float) -> float:
	return scale_for(clutter_ratio, distance)


## VT2 : moulins à l'échelle 1:1 à toute distance (`windmill_ratio` × leur taille de modèle).
func windmill_scale() -> float:
	return clampf(windmill_ratio, 1e-4, 1.0)


## VT (ADR 0138) : les hameaux sont à l'échelle 1:1 à toute distance (pas d'exagération) :
## toujours `hamlet_ratio` × leur taille de modèle. `distance` est gardé pour la signature commune.
func hamlet_scale(_distance: float) -> float:
	return clampf(hamlet_ratio, 1e-4, 1.0)


## VT2 : panaches de cheminée à l'échelle 1:1 à toute distance.
func chimney_scale() -> float:
	return clampf(chimney_ratio, 1e-4, 1.0)


## Facteur d'opacité des panaches de cheminée : `chimney_real_alpha`, éteint en approchant de
## `chimney_max_distance` (0 au-delà).
func chimney_alpha(distance: float) -> float:
	return chimney_real_alpha * range_weight(distance, chimney_max_distance)


## Vrai si les moulins sont dessinés à la distance `distance`.
func windmills_visible(distance: float) -> bool:
	return distance < windmill_max_distance


## Poids de visibilité (1 en deçà, 0 au-delà de `max_distance`, fondu sur `visibility_fade`).
func range_weight(distance: float, max_distance: float) -> float:
	var fade := maxf(max_distance * visibility_fade, 1e-4)
	return 1.0 - smoothstep(max_distance - fade, max_distance, distance)


func fire_scale(distance: float) -> float:
	return scale_for(fire_ratio, distance)


## Vrai si l'échelle `now` s'écarte assez de `applied` pour réécrire des instances.
func needs_rewrite(applied: float, now: float) -> bool:
	if applied <= 0.0:
		return true
	return absf(now / applied - 1.0) > rewrite_step or (now == 1.0 and applied != 1.0)


## Réglages par défaut (ressource partagée), ou une instance neuve si absente.
static func shared() -> MapPropScale:
	if _default == null:
		var path := "res://resources/map_prop_scale.tres"
		if ResourceLoader.exists(path):
			_default = load(path) as MapPropScale
		if _default == null:
			_default = MapPropScale.new()
	return _default
