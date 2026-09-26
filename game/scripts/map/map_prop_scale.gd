class_name MapPropScale
extends Resource

## Lot SZ4 (suites du zoom ZG, ADR 0036) : échelle des accessoires dessinés à l'échelle de la carte
## (arbres, moulins, hameaux, fumées) selon la distance du rig de caméra
## (`res://resources/map_prop_scale.tres`). Purement visuel.
##
## Au-delà de `shrink_start` (paliers moyen / Europe, haut du palier comté), les accessoires gardent
## leur taille de carte (lisibles de loin : un arbre fait ~1 km, un moulin ~2 km). En deçà, ils
## rétrécissent continûment jusqu'à leur taille réelle (`*_ratio` = taille réelle / taille carte),
## atteinte à `shrink_end`, avant le palier site : cohérents avec les maisons 1:1 du lot ZG6. La
## transition est un `smoothstep` sur le logarithme de la distance : la taille apparente décroît
## sans saut, et l'échelle ne varie jamais plus vite que la distance elle-même.

## Distance du rig (unités monde, 1 unité ≈ 719 m) au-dessus de laquelle rien ne change.
@export var shrink_start: float = 28.0
## Distance en deçà de laquelle les accessoires sont à leur taille réelle.
@export var shrink_end: float = 5.0
## Arbres : taille réelle atteinte plus bas (le semis est clairsemé pour des arbres de ~1 km ; à
## taille réelle dès le palier vallée haut, les forêts ne seraient plus qu'un tapis sombre).
@export var tree_shrink_end: float = 3.0
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
## (moulins, hameaux) ne sont pas réécrites (évite une réécriture par image pendant un zoom).
@export var rewrite_step: float = 0.04

static var _default: MapPropScale = null


## Avancement [0, 1] de la transition taille carte → taille réelle à la distance `distance`.
## `end` : distance de fin propre à une famille (`shrink_end` par défaut).
func progress(distance: float, end: float = -1.0) -> float:
	var stop := end if end > 0.0 else shrink_end
	if distance >= shrink_start:
		return 0.0
	if distance <= stop:
		return 1.0
	var t := log(shrink_start / maxf(distance, 1e-4)) / log(shrink_start / stop)
	return smoothstep(0.0, 1.0, t)


## Échelle (1 au loin, `ratio` de près) pour une famille de taille réelle `ratio`.
func scale_for(ratio: float, distance: float, end: float = -1.0) -> float:
	var t := progress(distance, end)
	if t <= 0.0:
		return 1.0
	return pow(clampf(ratio, 1e-4, 1.0), t)


func tree_scale(distance: float) -> float:
	return scale_for(tree_ratio, distance, tree_shrink_end)


func windmill_scale(distance: float) -> float:
	return scale_for(windmill_ratio, distance)


func hamlet_scale(distance: float) -> float:
	return scale_for(hamlet_ratio, distance)


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
