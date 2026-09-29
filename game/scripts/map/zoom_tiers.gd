class_name ZoomTiers
extends Resource

## Paliers de zoom de la carte de campagne. Les seuils portent sur la distance du rig de caméra
## (point visé → caméra, en unités monde = pixels de carte 4096). Chaque transition se fait en
## fondu sur une bande de largeur `*_fade` centrée sur le seuil.
##
## Lot DV (ADR 0124, `docs/superpowers/specs/2026-09-29-dv-deux-vues-campagne-design.md`) : deux
## vues seulement, qui remplacent les paliers près / moyen / loin du lot C6 :
## - vue normale (`distance < strategic_threshold`) : relief 3D, maquettes jusqu'à `model_range`,
##   nom + écu au-dessus des villes, frontières, marqueurs d'armée 3D ;
## - vue stratégique : le parchemin CM2 seul (`StrategicView`). `strategic_weight` est la seule
##   source du fondu entre les deux.
## Sous-paliers de la vue normale :
## - « détail proche » (`distance < near_threshold`, ancien palier « près » du C6) : hameaux,
##   toutes les routes, relief fin, gens et effets de vie. `near_weight` le pèse.
## - Lot ZG4 (ADR 0036) : « vallée » (`distance < valley_threshold`, ~5 km de terrain à l'écran) :
##   frontières et brouillard de guerre estompés, étiquettes limitées aux colonies proches,
##   marqueurs d'armée à taille écran bornée ; « site » (`distance < site_threshold`, ~1 km,
##   jusqu'à ~200 m dans les zones de détail) : frontières presque effacées, seules les
##   étiquettes des colonies voisines. `valley_weight` / `site_weight` sont cumulatifs (1 à ce
##   palier et en deçà) ; `near_weight` reste 1 dans les deux.
## Purement visuel.

enum Tier { NEAR, VALLEY, SITE, STRATEGIC }

## Seuil vue normale → vue stratégique (parchemin) et largeur du fondu croisé.
@export var strategic_threshold: float = 1200.0
@export var strategic_fade: float = 200.0
## Seuil du détail proche (hameaux, toutes les routes, effets de vie).
@export var near_threshold: float = 150.0
@export var near_fade: float = 40.0
## Relief fin (tuiles 8192²) : seulement sous cette distance caméra.
@export var fine_terrain_distance: float = 170.0
## Portée des maquettes (distance caméra → objet, fondu `visibility_range`) : toute la vue normale.
@export var model_range: float = 1250.0
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
## Échelle des arbres et autres accessoires de carte : lot SZ4, `MapPropScale`
## (`res://resources/map_prop_scale.tres`).


## Palier dominant pour une distance caméra.
func tier_at(distance: float) -> Tier:
	if distance < site_threshold:
		return Tier.SITE
	if distance < valley_threshold:
		return Tier.VALLEY
	if distance < strategic_threshold:
		return Tier.NEAR
	return Tier.STRATEGIC


## Lot DV : poids [0, 1] de la vue stratégique (0 en vue normale, 1 sur le parchemin).
func strategic_weight(distance: float) -> float:
	return smoothstep(strategic_threshold - strategic_fade * 0.5, strategic_threshold + strategic_fade * 0.5, distance)


## Opacité [0, 1] du détail proche (1 bien en deçà du seuil, 0 au-delà).
func near_weight(distance: float) -> float:
	return 1.0 - smoothstep(near_threshold - near_fade * 0.5, near_threshold + near_fade * 0.5, distance)


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


## Réglages par défaut (`res://resources/zoom_tiers.tres`), ou une instance neuve si absente.
static func load_default() -> ZoomTiers:
	var path := "res://resources/zoom_tiers.tres"
	if ResourceLoader.exists(path):
		var loaded := load(path) as ZoomTiers
		if loaded != null:
			return loaded
	return ZoomTiers.new()
