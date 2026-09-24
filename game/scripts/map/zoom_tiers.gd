class_name ZoomTiers
extends Resource

## Paliers de zoom de la carte de campagne (lot C6, `docs/design/2026-09-24-echelle-colonies.md`
## § 6). Les seuils portent sur la distance du rig de caméra (point visé → caméra, en unités
## monde = pixels de carte 4096). Chaque transition se fait en fondu sur une bande de largeur
## `*_fade` centrée sur le seuil.
##
## - Loin (`distance > far_threshold`) : provinces colorées + noms de provinces ;
## - Moyen : icônes de colonies, noms des cités, routes principales ;
## - Près (`distance < near_threshold`, vue « comté ») : maquettes, hameaux, toutes les routes,
##   noms de toutes les colonies, relief fin.
## Purement visuel.

enum Tier { NEAR, MEDIUM, FAR }

## Seuil Moyen → Loin.
@export var far_threshold: float = 620.0
@export var far_fade: float = 140.0
## Seuil Près → Moyen.
@export var near_threshold: float = 150.0
@export var near_fade: float = 40.0
## Distance en deçà de laquelle les noms des villes (`town`) s'ajoutent à ceux des cités
## (palier moyen rapproché) ; les autres colonies n'ont de nom qu'au palier près.
@export var town_label_distance: float = 320.0
## Relief fin (tuiles 8192²) : seulement sous cette distance caméra.
@export var fine_terrain_distance: float = 170.0
## Portée des maquettes et hameaux (distance caméra → objet, fondu `visibility_range`).
@export var model_range: float = 420.0
@export var hamlet_range: float = 260.0


## Palier dominant pour une distance caméra.
func tier_at(distance: float) -> Tier:
	if distance < near_threshold:
		return Tier.NEAR
	if distance < far_threshold:
		return Tier.MEDIUM
	return Tier.FAR


## Opacité [0, 1] du palier « près » (1 bien en deçà du seuil, 0 au-delà).
func near_weight(distance: float) -> float:
	return 1.0 - smoothstep(near_threshold - near_fade * 0.5, near_threshold + near_fade * 0.5, distance)


## Opacité [0, 1] du palier « loin ».
func far_weight(distance: float) -> float:
	return smoothstep(far_threshold - far_fade * 0.5, far_threshold + far_fade * 0.5, distance)


## Opacité [0, 1] du palier « moyen » (complément des deux autres).
func medium_weight(distance: float) -> float:
	return clampf(1.0 - near_weight(distance) - far_weight(distance), 0.0, 1.0)


## Réglages par défaut (`res://resources/zoom_tiers.tres`), ou une instance neuve si absente.
static func load_default() -> ZoomTiers:
	var path := "res://resources/zoom_tiers.tres"
	if ResourceLoader.exists(path):
		var loaded := load(path) as ZoomTiers
		if loaded != null:
			return loaded
	return ZoomTiers.new()
