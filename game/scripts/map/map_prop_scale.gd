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

## Distance du rig (unités monde, 1 unité ≈ 719 m) au-dessus de laquelle rien ne change.
@export var shrink_start: float = 28.0
## Distance en deçà de laquelle tout est à sa taille réelle (seuil du palier vallée, SZ4b).
@export var shrink_end: float = 8.0
## Exagération à `shrink_start` : 1 / plus petit rapport (moulins), toutes les familles y sont
## encore à leur taille de carte.
@export var max_exaggeration: float = 125.0
## Taille réelle / taille carte, par famille. Arbres : ~1,5 unité de haut sur la carte, 20-30 m en
## vrai. Moulins : corps de 3,3 unités (2,3 km), ~15 m en vrai. Hameaux : ~2 unités de large,
## 40-60 m en vrai. Panaches de cheminée : 1 × 3 km sur la carte, ~10 × 40 m en vrai ; fumées
## d'incendie : 2 × 9 km, ~60 × 400 m en vrai.
@export var tree_ratio: float = 0.035
@export var windmill_ratio: float = 0.008
@export var hamlet_ratio: float = 0.03
@export var chimney_ratio: float = 0.02
@export var fire_ratio: float = 0.05
## Opacité des panaches de cheminée à taille réelle (fondu avec l'échelle) : un filet de fumée de
## 20 m vu à un kilomètre n'est qu'un voile.
@export var chimney_real_alpha: float = 0.5
## Variation relative d'échelle en deçà de laquelle les instances recalculées sur le processeur
## (moulins, panaches) ne sont pas réécrites (évite une réécriture par image pendant un
## zoom).
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


func tree_scale(distance: float) -> float:
	return scale_for(tree_ratio, distance)


func windmill_scale(distance: float) -> float:
	return scale_for(windmill_ratio, distance)


## VT (ADR 0138) : les hameaux sont à l'échelle 1:1 à toute distance (pas d'exagération) :
## toujours `hamlet_ratio` × leur taille de modèle. `distance` est gardé pour la signature commune.
func hamlet_scale(_distance: float) -> float:
	return clampf(hamlet_ratio, 1e-4, 1.0)


func chimney_scale(distance: float) -> float:
	return scale_for(chimney_ratio, distance)


## Facteur d'opacité des panaches de cheminée (1 au loin, `chimney_real_alpha` de près).
func chimney_alpha(distance: float) -> float:
	return lerpf(1.0, chimney_real_alpha, progress(distance))


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
