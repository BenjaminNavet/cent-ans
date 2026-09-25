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
## - Lot ZG4 (ADR 0036), sous-paliers du « près » quand la pyramide de relief permet de descendre :
##   « vallée » (`distance < valley_threshold`, ~5 km de terrain à l'écran) : frontières et
##   brouillard de guerre estompés, étiquettes limitées aux colonies proches, marqueurs d'armée à
##   taille écran bornée ; « site » (`distance < site_threshold`, ~1 km, jusqu'à ~200 m dans les
##   zones de détail) : frontières presque effacées, seules les étiquettes des colonies voisines.
##   Les poids `valley_weight` / `site_weight` sont cumulatifs (1 à ce palier et en deçà) ;
##   `near_weight` reste 1 dans les deux.
## Purement visuel.

## VALLEY et SITE ajoutés à la fin (ZG4) : les valeurs existantes ne changent pas.
enum Tier { NEAR, MEDIUM, FAR, VALLEY, SITE }

## Seuil Moyen → Loin.
@export var far_threshold: float = 620.0
@export var far_fade: float = 140.0
## Seuil Près → Moyen.
@export var near_threshold: float = 150.0
@export var near_fade: float = 40.0
## Distance en deçà de laquelle les noms des villes (`town`) s'ajoutent à ceux des cités
## (palier moyen rapproché) ; les autres colonies n'ont de nom qu'au palier près.
@export var town_label_distance: float = 380.0
## Relief fin (tuiles 8192²) : seulement sous cette distance caméra.
@export var fine_terrain_distance: float = 170.0
## Portée des maquettes et hameaux (distance caméra → objet, fondu `visibility_range`).
@export var model_range: float = 420.0
@export var hamlet_range: float = 260.0
## Lot ZG4 : seuils Près → Vallée → Site (distance caméra) et largeurs de fondu.
@export var valley_threshold: float = 8.0
@export var valley_fade: float = 3.0
@export var site_threshold: float = 1.8
@export var site_fade: float = 0.8
## Opacité résiduelle des frontières et du voile du brouillard de guerre au palier site.
@export var site_border_alpha: float = 0.2
@export var site_fog_alpha: float = 0.35
## Portée des étiquettes de colonies aux paliers vallée / site, en multiples de la distance caméra
## (en vue rasante, l'horizon ne se couvre pas de noms).
@export var close_label_range_factor: float = 40.0
## Échelle des arbres (paramètre global `campaign_prop_scale`) sous `prop_scale_distance` :
## (distance / prop_scale_distance)^`prop_scale_exponent`, au moins `prop_scale_min`.
@export var prop_scale_distance: float = 22.0
@export var prop_scale_exponent: float = 0.8
@export var prop_scale_min: float = 0.04


## Palier dominant pour une distance caméra.
func tier_at(distance: float) -> Tier:
	if distance < site_threshold:
		return Tier.SITE
	if distance < valley_threshold:
		return Tier.VALLEY
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


## Lot ZG4 : poids [0, 1] du palier « vallée » et en deçà (1 sous le seuil, 0 au-dessus).
func valley_weight(distance: float) -> float:
	return 1.0 - smoothstep(valley_threshold - valley_fade * 0.5, valley_threshold + valley_fade * 0.5, distance)


## Lot ZG4 : poids [0, 1] du palier « site ».
func site_weight(distance: float) -> float:
	return 1.0 - smoothstep(site_threshold - site_fade * 0.5, site_threshold + site_fade * 0.5, distance)


## Lot ZG4 : opacité des frontières (1 jusqu'au palier comté, `site_border_alpha` au palier site).
func border_alpha(distance: float) -> float:
	return lerpf(1.0, lerpf(0.5, site_border_alpha, site_weight(distance)), valley_weight(distance))


## Lot ZG4 : opacité du voile du brouillard de guerre (même principe).
func fog_alpha(distance: float) -> float:
	return lerpf(1.0, lerpf(0.7, site_fog_alpha, site_weight(distance)), valley_weight(distance))


## Lot ZG4 : échelle des accessoires surdimensionnés (arbres) pour une distance caméra.
func prop_scale(distance: float) -> float:
	if distance >= prop_scale_distance:
		return 1.0
	return maxf(pow(maxf(distance, 1e-3) / prop_scale_distance, prop_scale_exponent), prop_scale_min)


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
